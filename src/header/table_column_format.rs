use crate::bin_table::Value;
use std::error::Error;
use std::str::from_utf8;

/// The type and repeat count of one binary table column, from its TFORMn card.
///
/// The contained count is the TFORMn repeat count `r`, not a byte length; use
/// [`TableColumnFormat::bytes_len`] for the width of the field in the row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableColumnFormat {
    /// `rA`: one string of `r` characters.
    String(usize),
    /// `rAw`: `r` characters holding `r / w` substrings of `w` characters.
    StringArray(usize, usize),
    /// `rL`: `r` logical values, one byte each.
    Boolean(usize),
    /// `rX`: `r` bits, packed into `ceil(r / 8)` bytes.
    Bit(usize),
    U8(usize),
    I8(usize),
    U16(usize),
    I16(usize),
    U32(usize),
    I32(usize),
    I64(usize),
    F32(usize),
    F64(usize),
    /// `rC`: `r` single precision complex values, two `f32` each.
    C32(usize),
    /// `rM`: `r` double precision complex values, two `f64` each.
    M64(usize),
}

impl TableColumnFormat {
    /// Decodes this column out of the front of `data`, which must be the
    /// remainder of the row starting at this column's offset.
    pub fn parse_into_value(&self, data: &[u8]) -> crate::Result<Value> {
        let width = self.bytes_len();

        let bytes = data.get(..width).ok_or_else(|| {
            crate::Error::DeserializationError(format!(
                "Column of format {} needs {} bytes but only {} remain in the row",
                String::from(*self),
                width,
                data.len()
            ))
        })?;

        match self {
            TableColumnFormat::String(_) => Ok(Value::String(decode_string(bytes)?)),

            TableColumnFormat::StringArray(_, substring_width) => {
                // TFORMn `rAw` is r characters total, split into substrings of w.
                let substring_width = (*substring_width).max(1);

                Ok(Value::StringArray(
                    bytes
                        .chunks(substring_width)
                        .map(decode_string)
                        .collect::<crate::Result<_>>()?,
                ))
            }

            // A logical is stored as ASCII 'T' or 'F'; a zero byte means
            // undefined. Anything else is not true.
            TableColumnFormat::Boolean(_) => Ok(Value::Boolean(
                bytes.iter().map(|byte| *byte == b'T').collect(),
            )),

            // The bits stay packed; `r` bits occupy ceil(r / 8) bytes.
            TableColumnFormat::Bit(_) => Ok(Value::Bit(bytes.to_vec())),

            TableColumnFormat::U8(_) => Ok(Value::U8(bytes.to_vec())),
            TableColumnFormat::I8(_) => {
                Ok(Value::I8(bytes.iter().map(|byte| *byte as i8).collect()))
            }

            TableColumnFormat::U16(_) => Ok(Value::U16(
                bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|value| u16::from_be_bytes(*value))
                    .collect(),
            )),
            TableColumnFormat::I16(_) => Ok(Value::I16(
                bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|value| i16::from_be_bytes(*value))
                    .collect(),
            )),
            TableColumnFormat::U32(_) => Ok(Value::U32(
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|value| u32::from_be_bytes(*value))
                    .collect(),
            )),
            TableColumnFormat::I32(_) => Ok(Value::I32(
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|value| i32::from_be_bytes(*value))
                    .collect(),
            )),
            TableColumnFormat::I64(_) => Ok(Value::I64(
                bytes
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|value| i64::from_be_bytes(*value))
                    .collect(),
            )),
            TableColumnFormat::F32(_) => Ok(Value::F32(
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|value| f32::from_be_bytes(*value))
                    .collect(),
            )),
            TableColumnFormat::F64(_) => Ok(Value::F64(
                bytes
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|value| f64::from_be_bytes(*value))
                    .collect(),
            )),

            // Complex values are a real part followed by an imaginary part.
            TableColumnFormat::C32(_) => Ok(Value::C32(
                bytes
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|value| {
                        let (real, imaginary) = value.split_at(4);
                        (
                            f32::from_be_bytes(real.try_into().expect("4 of 8 bytes")),
                            f32::from_be_bytes(imaginary.try_into().expect("4 of 8 bytes")),
                        )
                    })
                    .collect(),
            )),
            TableColumnFormat::M64(_) => Ok(Value::M64(
                bytes
                    .as_chunks::<16>()
                    .0
                    .iter()
                    .map(|value| {
                        let (real, imaginary) = value.split_at(8);
                        (
                            f64::from_be_bytes(real.try_into().expect("8 of 16 bytes")),
                            f64::from_be_bytes(imaginary.try_into().expect("8 of 16 bytes")),
                        )
                    })
                    .collect(),
            )),
        }
    }

    /// Width of this column in the row, in bytes.
    ///
    /// Every column's offset is the sum of the widths before it, so a wrong
    /// answer here misaligns every following column.
    pub fn bytes_len(&self) -> usize {
        match self {
            // `rA` and `rAw` both occupy r bytes; w only says how those bytes
            // are divided into substrings.
            TableColumnFormat::String(count) => *count,
            TableColumnFormat::StringArray(count, _) => *count,

            // r bits, rounded up to whole bytes.
            TableColumnFormat::Bit(count) => count.div_ceil(8),

            TableColumnFormat::Boolean(count) => *count,
            TableColumnFormat::U8(count) => *count,
            TableColumnFormat::I8(count) => *count,
            TableColumnFormat::U16(count) => 2 * count,
            TableColumnFormat::I16(count) => 2 * count,
            TableColumnFormat::U32(count) => 4 * count,
            TableColumnFormat::I32(count) => 4 * count,
            TableColumnFormat::I64(count) => 8 * count,
            TableColumnFormat::F32(count) => 4 * count,
            TableColumnFormat::F64(count) => 8 * count,

            // A complex value is a pair, so twice the width of its components.
            TableColumnFormat::C32(count) => 8 * count,
            TableColumnFormat::M64(count) => 16 * count,
        }
    }

    /// Number of elements this column holds.
    pub fn len(&self) -> usize {
        match self {
            TableColumnFormat::String(_) => 1,
            TableColumnFormat::StringArray(count, substring_width) => {
                count / (*substring_width).max(1)
            }
            TableColumnFormat::Boolean(count)
            | TableColumnFormat::Bit(count)
            | TableColumnFormat::U8(count)
            | TableColumnFormat::I8(count)
            | TableColumnFormat::U16(count)
            | TableColumnFormat::I16(count)
            | TableColumnFormat::U32(count)
            | TableColumnFormat::I32(count)
            | TableColumnFormat::I64(count)
            | TableColumnFormat::F32(count)
            | TableColumnFormat::F64(count)
            | TableColumnFormat::C32(count)
            | TableColumnFormat::M64(count) => *count,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Decodes one fixed-width FITS character field, which is space padded and may
/// be null terminated.
fn decode_string(bytes: &[u8]) -> crate::Result<String> {
    Ok(from_utf8(bytes)
        .map_err(|e| crate::Error::DeserializationError(format!("Not valid UTF-8: {}", e)))?
        .replace("\0", "")
        .trim_ascii()
        .to_string())
}

impl From<TableColumnFormat> for String {
    fn from(value: TableColumnFormat) -> String {
        match value {
            TableColumnFormat::String(repeat) => format!("{}A", repeat),
            TableColumnFormat::StringArray(repeat, items) => format!("{}A{}", repeat, items),
            TableColumnFormat::Boolean(repeat) => format!("{}L", repeat),
            TableColumnFormat::Bit(repeat) => format!("{}X", repeat),
            TableColumnFormat::U8(repeat) => format!("{}B", repeat),
            TableColumnFormat::I8(repeat) => format!("{}S", repeat),
            TableColumnFormat::U16(repeat) => format!("{}U", repeat),
            TableColumnFormat::I16(repeat) => format!("{}I", repeat),
            TableColumnFormat::U32(repeat) => format!("{}V", repeat),
            TableColumnFormat::I32(repeat) => format!("{}J", repeat),
            TableColumnFormat::I64(repeat) => format!("{}K", repeat),
            TableColumnFormat::F32(repeat) => format!("{}E", repeat),
            TableColumnFormat::F64(repeat) => format!("{}D", repeat),
            TableColumnFormat::C32(repeat) => format!("{}C", repeat),
            TableColumnFormat::M64(repeat) => format!("{}M", repeat),
        }
    }
}

impl TryFrom<String> for TableColumnFormat {
    type Error = Box<dyn Error + Send + Sync>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let (repeat, format, items) = extract_parts(&value)?;
        match format {
            'A' => {
                if items > 0 {
                    Ok(TableColumnFormat::StringArray(repeat, items))
                } else {
                    Ok(TableColumnFormat::String(repeat))
                }
            }
            'L' => Ok(TableColumnFormat::Boolean(repeat)),
            'X' => Ok(TableColumnFormat::Bit(repeat)),
            'B' => Ok(TableColumnFormat::U8(repeat)),
            'S' => Ok(TableColumnFormat::I8(repeat)),
            'I' => Ok(TableColumnFormat::I16(repeat)),
            'U' => Ok(TableColumnFormat::U16(repeat)),
            'J' => Ok(TableColumnFormat::I32(repeat)),
            'V' => Ok(TableColumnFormat::U32(repeat)),
            'K' => Ok(TableColumnFormat::I64(repeat)),
            'E' => Ok(TableColumnFormat::F32(repeat)),
            'D' => Ok(TableColumnFormat::F64(repeat)),
            'C' => Ok(TableColumnFormat::C32(repeat)),
            'M' => Ok(TableColumnFormat::M64(repeat)),
            // Variable length arrays store a descriptor in the row and the data
            // itself in the heap after the table, which is not read yet.
            'P' | 'Q' => Err(From::from(format!(
                "Variable length array columns are not supported yet: {}",
                value
            ))),
            _ => Err(From::from(format!(
                "Invalid TableColumnFormat value: {}",
                value
            ))),
        }
    }
}

fn extract_parts(value: &str) -> Result<(usize, char, usize), Box<dyn Error + Send + Sync>> {
    let mut chars = value.chars().peekable();
    let mut repeat_str = String::new();
    while let Some(c) = chars.peek() {
        if c.is_ascii_digit() {
            repeat_str.push(*c);
            chars.next();
        } else {
            break;
        }
    }

    let repeat = if repeat_str.is_empty() {
        1
    } else {
        repeat_str
            .parse::<usize>()
            .map_err(|_| "Invalid repeat count")?
    };

    // Parse type code
    let code = chars
        .next()
        .ok_or_else(|| "Missing format code".to_string())?;

    let mut width_str = String::new();
    while let Some(c) = chars.peek() {
        if c.is_ascii_digit() {
            width_str.push(*c);
            chars.next();
        } else {
            break;
        }
    }

    let width = if width_str.is_empty() {
        0
    } else {
        width_str
            .parse::<usize>()
            .map_err(|_| "Invalid string width")?
    };

    Ok((repeat, code, width))
}

#[cfg(test)]
mod tests {
    use super::TableColumnFormat;
    use crate::bin_table::Value;

    fn format(tform: &str) -> TableColumnFormat {
        TableColumnFormat::try_from(tform.to_string())
            .unwrap_or_else(|error| panic!("{tform} should parse: {error}"))
    }

    /// The byte width of every TFORMn code, per the FITS standard. A wrong width
    /// here shifts every following column in the row.
    #[test]
    fn every_format_code_reports_its_standard_width() {
        let cases = [
            ("1L", 1),
            ("8L", 8),
            // r bits rounded up to whole bytes.
            ("1X", 1),
            ("8X", 1),
            ("9X", 2),
            ("16X", 2),
            ("17X", 3),
            ("1B", 1),
            ("4B", 4),
            ("1I", 2),
            ("4I", 8),
            ("1J", 4),
            ("1K", 8),
            ("1E", 4),
            ("1D", 8),
            // A complex value is a pair of components.
            ("1C", 8),
            ("3C", 24),
            ("1M", 16),
            ("3M", 48),
            // rA and rAw both occupy r bytes.
            ("20A", 20),
            ("60A20", 60),
        ];

        for (tform, expected) in cases {
            assert_eq!(format(tform).bytes_len(), expected, "TFORM {tform}");
        }
    }

    #[test]
    fn element_counts_match_the_repeat_count() {
        assert_eq!(format("20A").len(), 1);
        assert_eq!(format("60A20").len(), 3);
        assert_eq!(format("4J").len(), 4);
        assert_eq!(format("3C").len(), 3);
    }

    #[test]
    fn single_precision_complex_decodes_both_components() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1.5_f32.to_be_bytes());
        bytes.extend_from_slice(&(-2.5_f32).to_be_bytes());

        let Ok(Value::C32(values)) = format("1C").parse_into_value(&bytes) else {
            panic!("a 1C column should decode to a complex value");
        };

        assert_eq!(values, vec![(1.5, -2.5)]);
    }

    #[test]
    fn double_precision_complex_decodes_both_components() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1.5_f64.to_be_bytes());
        bytes.extend_from_slice(&(-2.5_f64).to_be_bytes());

        let Ok(Value::M64(values)) = format("1M").parse_into_value(&bytes) else {
            panic!("a 1M column should decode to a complex value");
        };

        assert_eq!(values, vec![(1.5, -2.5)]);
    }

    #[test]
    fn a_string_array_splits_into_substrings_of_the_declared_width() {
        // 15A5 is 15 characters holding three 5-character substrings, each
        // space padded to its full width.
        let Ok(Value::StringArray(values)) = format("15A5").parse_into_value(b"alphabeta gamma")
        else {
            panic!("a 15A5 column should decode to a string array");
        };

        assert_eq!(values, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn logical_columns_distinguish_true_from_false() {
        // 'F' is a non-zero byte, so a zero test reports it as true.
        let Ok(Value::Boolean(values)) = format("3L").parse_into_value(b"TF\0") else {
            panic!("a 3L column should decode to logicals");
        };

        assert_eq!(values, vec![true, false, false]);
    }

    #[test]
    fn a_row_too_short_for_the_column_is_an_error() {
        let error = format("4J")
            .parse_into_value(&[0, 0, 0, 1, 0, 0])
            .expect_err("a 16 byte column cannot be read from 6 bytes");

        assert!(error.to_string().contains("needs 16 bytes"), "got: {error}");
    }

    #[test]
    fn variable_length_array_columns_report_that_they_are_unsupported() {
        let error = TableColumnFormat::try_from("1PJ(10)".to_string())
            .expect_err("variable length arrays are not supported");

        assert!(
            error.to_string().contains("not supported yet"),
            "got: {error}"
        );
    }
}
