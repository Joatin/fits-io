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
    /// `rB`: `r` unsigned bytes.
    U8(usize),
    /// `rS`: `r` signed bytes.
    I8(usize),
    /// `rU`: `r` unsigned 16-bit integers.
    U16(usize),
    /// `rI`: `r` signed 16-bit integers.
    I16(usize),
    /// `rV`: `r` unsigned 32-bit integers.
    U32(usize),
    /// `rJ`: `r` signed 32-bit integers.
    I32(usize),
    /// `rK`: `r` signed 64-bit integers.
    I64(usize),
    /// `rE`: `r` single precision floats.
    F32(usize),
    /// `rD`: `r` double precision floats.
    F64(usize),
    /// `rC`: `r` single precision complex values, two `f32` each.
    C32(usize),
    /// `rM`: `r` double precision complex values, two `f64` each.
    M64(usize),
    /// `rPt(max)` or `rQt(max)`: a variable length array.
    ///
    /// The row itself holds only a descriptor saying how many values there are
    /// and where in the table's heap they start; the values live in the heap,
    /// after the last row.
    VariableLengthArray {
        /// The type of the values in the heap.
        element: TableElementFormat,
        /// Which descriptor the row carries: `P` for 32-bit, `Q` for 64-bit.
        descriptor: ArrayDescriptor,
        /// The `(max)` hint on the format, or 0 when it carries none. This is
        /// the longest array the column promises, not the length of any
        /// particular row's array.
        max: usize,
    },
}

/// Which of the two variable length array descriptors a column uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArrayDescriptor {
    /// `P`: a pair of 32-bit integers, so a heap of up to 2 GiB.
    P32,
    /// `Q`: a pair of 64-bit integers, for heaps beyond that.
    Q64,
}

impl ArrayDescriptor {
    /// Width of the descriptor as stored in the row.
    pub fn bytes_len(&self) -> usize {
        match self {
            ArrayDescriptor::P32 => 8,
            ArrayDescriptor::Q64 => 16,
        }
    }

    /// Reads a descriptor: how many elements the array has, and its byte offset
    /// into the heap.
    ///
    /// Returns `None` for a descriptor that is truncated, or that describes a
    /// negative count or offset.
    pub fn read(&self, bytes: &[u8]) -> Option<(usize, usize)> {
        let (count, offset) = match self {
            ArrayDescriptor::P32 => {
                let (count, offset) = bytes.get(..8)?.split_at(4);
                (
                    i32::from_be_bytes(count.try_into().ok()?) as i64,
                    i32::from_be_bytes(offset.try_into().ok()?) as i64,
                )
            }
            ArrayDescriptor::Q64 => {
                let (count, offset) = bytes.get(..16)?.split_at(8);
                (
                    i64::from_be_bytes(count.try_into().ok()?),
                    i64::from_be_bytes(offset.try_into().ok()?),
                )
            }
        };

        Some((usize::try_from(count).ok()?, usize::try_from(offset).ok()?))
    }
}

/// The type of a single element, for the columns whose length is not fixed by
/// their format.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableElementFormat {
    /// `A`: one character.
    Character,
    /// `L`: a logical.
    Boolean,
    /// `X`: a bit.
    Bit,
    /// `B`: an unsigned byte.
    U8,
    /// `S`: a signed byte.
    I8,
    /// `U`: an unsigned 16-bit integer.
    U16,
    /// `I`: a signed 16-bit integer.
    I16,
    /// `V`: an unsigned 32-bit integer.
    U32,
    /// `J`: a signed 32-bit integer.
    I32,
    /// `K`: a signed 64-bit integer.
    I64,
    /// `E`: a single precision float.
    F32,
    /// `D`: a double precision float.
    F64,
    /// `C`: a single precision complex value.
    C32,
    /// `M`: a double precision complex value.
    M64,
}

impl TableElementFormat {
    /// The TFORMn code for this element type.
    pub fn code(&self) -> char {
        match self {
            TableElementFormat::Character => 'A',
            TableElementFormat::Boolean => 'L',
            TableElementFormat::Bit => 'X',
            TableElementFormat::U8 => 'B',
            TableElementFormat::I8 => 'S',
            TableElementFormat::U16 => 'U',
            TableElementFormat::I16 => 'I',
            TableElementFormat::U32 => 'V',
            TableElementFormat::I32 => 'J',
            TableElementFormat::I64 => 'K',
            TableElementFormat::F32 => 'E',
            TableElementFormat::F64 => 'D',
            TableElementFormat::C32 => 'C',
            TableElementFormat::M64 => 'M',
        }
    }

    fn from_code(code: char) -> Option<Self> {
        Some(match code {
            'A' => TableElementFormat::Character,
            'L' => TableElementFormat::Boolean,
            'X' => TableElementFormat::Bit,
            'B' => TableElementFormat::U8,
            'S' => TableElementFormat::I8,
            'U' => TableElementFormat::U16,
            'I' => TableElementFormat::I16,
            'V' => TableElementFormat::U32,
            'J' => TableElementFormat::I32,
            'K' => TableElementFormat::I64,
            'E' => TableElementFormat::F32,
            'D' => TableElementFormat::F64,
            'C' => TableElementFormat::C32,
            'M' => TableElementFormat::M64,
            _ => return None,
        })
    }

    /// This element type repeated `count` times, as a fixed-width column format.
    ///
    /// That is what a variable length array becomes once its descriptor has said
    /// how long it actually is.
    pub fn repeated(&self, count: usize) -> TableColumnFormat {
        match self {
            TableElementFormat::Character => TableColumnFormat::String(count),
            TableElementFormat::Boolean => TableColumnFormat::Boolean(count),
            TableElementFormat::Bit => TableColumnFormat::Bit(count),
            TableElementFormat::U8 => TableColumnFormat::U8(count),
            TableElementFormat::I8 => TableColumnFormat::I8(count),
            TableElementFormat::U16 => TableColumnFormat::U16(count),
            TableElementFormat::I16 => TableColumnFormat::I16(count),
            TableElementFormat::U32 => TableColumnFormat::U32(count),
            TableElementFormat::I32 => TableColumnFormat::I32(count),
            TableElementFormat::I64 => TableColumnFormat::I64(count),
            TableElementFormat::F32 => TableColumnFormat::F32(count),
            TableElementFormat::F64 => TableColumnFormat::F64(count),
            TableElementFormat::C32 => TableColumnFormat::C32(count),
            TableElementFormat::M64 => TableColumnFormat::M64(count),
        }
    }
}

impl TableColumnFormat {
    /// Decodes this column out of the front of `data`, which must be the
    /// remainder of the row starting at this column's offset.
    ///
    /// `heap` is the table's heap, which only a variable length array column
    /// reads from; pass an empty slice for a table that has none.
    pub fn parse_into_value(&self, data: &[u8], heap: &[u8]) -> crate::Result<Value> {
        if let TableColumnFormat::VariableLengthArray {
            element,
            descriptor,
            ..
        } = self
        {
            return self.parse_array_from_heap(*element, *descriptor, data, heap);
        }

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
            TableColumnFormat::Bit(count) => Ok(Value::Bit {
                bytes: bytes.to_vec(),
                len: *count,
            }),

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

            // Handled above, before the row slice is taken: this column's bytes
            // are in the heap, not in the row.
            TableColumnFormat::VariableLengthArray { .. } => {
                unreachable!("a variable length array column is decoded from the heap")
            }
        }
    }

    /// Follows a variable length array's descriptor into the heap and decodes
    /// the values it points at.
    fn parse_array_from_heap(
        &self,
        element: TableElementFormat,
        descriptor: ArrayDescriptor,
        data: &[u8],
        heap: &[u8],
    ) -> crate::Result<Value> {
        let (count, offset) = descriptor.read(data).ok_or_else(|| {
            crate::Error::DeserializationError(format!(
                "Column of format {} has an unreadable array descriptor",
                String::from(*self)
            ))
        })?;

        // A zero-length array is the normal way to say a row has no values for
        // this column, and points nowhere.
        let format = element.repeated(count);
        if count == 0 {
            return format.parse_into_value(&[], &[]);
        }

        let width = format.bytes_len();
        let bytes = heap
            .get(offset..)
            .and_then(|heap| heap.get(..width))
            .ok_or_else(|| {
                crate::Error::DeserializationError(format!(
                    "Column of format {} points at bytes {}..{} of a {} byte heap",
                    String::from(*self),
                    offset,
                    offset + width,
                    heap.len()
                ))
            })?;

        format.parse_into_value(bytes, &[])
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

            // Only the descriptor sits in the row; the values are in the heap.
            TableColumnFormat::VariableLengthArray { descriptor, .. } => descriptor.bytes_len(),
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

            // A variable length array is a different length in every row, so the
            // format can only report the upper bound it declares.
            TableColumnFormat::VariableLengthArray { max, .. } => *max,
        }
    }

    /// Whether this column holds no elements at all.
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
            TableColumnFormat::VariableLengthArray {
                element,
                descriptor,
                max,
            } => {
                let code = match descriptor {
                    ArrayDescriptor::P32 => 'P',
                    ArrayDescriptor::Q64 => 'Q',
                };
                format!("1{}{}({})", code, element.code(), max)
            }
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
            'P' | 'Q' => parse_variable_length_array(&value, repeat, format),
            _ => Err(From::from(format!(
                "Invalid TableColumnFormat value: {}",
                value
            ))),
        }
    }
}

/// Parses `rPt(max)` / `rQt(max)`, the two variable length array formats.
///
/// The standard allows only a repeat count of 0 or 1 here: the row holds one
/// descriptor, and the count of values is in the descriptor rather than the
/// format.
fn parse_variable_length_array(
    value: &str,
    repeat: usize,
    code: char,
) -> Result<TableColumnFormat, Box<dyn Error + Send + Sync>> {
    if repeat > 1 {
        return Err(From::from(format!(
            "A variable length array column holds one descriptor, so its repeat count must be 0 \
             or 1, but {} says {}",
            value, repeat
        )));
    }

    let descriptor = match code {
        'P' => ArrayDescriptor::P32,
        _ => ArrayDescriptor::Q64,
    };

    // Everything after the leading repeat count and the P or Q.
    let rest = value
        .trim_start_matches(|c: char| c.is_ascii_digit())
        .get(1..)
        .unwrap_or_default();

    let (element_code, rest) = {
        let mut chars = rest.chars();
        let element_code = chars.next().ok_or_else(|| {
            format!(
                "Variable length array format {} names no element type",
                value
            )
        })?;
        (element_code, chars.as_str())
    };

    let element = TableElementFormat::from_code(element_code).ok_or_else(|| {
        format!(
            "Variable length array format {} has an invalid element type: {}",
            value, element_code
        )
    })?;

    // The `(max)` suffix is a hint, and the standard makes it optional.
    let max = match rest
        .trim()
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
    {
        Some(max) => max.trim().parse::<usize>().map_err(|_| {
            format!(
                "Variable length array format {} has an invalid maximum",
                value
            )
        })?,
        None if rest.trim().is_empty() => 0,
        None => {
            return Err(From::from(format!(
                "Trailing characters in variable length array format {}",
                value
            )));
        }
    };

    Ok(TableColumnFormat::VariableLengthArray {
        element,
        descriptor,
        max,
    })
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
    use super::{ArrayDescriptor, TableColumnFormat, TableElementFormat};
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

        let Ok(Value::C32(values)) = format("1C").parse_into_value(&bytes, &[]) else {
            panic!("a 1C column should decode to a complex value");
        };

        assert_eq!(values, vec![(1.5, -2.5)]);
    }

    #[test]
    fn double_precision_complex_decodes_both_components() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1.5_f64.to_be_bytes());
        bytes.extend_from_slice(&(-2.5_f64).to_be_bytes());

        let Ok(Value::M64(values)) = format("1M").parse_into_value(&bytes, &[]) else {
            panic!("a 1M column should decode to a complex value");
        };

        assert_eq!(values, vec![(1.5, -2.5)]);
    }

    #[test]
    fn a_string_array_splits_into_substrings_of_the_declared_width() {
        // 15A5 is 15 characters holding three 5-character substrings, each
        // space padded to its full width.
        let Ok(Value::StringArray(values)) =
            format("15A5").parse_into_value(b"alphabeta gamma", &[])
        else {
            panic!("a 15A5 column should decode to a string array");
        };

        assert_eq!(values, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn logical_columns_distinguish_true_from_false() {
        // 'F' is a non-zero byte, so a zero test reports it as true.
        let Ok(Value::Boolean(values)) = format("3L").parse_into_value(b"TF\0", &[]) else {
            panic!("a 3L column should decode to logicals");
        };

        assert_eq!(values, vec![true, false, false]);
    }

    #[test]
    fn a_row_too_short_for_the_column_is_an_error() {
        let error = format("4J")
            .parse_into_value(&[0, 0, 0, 1, 0, 0], &[])
            .expect_err("a 16 byte column cannot be read from 6 bytes");

        assert!(error.to_string().contains("needs 16 bytes"), "got: {error}");
    }

    #[test]
    fn variable_length_array_formats_parse() {
        assert_eq!(
            format("1PJ(10)"),
            TableColumnFormat::VariableLengthArray {
                element: TableElementFormat::I32,
                descriptor: ArrayDescriptor::P32,
                max: 10,
            }
        );

        // The `(max)` hint is optional, and Q is the 64-bit descriptor.
        assert_eq!(
            format("1QE"),
            TableColumnFormat::VariableLengthArray {
                element: TableElementFormat::F32,
                descriptor: ArrayDescriptor::Q64,
                max: 0,
            }
        );

        // Only the descriptor occupies space in the row.
        assert_eq!(format("1PJ(10)").bytes_len(), 8);
        assert_eq!(format("1QE").bytes_len(), 16);
    }

    #[test]
    fn a_variable_length_array_repeat_count_above_one_is_rejected() {
        // A row holds exactly one descriptor, so `2PJ` is not a thing.
        let error = TableColumnFormat::try_from("2PJ(10)".to_string())
            .expect_err("a repeat count above one is invalid");

        assert!(error.to_string().contains("repeat count"), "got: {error}");
    }

    #[test]
    fn a_variable_length_array_reads_its_values_from_the_heap() {
        // Descriptor: three elements, starting at byte 4 of the heap.
        let mut descriptor = Vec::new();
        descriptor.extend_from_slice(&3_i32.to_be_bytes());
        descriptor.extend_from_slice(&4_i32.to_be_bytes());

        let mut heap = vec![0xFF; 4];
        for value in [7_i32, 8, 9] {
            heap.extend_from_slice(&value.to_be_bytes());
        }

        let Ok(Value::I32(values)) = format("1PJ(10)").parse_into_value(&descriptor, &heap) else {
            panic!("a 1PJ column should decode to its heap values");
        };

        assert_eq!(values, vec![7, 8, 9]);
    }

    #[test]
    fn an_empty_variable_length_array_points_nowhere() {
        let descriptor = [0_u8; 8];

        let Ok(Value::I32(values)) = format("1PJ(10)").parse_into_value(&descriptor, &[]) else {
            panic!("a zero-length array should decode to no values");
        };

        assert!(values.is_empty());
    }

    #[test]
    fn a_variable_length_array_past_the_end_of_the_heap_is_an_error() {
        let mut descriptor = Vec::new();
        descriptor.extend_from_slice(&3_i32.to_be_bytes());
        descriptor.extend_from_slice(&100_i32.to_be_bytes());

        let error = format("1PJ(10)")
            .parse_into_value(&descriptor, &[0; 8])
            .expect_err("an out of range descriptor cannot be followed");

        assert!(error.to_string().contains("heap"), "got: {error}");
    }
}
