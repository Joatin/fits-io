use crate::bin_table::Value;
use crate::header::{Header, TableColumnFormat};
use std::error::Error;

/// Parses a TDIMn card's `(a,b,c)` shape.
///
/// A shape that does not parse is dropped rather than raised: TDIMn is a hint
/// about how to fold a column that reads perfectly well without it, so a
/// malformed one should not cost the caller the whole table.
fn parse_dimensions(value: &str) -> Vec<usize> {
    let Some(inner) = value
        .trim()
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
    else {
        return Vec::new();
    };

    inner
        .split(',')
        .map(|axis| axis.trim().parse::<usize>())
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default()
}

/// Everything the header says about one binary table column.
///
/// A column is more than its TFORMn type: TSCALn and TZEROn describe a linear
/// transform from the stored entry to the physical value it stands for, and
/// TNULLn names the stored entry that means "undefined". [`decode`] applies all
/// three, so callers see physical values rather than raw ones.
///
/// [`decode`]: FieldDefinition::decode
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDefinition {
    /// TFORMn: this column's type and repeat count.
    pub format: TableColumnFormat,
    /// Byte offset of this column from the start of a row.
    pub offset: usize,
    /// TTYPEn, or an empty string for a column that carries no name.
    pub name: String,
    /// TSCALn: the multiplier applied to a stored entry.
    pub scale: Option<f64>,
    /// TZEROn: the offset added after scaling.
    pub zero: Option<f64>,
    /// TNULLn: the stored entry that marks an undefined value.
    pub null: Option<i64>,
    /// TDIMn: the shape this column's entry has, fastest-varying axis first.
    ///
    /// Empty for a column with no TDIMn card, which is a plain run of
    /// [`TableColumnFormat::len`] elements. [`Value`] holds the elements flat
    /// either way; this says how to fold them.
    pub dimensions: Vec<usize>,
}

impl FieldDefinition {
    /// Reads every column of `header` in order, resolving each one's offset from
    /// the widths of the columns before it.
    pub fn all_from_header(header: &Header) -> Result<Vec<Self>, Box<dyn Error + Send + Sync>> {
        let table_fields = header
            .table_fields()
            .ok_or("Table header is missing its TFIELDS card")?;
        let table_fields = usize::try_from(table_fields)
            .map_err(|_| format!("TFIELDS must not be negative, but was {}", table_fields))?;

        let mut offset = 0;

        (0..table_fields)
            .map(|index| {
                // TFORMn is mandatory: without it the column width, and therefore
                // every following column offset, is unknown.
                let format = header.table_column_format(index).ok_or_else(|| {
                    format!("Table header is missing its TFORM{} card", index + 1)
                })?;

                // TTYPEn is optional. An unnamed column still occupies its bytes,
                // it just cannot be looked up by name.
                let name = header
                    .table_column_type(index)
                    .unwrap_or_default()
                    .to_string();

                let field = Self {
                    format,
                    offset,
                    name,
                    scale: header.table_scaling_factor(index),
                    zero: header.table_scaling_zero_point(index),
                    null: header
                        .table_null_value(index)
                        .and_then(|null| null.as_integer()),
                    dimensions: header
                        .table_dimensions(index)
                        .map(parse_dimensions)
                        .unwrap_or_default(),
                };
                offset += format.bytes_len();

                Ok::<_, Box<dyn Error + Send + Sync>>(field)
            })
            .collect()
    }

    /// Decodes this column out of `row`, which must be a whole row of the table.
    ///
    /// `heap` is the table's heap, which only a variable length array column
    /// reads from; pass an empty slice for a table that has none.
    pub fn decode(&self, row: &[u8], heap: &[u8]) -> crate::Result<Value> {
        let data = row.get(self.offset..).ok_or_else(|| {
            crate::Error::DeserializationError(format!(
                "Column {} starts at byte {} of a {} byte row",
                self.name,
                self.offset,
                row.len()
            ))
        })?;

        let value = self.format.parse_into_value(data, heap)?;

        if self.is_null(&value) {
            return Ok(Value::Null);
        }

        Ok(self.scaled(value))
    }

    /// Whether every element of `value` is the TNULLn entry.
    ///
    /// TNULLn applies only to the integer columns; floating point columns say
    /// "undefined" with a NaN, which needs no header to interpret. A column
    /// entry of repeat count `r` holds `r` values that are each null or not, and
    /// [`Value`] has no way to say "the third element is undefined", so only an
    /// entry that is null throughout is reported as such. In practice TNULLn is
    /// used with scalar columns, where the two are the same thing.
    fn is_null(&self, value: &Value) -> bool {
        let Some(null) = self.null else {
            return false;
        };

        fn all_equal<T: Copy + Into<i64>>(values: &[T], null: i64) -> bool {
            !values.is_empty() && values.iter().all(|value| (*value).into() == null)
        }

        match value {
            Value::U8(values) => all_equal(values, null),
            Value::I8(values) => all_equal(values, null),
            Value::U16(values) => all_equal(values, null),
            Value::I16(values) => all_equal(values, null),
            Value::U32(values) => all_equal(values, null),
            Value::I32(values) => all_equal(values, null),
            Value::I64(values) => all_equal(values, null),
            _ => false,
        }
    }

    /// Applies TSCALn and TZEROn to a decoded entry.
    fn scaled(&self, value: Value) -> Value {
        let scale = self.scale.unwrap_or(1.0);
        let zero = self.zero.unwrap_or(0.0);

        if scale == 1.0 && zero == 0.0 {
            return value;
        }

        // The FITS standard has no unsigned integer TFORMn codes. Unsigned
        // columns are instead written as the signed type of the same width with
        // a TZEROn of half its range, so that reading them back as a float would
        // both lose precision and lose the type. Recognise that pairing and
        // return the integers the file actually means.
        //
        // The conversion has to *add* the offset, which for a half-range TZEROn
        // is the same as flipping the top bit. Reinterpreting the stored bit
        // pattern instead leaves every value out by half the range: a `1I`
        // column with TZERO 32768 storing -32767 means 1, not 32769.
        if scale == 1.0 {
            match (&value, zero) {
                (Value::U8(values), -128.0) => {
                    return Value::I8(
                        values
                            .iter()
                            .map(|v| v.wrapping_sub(1 << 7) as i8)
                            .collect(),
                    );
                }
                (Value::I16(values), 32768.0) => {
                    return Value::U16(
                        values
                            .iter()
                            .map(|v| (*v as u16).wrapping_add(1 << 15))
                            .collect(),
                    );
                }
                (Value::I32(values), 2147483648.0) => {
                    return Value::U32(
                        values
                            .iter()
                            .map(|v| (*v as u32).wrapping_add(1 << 31))
                            .collect(),
                    );
                }
                (Value::I64(values), 9223372036854775808.0) => {
                    return Value::U64(
                        values
                            .iter()
                            .map(|v| (*v as u64).wrapping_add(1 << 63))
                            .collect(),
                    );
                }
                _ => {}
            }
        }

        let apply = |raw: f64| zero + scale * raw;

        match value {
            Value::U8(values) => Value::F64(values.into_iter().map(|v| apply(v as f64)).collect()),
            Value::I8(values) => Value::F64(values.into_iter().map(|v| apply(v as f64)).collect()),
            Value::U16(values) => Value::F64(values.into_iter().map(|v| apply(v as f64)).collect()),
            Value::I16(values) => Value::F64(values.into_iter().map(|v| apply(v as f64)).collect()),
            Value::U32(values) => Value::F64(values.into_iter().map(|v| apply(v as f64)).collect()),
            Value::I32(values) => Value::F64(values.into_iter().map(|v| apply(v as f64)).collect()),
            Value::I64(values) => Value::F64(values.into_iter().map(|v| apply(v as f64)).collect()),
            Value::U64(values) => Value::F64(values.into_iter().map(|v| apply(v as f64)).collect()),
            Value::F32(values) => Value::F64(values.into_iter().map(|v| apply(v as f64)).collect()),
            Value::F64(values) => Value::F64(values.into_iter().map(apply).collect()),

            // Both components of a complex value scale by the same factor.
            Value::C32(values) => Value::C32(
                values
                    .into_iter()
                    .map(|(re, im)| (apply(re as f64) as f32, apply(im as f64) as f32))
                    .collect(),
            ),
            Value::M64(values) => Value::M64(
                values
                    .into_iter()
                    .map(|(re, im)| (apply(re), apply(im)))
                    .collect(),
            ),

            // Scaling is meaningless for the non-numeric columns.
            value @ (Value::String(_)
            | Value::StringArray(_)
            | Value::Boolean(_)
            | Value::Bit(_)
            | Value::Null) => value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_dimensions;

    #[test]
    fn a_tdim_card_gives_the_shape_of_a_column_entry() {
        assert_eq!(parse_dimensions("(2,3)"), vec![2, 3]);
        assert_eq!(parse_dimensions("(4, 5, 6)"), vec![4, 5, 6]);
        assert_eq!(parse_dimensions(" (7) "), vec![7]);
    }

    #[test]
    fn a_malformed_tdim_card_is_dropped_rather_than_raised() {
        // The column reads fine without its shape, so a bad TDIMn must not cost
        // the caller the table.
        assert!(parse_dimensions("2,3").is_empty());
        assert!(parse_dimensions("(2,x)").is_empty());
        assert!(parse_dimensions("").is_empty());
    }
}
