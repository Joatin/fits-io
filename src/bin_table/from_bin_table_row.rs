use crate::ascii_table::AsciiRow;
use crate::bin_table::row_columns::RowColumns;
use crate::bin_table::{Row, Value};
use serde::Deserialize;
use serde::de::{
    DeserializeSeed, Error, IntoDeserializer, MapAccess, SeqAccess, Unexpected, Visitor,
};

struct Deserializer<'de, R: RowColumns> {
    row: &'de R,
    field_offset: usize,
}

/// Deserialises one row of a binary table into `T`.
pub fn from_bin_table_row<'a, T: Deserialize<'a>>(row: &'a Row) -> crate::Result<T> {
    from_row(row)
}

/// Deserialises one row of an ASCII table into `T`.
pub fn from_ascii_table_row<'a, T: Deserialize<'a>>(row: &'a AsciiRow) -> crate::Result<T> {
    from_row(row)
}

fn from_row<'a, T: Deserialize<'a>, R: RowColumns>(row: &'a R) -> crate::Result<T> {
    let mut deserializer = Deserializer {
        row,
        field_offset: 0,
    };
    T::deserialize(&mut deserializer)
}

impl<'de, R: RowColumns> Deserializer<'de, R> {
    fn name(&self) -> crate::Result<&'de str> {
        self.row.column_name(self.field_offset).ok_or_else(|| {
            crate::Error::custom(format!(
                "Column {} is past the end of a {} column table",
                self.field_offset,
                self.row.column_count()
            ))
        })
    }

    /// The decoded contents of the current column.
    fn value(&self) -> crate::Result<Value> {
        self.row.value_at(self.field_offset)?.ok_or_else(|| {
            crate::Error::custom(format!("Column {} has no value", self.field_offset))
        })
    }

    fn invalid_type(&self, expected: &str) -> crate::Error {
        let format = self
            .row
            .column_description(self.field_offset)
            .unwrap_or_else(|| "an unknown column".to_string());

        crate::Error::invalid_type(Unexpected::Other(&format), &expected)
    }
}

/// The first element of a numeric column, widened so every integer column type
/// fits without loss.
fn as_integer(value: &Value) -> Option<i128> {
    match value {
        Value::Boolean(values) => values.first().map(|value| i128::from(*value)),
        Value::Bit(values) | Value::U8(values) => values.first().map(|value| i128::from(*value)),
        Value::I8(values) => values.first().map(|value| i128::from(*value)),
        Value::U16(values) => values.first().map(|value| i128::from(*value)),
        Value::I16(values) => values.first().map(|value| i128::from(*value)),
        Value::U32(values) => values.first().map(|value| i128::from(*value)),
        Value::I32(values) => values.first().map(|value| i128::from(*value)),
        Value::I64(values) => values.first().map(|value| i128::from(*value)),
        Value::U64(values) => values.first().map(|value| i128::from(*value)),
        // A complex value is a pair, not a scalar; it deserializes as a
        // sequence of its components instead. An undefined entry has no number
        // to offer at all.
        Value::String(_)
        | Value::StringArray(_)
        | Value::F32(_)
        | Value::F64(_)
        | Value::C32(_)
        | Value::M64(_)
        | Value::Null => None,
    }
}

/// The first element of a column as a float. Integer columns widen into floats,
/// which is what a `f64` struct field on an integer column asks for.
fn as_float(value: &Value) -> Option<f64> {
    match value {
        Value::F32(values) => values.first().map(|value| *value as f64),
        Value::F64(values) => values.first().copied(),
        other => as_integer(other).map(|value| value as f64),
    }
}

/// Number of elements in a column, for the sequence deserializers.
fn len(value: &Value) -> usize {
    match value {
        Value::String(_) => 1,
        Value::StringArray(values) => values.len(),
        Value::Boolean(values) => values.len(),
        Value::Bit(values) | Value::U8(values) => values.len(),
        Value::I8(values) => values.len(),
        Value::U16(values) => values.len(),
        Value::I16(values) => values.len(),
        Value::U32(values) => values.len(),
        Value::I32(values) => values.len(),
        Value::I64(values) => values.len(),
        Value::U64(values) => values.len(),
        Value::F32(values) => values.len(),
        Value::F64(values) => values.len(),
        // Complex columns present as a flat run of components, so `1C` reads as
        // a two element sequence and `[f32; 2]` or `(f32, f32)` both work.
        Value::C32(values) => values.len() * 2,
        Value::M64(values) => values.len() * 2,
        // An undefined entry has no elements to walk.
        Value::Null => 0,
    }
}

/// Implements one integer `deserialize_*` method by widening to `i128` and
/// range-checking on the way back down.
macro_rules! deserialize_integer {
    ($method:ident, $visit:ident, $target:ty) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            let value = self.value()?;
            let number =
                as_integer(&value).ok_or_else(|| self.invalid_type(stringify!($target)))?;

            let number = <$target>::try_from(number).map_err(|_| {
                crate::Error::custom(format!(
                    "Column {} holds {}, which does not fit in a {}",
                    self.name().unwrap_or_default(),
                    number,
                    stringify!($target)
                ))
            })?;

            visitor.$visit(number)
        }
    };
}

impl<'de, R: RowColumns> serde::de::Deserializer<'de> for &mut Deserializer<'de, R> {
    type Error = crate::Error;

    /// Deserializes the column as whatever its TFORMn says it is.
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.value()?;

        if len(&value) != 1 {
            return self.deserialize_seq(visitor);
        }

        match &value {
            Value::String(text) => visitor.visit_str(text),
            Value::Boolean(_) => self.deserialize_bool(visitor),
            Value::F32(_) => self.deserialize_f32(visitor),
            Value::F64(_) => self.deserialize_f64(visitor),
            _ => self.deserialize_i64(visitor),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.value()?;

        match &value {
            Value::Boolean(values) => {
                let value = values.first().ok_or_else(|| {
                    crate::Error::invalid_length(0, &"a column with at least one element")
                })?;
                visitor.visit_bool(*value)
            }
            // A logical column is the natural source, but an integer column
            // reads as false for zero and true otherwise.
            other => match as_integer(other) {
                Some(number) => visitor.visit_bool(number != 0),
                None => Err(self.invalid_type("bool")),
            },
        }
    }

    deserialize_integer!(deserialize_i8, visit_i8, i8);
    deserialize_integer!(deserialize_i16, visit_i16, i16);
    deserialize_integer!(deserialize_i32, visit_i32, i32);
    deserialize_integer!(deserialize_i64, visit_i64, i64);
    deserialize_integer!(deserialize_u8, visit_u8, u8);
    deserialize_integer!(deserialize_u16, visit_u16, u16);
    deserialize_integer!(deserialize_u32, visit_u32, u32);
    deserialize_integer!(deserialize_u64, visit_u64, u64);

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.value()?;
        let number = as_float(&value).ok_or_else(|| self.invalid_type("f32"))?;

        visitor.visit_f32(number as f32)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.value()?;
        let number = as_float(&value).ok_or_else(|| self.invalid_type("f64"))?;

        visitor.visit_f64(number)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.value()?;
        let Value::String(text) = &value else {
            return Err(self.invalid_type("char"));
        };

        let mut characters = text.chars();
        match (characters.next(), characters.next()) {
            (Some(character), None) => visitor.visit_char(character),
            _ => Err(crate::Error::invalid_value(
                Unexpected::Str(text),
                &"a single character",
            )),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.value()?;

        match value {
            Value::String(text) => visitor.visit_string(text),
            _ => Err(self.invalid_type("string")),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.value()?;

        match value {
            Value::U8(bytes) | Value::Bit(bytes) => visitor.visit_byte_buf(bytes),
            Value::String(text) => visitor.visit_byte_buf(text.into_bytes()),
            _ => Err(self.invalid_type("bytes")),
        }
    }

    /// Every column of a binary table row is physically present, so "absent"
    /// here means the column declares a TNULLn value and this entry is it.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.value()?.is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    /// Deserializes a column whose TFORMn repeat count is greater than one.
    /// Deserializes the column as a sequence.
    ///
    /// A column with a TDIMn card holds a multidimensional array rather than a
    /// flat run of values, and is presented as nested sequences so that a
    /// `Vec<Vec<T>>` field sees the shape the card describes. TDIMn lists the
    /// fastest-varying axis first, so its *last* axis is the outermost one here.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let shape = self.row.column_dimensions(self.field_offset);

        if shape.len() > 1 {
            return visitor.visit_seq(ShapedAccess::new(self.value()?, shape.to_vec()));
        }

        visitor.visit_seq(ColumnAccess::new(self.value()?))
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(RowAccess::new(self))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    /// Deserializes a string column into a unit-only enum, matching the column
    /// text against the variant names.
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.value()?;

        match value {
            Value::String(text) => visitor.visit_enum(text.into_deserializer()),
            _ => Err(self.invalid_type("enum")),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.name()?)
    }

    /// Skips a column the target type does not want.
    ///
    /// The column is not decoded at all, so a struct can name a handful of
    /// columns out of a wide table without paying for the rest.
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

/// Walks the elements of one column, for columns whose TFORMn repeat count is
/// greater than one.
struct ColumnAccess {
    value: Value,
    index: usize,
}

impl ColumnAccess {
    fn new(value: Value) -> Self {
        Self { value, index: 0 }
    }

    /// One element of the column, as its own single-element `Value`.
    fn element(&self) -> Option<Value> {
        let index = self.index;

        let element = match &self.value {
            Value::String(text) => Value::String(text.clone()),
            Value::StringArray(values) => Value::String(values.get(index)?.clone()),
            Value::Boolean(values) => Value::Boolean(vec![*values.get(index)?]),
            Value::Bit(values) => Value::Bit(vec![*values.get(index)?]),
            Value::U8(values) => Value::U8(vec![*values.get(index)?]),
            Value::I8(values) => Value::I8(vec![*values.get(index)?]),
            Value::U16(values) => Value::U16(vec![*values.get(index)?]),
            Value::I16(values) => Value::I16(vec![*values.get(index)?]),
            Value::U32(values) => Value::U32(vec![*values.get(index)?]),
            Value::I32(values) => Value::I32(vec![*values.get(index)?]),
            Value::I64(values) => Value::I64(vec![*values.get(index)?]),
            Value::U64(values) => Value::U64(vec![*values.get(index)?]),
            Value::C32(values) => {
                let (real, imaginary) = values.get(index / 2)?;
                Value::F32(vec![if index.is_multiple_of(2) {
                    *real
                } else {
                    *imaginary
                }])
            }
            Value::M64(values) => {
                let (real, imaginary) = values.get(index / 2)?;
                Value::F64(vec![if index.is_multiple_of(2) {
                    *real
                } else {
                    *imaginary
                }])
            }
            Value::F32(values) => Value::F32(vec![*values.get(index)?]),
            Value::F64(values) => Value::F64(vec![*values.get(index)?]),
            Value::Null => return None,
        };

        Some(element)
    }
}

impl<'de> SeqAccess<'de> for ColumnAccess {
    type Error = crate::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some(element) = self.element() else {
            return Ok(None);
        };
        self.index += 1;

        seed.deserialize(ElementDeserializer::scalar(element))
            .map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(len(&self.value).saturating_sub(self.index))
    }
}

/// Deserializes a single element that has already been pulled out of a column.
struct ElementDeserializer {
    value: Value,
    /// The shape of this element, when it is itself a slice of a
    /// multidimensional column. Empty for a scalar.
    shape: Vec<usize>,
}

impl ElementDeserializer {
    fn scalar(value: Value) -> Self {
        Self {
            value,
            shape: Vec::new(),
        }
    }
}

/// Walks the outermost axis of a multidimensional column.
///
/// TDIMn gives the axes fastest-varying first, so the last axis is the one that
/// varies slowest and therefore the outer sequence here. Each step hands back
/// one slice of the flat values, itself shaped by the axes that remain.
struct ShapedAccess {
    value: Value,
    shape: Vec<usize>,
    index: usize,
}

impl ShapedAccess {
    fn new(value: Value, shape: Vec<usize>) -> Self {
        Self {
            value,
            shape,
            index: 0,
        }
    }

    /// How many slices the outermost axis is divided into.
    fn groups(&self) -> usize {
        self.shape.last().copied().unwrap_or(0)
    }

    /// How many values each of those slices holds.
    fn group_len(&self) -> usize {
        self.shape[..self.shape.len().saturating_sub(1)]
            .iter()
            .product()
    }
}

impl<'de> SeqAccess<'de> for ShapedAccess {
    type Error = crate::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.index >= self.groups() {
            return Ok(None);
        }

        let len = self.group_len();
        let group = slice(&self.value, self.index * len, len);
        self.index += 1;

        seed.deserialize(ElementDeserializer {
            value: group,
            shape: self.shape[..self.shape.len() - 1].to_vec(),
        })
        .map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.groups().saturating_sub(self.index))
    }
}

/// A run of `len` elements starting at `start`, as a value of the same kind.
fn slice(value: &Value, start: usize, len: usize) -> Value {
    fn take<T: Clone>(values: &[T], start: usize, len: usize) -> Vec<T> {
        values
            .get(start..)
            .map(|rest| rest[..len.min(rest.len())].to_vec())
            .unwrap_or_default()
    }

    match value {
        Value::StringArray(values) => Value::StringArray(take(values, start, len)),
        Value::Boolean(values) => Value::Boolean(take(values, start, len)),
        Value::Bit(values) => Value::Bit(take(values, start, len)),
        Value::U8(values) => Value::U8(take(values, start, len)),
        Value::I8(values) => Value::I8(take(values, start, len)),
        Value::U16(values) => Value::U16(take(values, start, len)),
        Value::I16(values) => Value::I16(take(values, start, len)),
        Value::U32(values) => Value::U32(take(values, start, len)),
        Value::I32(values) => Value::I32(take(values, start, len)),
        Value::I64(values) => Value::I64(take(values, start, len)),
        Value::U64(values) => Value::U64(take(values, start, len)),
        Value::F32(values) => Value::F32(take(values, start, len)),
        Value::F64(values) => Value::F64(take(values, start, len)),
        Value::C32(values) => Value::C32(take(values, start, len)),
        Value::M64(values) => Value::M64(take(values, start, len)),
        // A string and an undefined entry have no elements to slice.
        Value::String(_) | Value::Null => value.clone(),
    }
}

impl ElementDeserializer {
    fn invalid_type(&self, expected: &str) -> crate::Error {
        crate::Error::invalid_type(Unexpected::Other("a binary table element"), &expected)
    }
}

macro_rules! deserialize_element_integer {
    ($method:ident, $visit:ident, $target:ty) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            let number =
                as_integer(&self.value).ok_or_else(|| self.invalid_type(stringify!($target)))?;

            let number = <$target>::try_from(number).map_err(|_| {
                crate::Error::custom(format!(
                    "{} does not fit in a {}",
                    number,
                    stringify!($target)
                ))
            })?;

            visitor.$visit(number)
        }
    };
}

impl<'de> serde::de::Deserializer<'de> for ElementDeserializer {
    type Error = crate::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::String(text) => visitor.visit_str(text),
            Value::Boolean(_) => self.deserialize_bool(visitor),
            Value::F32(_) => self.deserialize_f32(visitor),
            Value::F64(_) => self.deserialize_f64(visitor),
            _ => self.deserialize_i64(visitor),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let number = as_integer(&self.value).ok_or_else(|| self.invalid_type("bool"))?;

        visitor.visit_bool(number != 0)
    }

    deserialize_element_integer!(deserialize_i8, visit_i8, i8);
    deserialize_element_integer!(deserialize_i16, visit_i16, i16);
    deserialize_element_integer!(deserialize_i32, visit_i32, i32);
    deserialize_element_integer!(deserialize_i64, visit_i64, i64);
    deserialize_element_integer!(deserialize_u8, visit_u8, u8);
    deserialize_element_integer!(deserialize_u16, visit_u16, u16);
    deserialize_element_integer!(deserialize_u32, visit_u32, u32);
    deserialize_element_integer!(deserialize_u64, visit_u64, u64);

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let number = as_float(&self.value).ok_or_else(|| self.invalid_type("f32"))?;

        visitor.visit_f32(number as f32)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let number = as_float(&self.value).ok_or_else(|| self.invalid_type("f64"))?;

        visitor.visit_f64(number)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let Value::String(text) = &self.value else {
            return Err(self.invalid_type("char"));
        };

        let mut characters = text.chars();
        match (characters.next(), characters.next()) {
            (Some(character), None) => visitor.visit_char(character),
            _ => Err(crate::Error::invalid_value(
                Unexpected::Str(text),
                &"a single character",
            )),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(text) => visitor.visit_string(text),
            _ => Err(self.invalid_type("string")),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::U8(bytes) | Value::Bit(bytes) => visitor.visit_byte_buf(bytes),
            Value::String(text) => visitor.visit_byte_buf(text.into_bytes()),
            _ => Err(self.invalid_type("bytes")),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.shape.len() > 1 {
            return visitor.visit_seq(ShapedAccess::new(self.value, self.shape));
        }

        visitor.visit_seq(ColumnAccess::new(self.value))
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(self.invalid_type("map"))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(self.invalid_type("struct"))
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(text) => visitor.visit_enum(text.into_deserializer()),
            _ => Err(self.invalid_type("enum")),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

struct RowAccess<'a, 'de: 'a, R: RowColumns> {
    de: &'a mut Deserializer<'de, R>,
}

impl<'a, 'de, R: RowColumns> RowAccess<'a, 'de, R> {
    pub fn new(de: &'a mut Deserializer<'de, R>) -> Self {
        Self { de }
    }
}

impl<'de, 'a, R: RowColumns> MapAccess<'de> for RowAccess<'a, 'de, R> {
    type Error = crate::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.de.field_offset >= self.de.row.column_count() {
            return Ok(None);
        }
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let result = seed.deserialize(&mut *self.de);
        self.de.field_offset += 1;
        result
    }

    fn size_hint(&self) -> Option<usize> {
        Some(
            self.de
                .row
                .column_count()
                .saturating_sub(self.de.field_offset),
        )
    }
}
