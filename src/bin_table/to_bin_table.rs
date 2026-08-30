use crate::bin_table::encode::encode;
use crate::bin_table::{BinTable, FieldDefinition, Value};
use crate::header::{ArrayDescriptor, TableColumnFormat, TableElementFormat};
use serde::ser::Impossible;
use serde::{Serialize, ser};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum Error {
    NotSupported(&'static str),
    /// The input is not shaped like a table: a table is a sequence of rows, and
    /// a row is a struct whose fields are its columns.
    NotATable,
    /// Rows disagree about what columns the table has.
    InconsistentColumns {
        expected: String,
        found: String,
    },
    Custom(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotSupported(kind) => {
                write!(f, "Binary tables do not support {} values", kind)
            }
            Error::NotATable => write!(
                f,
                "A binary table is a sequence of structs, one struct per row"
            ),
            Error::InconsistentColumns { expected, found } => write!(
                f,
                "Every row must have the same columns, but one row has [{}] and another [{}]",
                expected, found
            ),
            Error::Custom(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for Error {}
impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::Custom(msg.to_string())
    }
}

type Result<T> = std::result::Result<T, Error>;

/// One field of a row: its value, and the shape that value has.
///
/// A field that arrives as `Vec<Vec<T>>` holds one flat run of values with a
/// shape of two axes. FITS stores it exactly that way — flat, with a TDIMn card
/// giving the shape — so the shape has to be carried alongside the values here
/// rather than thrown away and guessed at later.
#[derive(Debug, Clone)]
pub(crate) struct Column {
    pub(crate) value: Value,
    /// The axes, fastest-varying first, as TDIMn writes them. Empty for a
    /// scalar, one axis for a plain array.
    pub(crate) shape: Vec<usize>,
}

impl Column {
    fn scalar(value: Value) -> Self {
        Self {
            value,
            shape: Vec::new(),
        }
    }
}

/// One row, as the columns it is made of.
pub(crate) type Row = Vec<(String, Column)>;

/// Collects the rows out of anything shaped like a table.
///
/// `data` is a sequence of structs — a `Vec<Row>` or a slice of them — where
/// each struct's fields are the table's columns. A single struct is taken as a
/// one-row table. Both table writers start here; they differ only in how they
/// turn the collected values into columns.
pub(crate) fn collect_rows<T: Serialize>(data: &T) -> Result<Vec<Row>> {
    let mut table = TableSerializer {
        rows: Vec::new(),
        current: Vec::new(),
    };
    data.serialize(&mut table)?;

    Ok(table.rows)
}

/// Checks that every row has the same columns, and returns their names.
///
/// Rows that disagree cannot make one table: a column would either be dropped or
/// silently filled from the wrong field.
pub(crate) fn column_names(rows: &[Row]) -> Result<Vec<&str>> {
    let Some(first) = rows.first() else {
        return Ok(Vec::new());
    };

    let names: Vec<&str> = first.iter().map(|(name, _)| name.as_str()).collect();

    for row in rows {
        let found: Vec<&str> = row.iter().map(|(name, _)| name.as_str()).collect();
        if found != names {
            return Err(Error::InconsistentColumns {
                expected: names.join(", "),
                found: found.join(", "),
            });
        }
    }

    Ok(names)
}

/// Serialises rows into a binary table.
///
/// `data` is a sequence of structs — a `Vec<Row>` or a slice of them — where
/// each struct's fields are the table's columns. A single struct is taken as a
/// one-row table.
///
/// Each column's TFORMn is worked out from the values in it: the narrowest
/// integer type that holds every value, `D` for a column with any floating point
/// value in it, and a character field as wide as the longest string. A column
/// whose entries are different lengths is padded out to the longest.
pub fn to_bin_table<T: Serialize>(data: &T) -> Result<BinTable> {
    build(collect_rows(data)?)
}

/// Turns collected rows into an encoded table.
fn build(rows: Vec<Row>) -> Result<BinTable> {
    let names = column_names(&rows)?;
    if names.is_empty() {
        return Ok(BinTable::from_parts(Vec::new(), Vec::new(), 0, 0, 0));
    }

    let mut field_definitions = Vec::with_capacity(names.len());
    let mut offset = 0;

    for (index, name) in names.iter().enumerate() {
        let format = column_format(rows.iter().map(|row| &row[index].1.value));

        // A column whose rows hold different numbers of elements does not fit a
        // fixed-width field. Padding the short rows out would invent values that
        // read back as real ones, so such a column becomes a variable length
        // array, which records each row's length exactly.
        let format = match ragged(rows.iter().map(|row| &row[index].1.value))
            .then(|| element_format(format))
            .flatten()
        {
            Some(element) => TableColumnFormat::VariableLengthArray {
                element,
                descriptor: ArrayDescriptor::P32,
                max: format.len(),
            },
            None => format,
        };

        // FITS has no unsigned integer TFORMn code. A column that has to hold a
        // `u64` larger than `i64::MAX` is written as the signed type with a
        // TZEROn of half its range, which is the standard's way of saying
        // unsigned -- and what this crate's reader already understands.
        let unsigned = matches!(format, TableColumnFormat::I64(_))
            && rows
                .iter()
                .any(|row| matches!(row[index].1.value, Value::U64(_)));

        field_definitions.push(FieldDefinition {
            format,
            offset,
            name: (*name).to_string(),
            scale: unsigned.then_some(1.0),
            zero: unsigned.then_some(UNSIGNED_64_ZERO),
            // An integer column with a missing entry needs a value that stands
            // for "undefined"; the standard's way to say that is TNULLn.
            null: has_null(rows.iter().map(|row| &row[index].1.value))
                .then_some(null_sentinel(format))
                .flatten(),
            // A shape of one axis is what a plain `rT` column already says, so
            // only a genuinely multidimensional column needs a TDIMn card.
            dimensions: shape_of(rows.iter().map(|row| &row[index].1)),
        });

        offset += format.bytes_len();
    }

    let bytes_per_row = offset;
    let mut data = Vec::with_capacity(bytes_per_row * rows.len());

    // Variable length array columns keep their values in the heap that follows
    // the rows, and put only a descriptor in the row itself.
    let mut heap = Vec::new();

    for row in &rows {
        for (field, (_, value)) in field_definitions.iter().zip(row) {
            let value = &value.value;
            let value = match (value, field.null) {
                (Value::Null, Some(null)) => sentinel_value(field.format, null),
                (Value::Null, None) => Value::Null,
                (value, _) => unsigned_to_stored(value.clone(), field.zero),
            };

            match field.format {
                TableColumnFormat::VariableLengthArray {
                    element,
                    descriptor,
                    ..
                } => encode_array(&value, element, descriptor, &mut data, &mut heap),
                format => encode(&value, format, &mut data),
            }
        }
    }

    let heap_offset = data.len();
    data.extend_from_slice(&heap);

    Ok(BinTable::from_parts(
        field_definitions,
        data,
        bytes_per_row,
        rows.len(),
        heap_offset,
    ))
}

/// Writes a variable length array: its values into the heap, and a descriptor
/// saying how many there are and where they start into the row.
fn encode_array(
    value: &Value,
    element: TableElementFormat,
    descriptor: ArrayDescriptor,
    row: &mut Vec<u8>,
    heap: &mut Vec<u8>,
) {
    let count = element_count(value);
    let offset = heap.len();

    if count > 0 {
        encode(value, element.repeated(count), heap);
    }

    match descriptor {
        ArrayDescriptor::P32 => {
            row.extend_from_slice(&(count as i32).to_be_bytes());
            row.extend_from_slice(&(offset as i32).to_be_bytes());
        }
        ArrayDescriptor::Q64 => {
            row.extend_from_slice(&(count as i64).to_be_bytes());
            row.extend_from_slice(&(offset as i64).to_be_bytes());
        }
    }
}

/// The element type a variable length array column holds.
fn element_format(format: TableColumnFormat) -> Option<TableElementFormat> {
    Some(match format {
        TableColumnFormat::Boolean(_) => TableElementFormat::Boolean,
        TableColumnFormat::Bit(_) => TableElementFormat::Bit,
        TableColumnFormat::U8(_) => TableElementFormat::U8,
        TableColumnFormat::I8(_) => TableElementFormat::I8,
        TableColumnFormat::U16(_) => TableElementFormat::U16,
        TableColumnFormat::I16(_) => TableElementFormat::I16,
        TableColumnFormat::U32(_) => TableElementFormat::U32,
        TableColumnFormat::I32(_) => TableElementFormat::I32,
        TableColumnFormat::I64(_) => TableElementFormat::I64,
        TableColumnFormat::F32(_) => TableElementFormat::F32,
        TableColumnFormat::F64(_) => TableElementFormat::F64,
        TableColumnFormat::C32(_) => TableElementFormat::C32,
        TableColumnFormat::M64(_) => TableElementFormat::M64,
        TableColumnFormat::String(_) | TableColumnFormat::StringArray(..) => {
            TableElementFormat::Character
        }
        TableColumnFormat::VariableLengthArray { .. } => return None,
    })
}

fn has_null<'a>(values: impl Iterator<Item = &'a Value>) -> bool {
    values.into_iter().any(|value| value.is_null())
}

/// The shape a column's TDIMn card should record, if any.
///
/// Rows that disagree about the shape have none to record, and neither has a
/// column that is a plain run of values.
fn shape_of<'a>(mut columns: impl Iterator<Item = &'a Column>) -> Vec<usize> {
    let Some(first) = columns.next() else {
        return Vec::new();
    };

    if first.shape.len() < 2 || columns.any(|column| column.shape != first.shape) {
        return Vec::new();
    }

    first.shape.clone()
}

/// The TZEROn that marks a 64-bit column as holding unsigned values.
const UNSIGNED_64_ZERO: f64 = 9223372036854775808.0;

/// Shifts an unsigned value down into the signed range the column stores.
///
/// The column records the shift in its TZEROn card, so reading it back adds the
/// offset again and recovers the original number. Casting straight to `i64`
/// instead would turn `u64::MAX` into `-1`.
fn unsigned_to_stored(value: Value, zero: Option<f64>) -> Value {
    if zero != Some(UNSIGNED_64_ZERO) {
        return value;
    }

    match value {
        Value::U64(values) => Value::I64(
            values
                .into_iter()
                .map(|value| value.wrapping_sub(1 << 63) as i64)
                .collect(),
        ),
        other => other,
    }
}

/// The stored value that will stand for "undefined" in a column.
///
/// Only the integer columns need one: a floating point column says undefined
/// with a NaN, and a character column with blanks.
fn null_sentinel(format: TableColumnFormat) -> Option<i64> {
    match format {
        TableColumnFormat::U8(_) => Some(u8::MAX as i64),
        TableColumnFormat::I16(_) => Some(i16::MIN as i64),
        TableColumnFormat::I32(_) => Some(i32::MIN as i64),
        TableColumnFormat::I64(_) => Some(i64::MIN),
        _ => None,
    }
}

/// A single-element value holding `null`, for writing into an undefined entry.
fn sentinel_value(format: TableColumnFormat, null: i64) -> Value {
    match format {
        TableColumnFormat::U8(_) => Value::U8(vec![null as u8]),
        TableColumnFormat::I16(_) => Value::I16(vec![null as i16]),
        TableColumnFormat::I32(_) => Value::I32(vec![null as i32]),
        _ => Value::I64(vec![null]),
    }
}

/// Whether a column's rows hold different numbers of elements.
///
/// A text column is not ragged in this sense: strings of different lengths sit
/// perfectly well in one field, padded with the blanks the standard specifies.
fn ragged<'a>(values: impl Iterator<Item = &'a Value> + Clone) -> bool {
    if values
        .clone()
        .any(|value| matches!(value, Value::String(_) | Value::StringArray(_)))
    {
        return false;
    }

    let mut counts = values.filter(|value| !value.is_null()).map(element_count);

    let Some(first) = counts.next() else {
        return false;
    };

    counts.any(|count| count != first)
}

/// Picks a TFORMn that every value in a column fits into.
fn column_format<'a>(values: impl Iterator<Item = &'a Value> + Clone) -> TableColumnFormat {
    let repeat = values.clone().map(element_count).max().unwrap_or(0).max(1);

    // Character columns are as wide as their longest string; the count is bytes,
    // not elements.
    if values
        .clone()
        .any(|value| matches!(value, Value::String(_) | Value::StringArray(_)))
    {
        let width = values
            .clone()
            .map(|value| match value {
                Value::String(text) => text.len(),
                Value::StringArray(values) => values.iter().map(String::len).sum(),
                _ => 0,
            })
            .max()
            .unwrap_or(0)
            .max(1);

        return TableColumnFormat::String(width);
    }

    if values.clone().any(|value| matches!(value, Value::M64(_))) {
        return TableColumnFormat::M64(repeat);
    }
    if values.clone().any(|value| matches!(value, Value::C32(_))) {
        return TableColumnFormat::C32(repeat);
    }

    // A column with any double in it is a double column; one that is entirely
    // single precision stays single.
    if values.clone().any(|value| matches!(value, Value::F64(_))) {
        return TableColumnFormat::F64(repeat);
    }
    if values.clone().any(|value| matches!(value, Value::F32(_))) {
        return TableColumnFormat::F32(repeat);
    }

    if values
        .clone()
        .all(|value| matches!(value, Value::Boolean(_) | Value::Null))
    {
        return TableColumnFormat::Boolean(repeat);
    }

    // Integer columns keep the width the caller's own type asked for, widened
    // to the narrowest standard code that holds it. Choosing by the values
    // instead would let a column of `i32` come out as `B` because this
    // particular batch of rows happened to be small.
    let width = values
        .map(integer_width)
        .max()
        .unwrap_or(IntegerWidth::Byte);

    match width {
        IntegerWidth::Byte => TableColumnFormat::U8(repeat),
        IntegerWidth::Short => TableColumnFormat::I16(repeat),
        IntegerWidth::Int => TableColumnFormat::I32(repeat),
        IntegerWidth::Long => TableColumnFormat::I64(repeat),
    }
}

/// The four integer widths a binary table can store: TFORMn `B`, `I`, `J`, `K`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum IntegerWidth {
    Byte,
    Short,
    Int,
    Long,
}

/// The narrowest column that can hold this value without losing anything.
///
/// The unsigned types step up a width rather than reusing the signed one of the
/// same size: a `u16` of 40000 does not fit in an `I`.
fn integer_width(value: &Value) -> IntegerWidth {
    match value {
        Value::Bit(_) | Value::U8(_) | Value::Null => IntegerWidth::Byte,
        Value::I8(_) | Value::I16(_) => IntegerWidth::Short,
        Value::U16(_) | Value::I32(_) => IntegerWidth::Int,
        _ => IntegerWidth::Long,
    }
}

fn element_count(value: &Value) -> usize {
    match value {
        Value::Null => 0,
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
        Value::C32(values) => values.len(),
        Value::M64(values) => values.len(),
    }
}

/// Collects the table.
///
/// The same serializer handles both shapes the input may take: a sequence hands
/// each of its elements back to it as a struct, and a bare struct arrives as one
/// directly. Either way a completed struct becomes a row.
struct TableSerializer {
    rows: Vec<Row>,
    /// The row currently being collected, moved into `rows` when its struct ends.
    current: Row,
}

impl ser::Serializer for &mut TableSerializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Self;
    type SerializeStructVariant = Impossible<(), Error>;

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(self)
    }

    /// Each struct is one row: the table's own if it came in bare, or one
    /// element of the sequence that holds them.
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.current = Vec::with_capacity(len);
        Ok(self)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_bool(self, _v: bool) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_i8(self, _v: i8) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_i16(self, _v: i16) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_i32(self, _v: i32) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_i64(self, _v: i64) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_u8(self, _v: u8) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_u16(self, _v: u16) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_u32(self, _v: u32) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_u64(self, _v: u64) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_f32(self, _v: f32) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_f64(self, _v: f64) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_char(self, _v: char) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_str(self, _v: &str) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_none(self) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        Err(Error::NotATable)
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::NotSupported("enum"))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::NotATable)
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::NotSupported("enum"))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::NotSupported("map"))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::NotSupported("enum"))
    }
}

impl ser::SerializeSeq for &mut TableSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // Each element is a struct, which lands back in `serialize_struct` and
        // becomes a row when it ends.
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeStruct for &mut TableSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.current
            .push((key.to_string(), value.serialize(ValueSerializer)?));
        Ok(())
    }

    fn end(self) -> Result<()> {
        let row = std::mem::take(&mut self.current);
        self.rows.push(row);
        Ok(())
    }
}

impl ser::SerializeTuple for &mut TableSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

/// Turns one field of a row into the [`Value`] its column will hold.
struct ValueSerializer;

impl ser::Serializer for ValueSerializer {
    type Ok = Column;
    type Error = Error;

    type SerializeSeq = SeqSerializer;
    type SerializeTuple = SeqSerializer;
    type SerializeTupleStruct = SeqSerializer;
    type SerializeTupleVariant = Impossible<Column, Error>;
    type SerializeMap = Impossible<Column, Error>;
    type SerializeStruct = Impossible<Column, Error>;
    type SerializeStructVariant = Impossible<Column, Error>;

    fn serialize_bool(self, v: bool) -> Result<Column> {
        Ok(Column::scalar(Value::Boolean(vec![v])))
    }
    fn serialize_i8(self, v: i8) -> Result<Column> {
        Ok(Column::scalar(Value::I8(vec![v])))
    }
    fn serialize_i16(self, v: i16) -> Result<Column> {
        Ok(Column::scalar(Value::I16(vec![v])))
    }
    fn serialize_i32(self, v: i32) -> Result<Column> {
        Ok(Column::scalar(Value::I32(vec![v])))
    }
    fn serialize_i64(self, v: i64) -> Result<Column> {
        Ok(Column::scalar(Value::I64(vec![v])))
    }
    fn serialize_u8(self, v: u8) -> Result<Column> {
        Ok(Column::scalar(Value::U8(vec![v])))
    }
    fn serialize_u16(self, v: u16) -> Result<Column> {
        Ok(Column::scalar(Value::U16(vec![v])))
    }
    fn serialize_u32(self, v: u32) -> Result<Column> {
        Ok(Column::scalar(Value::U32(vec![v])))
    }
    fn serialize_u64(self, v: u64) -> Result<Column> {
        Ok(Column::scalar(Value::U64(vec![v])))
    }
    fn serialize_f32(self, v: f32) -> Result<Column> {
        Ok(Column::scalar(Value::F32(vec![v])))
    }
    fn serialize_f64(self, v: f64) -> Result<Column> {
        Ok(Column::scalar(Value::F64(vec![v])))
    }
    fn serialize_char(self, v: char) -> Result<Column> {
        Ok(Column::scalar(Value::String(v.to_string())))
    }
    fn serialize_str(self, v: &str) -> Result<Column> {
        Ok(Column::scalar(Value::String(v.to_string())))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Column> {
        Ok(Column::scalar(Value::U8(v.to_vec())))
    }

    /// A `None` field is an undefined entry, which the column will mark with its
    /// TNULLn value.
    fn serialize_none(self) -> Result<Column> {
        Ok(Column::scalar(Value::Null))
    }
    fn serialize_some<T>(self, value: &T) -> Result<Column>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<Column> {
        Ok(Column::scalar(Value::Null))
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Column> {
        Ok(Column::scalar(Value::Null))
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<Column> {
        Ok(Column::scalar(Value::String(variant.to_string())))
    }
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Column>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Column>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::NotSupported("enum"))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SeqSerializer {
            elements: Vec::with_capacity(len.unwrap_or_default()),
        })
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::NotSupported("enum"))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::NotSupported("map"))
    }
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(Error::NotSupported("nested struct"))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::NotSupported("enum"))
    }
}

/// Gathers an array-valued field into a single value and its shape.
struct SeqSerializer {
    elements: Vec<Column>,
}

impl SeqSerializer {
    /// Merges the elements into the one value the column holds, and works out
    /// the shape they were arranged in.
    ///
    /// Every element has to be the same kind of thing: a column has one type,
    /// and `[1, "two"]` is not something a binary table can hold. Nested
    /// sequences have to agree on their shape too, since a ragged
    /// `Vec<Vec<T>>` has no rectangular shape for TDIMn to describe.
    fn merge(self) -> Result<Column> {
        let count = self.elements.len();

        // The elements' own shape becomes the inner axes, with this sequence's
        // length as the outermost. TDIMn wants the fastest-varying axis first,
        // which is the innermost, so the new axis goes on the end.
        let inner = self.elements.first().map(|first| first.shape.clone());
        let shape = match inner {
            Some(inner) if self.elements.iter().all(|element| element.shape == inner) => {
                let mut shape = inner;
                shape.push(count);
                shape
            }
            // Elements of different shapes cannot make a rectangular array; the
            // values still go in, flat and without a TDIMn card.
            _ => vec![count],
        };

        let mut elements = self.elements.into_iter().map(|element| element.value);

        let Some(first) = elements.next() else {
            return Ok(Column {
                value: Value::Null,
                shape,
            });
        };

        let mut merged = first;
        for element in elements {
            merged = match (merged, element) {
                (Value::Boolean(mut a), Value::Boolean(b)) => {
                    a.extend(b);
                    Value::Boolean(a)
                }
                (Value::U8(mut a), Value::U8(b)) => {
                    a.extend(b);
                    Value::U8(a)
                }
                (Value::I8(mut a), Value::I8(b)) => {
                    a.extend(b);
                    Value::I8(a)
                }
                (Value::U16(mut a), Value::U16(b)) => {
                    a.extend(b);
                    Value::U16(a)
                }
                (Value::I16(mut a), Value::I16(b)) => {
                    a.extend(b);
                    Value::I16(a)
                }
                (Value::U32(mut a), Value::U32(b)) => {
                    a.extend(b);
                    Value::U32(a)
                }
                (Value::I32(mut a), Value::I32(b)) => {
                    a.extend(b);
                    Value::I32(a)
                }
                (Value::I64(mut a), Value::I64(b)) => {
                    a.extend(b);
                    Value::I64(a)
                }
                (Value::U64(mut a), Value::U64(b)) => {
                    a.extend(b);
                    Value::U64(a)
                }
                (Value::F32(mut a), Value::F32(b)) => {
                    a.extend(b);
                    Value::F32(a)
                }
                (Value::F64(mut a), Value::F64(b)) => {
                    a.extend(b);
                    Value::F64(a)
                }
                (Value::String(a), Value::String(b)) => Value::StringArray(vec![a, b]),
                (Value::StringArray(mut a), Value::String(b)) => {
                    a.push(b);
                    Value::StringArray(a)
                }
                _ => return Err(Error::NotSupported("mixed-type array")),
            };
        }

        Ok(Column {
            value: merged,
            shape,
        })
    }
}

impl ser::SerializeSeq for SeqSerializer {
    type Ok = Column;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.elements.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Column> {
        self.merge()
    }
}

impl ser::SerializeTuple for SeqSerializer {
    type Ok = Column;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Column> {
        self.merge()
    }
}

impl ser::SerializeTupleStruct for SeqSerializer {
    type Ok = Column;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Column> {
        self.merge()
    }
}
