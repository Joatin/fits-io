use crate::ascii_table::AsciiColumnFormat;
use crate::bin_table::Value;
use crate::header::Header;
use std::error::Error;

/// Everything the header says about one ASCII table column.
///
/// Unlike a binary table, an ASCII table gives each column's position
/// explicitly with a TBCOLn card rather than leaving it to be summed from the
/// widths before it. Columns may therefore be separated by filler, and in a
/// malformed file may even overlap.
#[derive(Debug, Clone, PartialEq)]
pub struct AsciiFieldDefinition {
    /// TFORMn: this column's type and field width.
    pub format: AsciiColumnFormat,
    /// Byte offset of this column's field from the start of a row.
    ///
    /// This is TBCOLn minus one: the card counts from 1, the offset from 0.
    pub offset: usize,
    /// TTYPEn, or an empty string for a column that carries no name.
    pub name: String,
    /// TNULLn: the exact characters that mark an undefined entry.
    pub null: Option<String>,
    /// TSCALn: the multiplier applied to a stored entry.
    pub scale: Option<f64>,
    /// TZEROn: the offset added after scaling.
    pub zero: Option<f64>,
}

impl AsciiFieldDefinition {
    /// Reads every column of `header` in order.
    pub fn all_from_header(header: &Header) -> Result<Vec<Self>, Box<dyn Error + Send + Sync>> {
        let table_fields = header
            .table_fields()
            .ok_or("Table header is missing its TFIELDS card")?;
        let table_fields = usize::try_from(table_fields)
            .map_err(|_| format!("TFIELDS must not be negative, but was {}", table_fields))?;

        (0..table_fields)
            .map(|index| {
                let format = header.ascii_column_format(index).ok_or_else(|| {
                    match header.table_format(index) {
                        Some(format) => format!(
                            "Table header has an invalid ASCII TFORM{} card: {:?}",
                            index + 1,
                            format
                        ),
                        None => format!("Table header is missing its TFORM{} card", index + 1),
                    }
                })?;

                // TBCOLn is mandatory for an ASCII table: nothing else says
                // where the column starts.
                let column = header.table_column(index).ok_or_else(|| {
                    format!("Table header is missing its TBCOL{} card", index + 1)
                })?;
                let offset = usize::try_from(column)
                    .map_err(|_| format!("TBCOL{} must not be negative", index + 1))?
                    .saturating_sub(1);

                Ok::<_, Box<dyn Error + Send + Sync>>(Self {
                    format,
                    offset,
                    name: header
                        .table_column_type(index)
                        .unwrap_or_default()
                        .to_string(),
                    // TNULLn is a character string for an ASCII table, matched
                    // against the field as written.
                    null: header
                        .table_null_value(index)
                        .and_then(|null| null.as_str())
                        .map(|null| null.trim().to_string()),
                    scale: header.table_scaling_factor(index),
                    zero: header.table_scaling_zero_point(index),
                })
            })
            .collect()
    }

    /// Decodes this column out of `row`, which must be a whole row of the table.
    pub fn decode(&self, row: &[u8]) -> crate::Result<Value> {
        let width = self.format.bytes_len();

        let field = row
            .get(self.offset..)
            .and_then(|row| row.get(..width))
            .ok_or_else(|| {
                crate::Error::DeserializationError(format!(
                    "Column {} occupies bytes {}..{} of a {} byte row",
                    self.name,
                    self.offset,
                    self.offset + width,
                    row.len()
                ))
            })?;

        if let Some(null) = &self.null
            && String::from_utf8_lossy(field).trim() == null
        {
            return Ok(Value::Null);
        }

        Ok(self.scaled(self.format.parse_into_value(field)?))
    }

    /// Applies TSCALn and TZEROn, which an ASCII table may carry just as a
    /// binary one may.
    fn scaled(&self, value: Value) -> Value {
        let scale = self.scale.unwrap_or(1.0);
        let zero = self.zero.unwrap_or(0.0);

        if scale == 1.0 && zero == 0.0 {
            return value;
        }

        match value {
            Value::I64(values) => Value::F64(
                values
                    .into_iter()
                    .map(|raw| zero + scale * raw as f64)
                    .collect(),
            ),
            Value::F64(values) => {
                Value::F64(values.into_iter().map(|raw| zero + scale * raw).collect())
            }
            // Text and undefined entries have nothing to scale.
            other => other,
        }
    }
}
