use crate::bin_table::Value;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use core::error::Error;
use core::str::from_utf8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableColumnFormat {
    String(usize),
    StringArray(usize, usize),
    Boolean(usize),
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
    C32(usize),
    M64(usize),
}

impl TableColumnFormat {
    pub fn parse_into_value(&self, data: &[u8]) -> crate::Result<Value> {
        match self {
            TableColumnFormat::String(byte_count) => {
                let end = *byte_count;
                Ok(Value::String(
                    from_utf8(&data[..end])
                        .map_err(|e| {
                            crate::Error::DeserializationError(format!("Not valid UTF-8: {}", e))
                        })?
                        .replace("\0", "")
                        .trim_ascii()
                        .to_string(),
                ))
            }
            TableColumnFormat::StringArray(byte_count, items) => Ok(Value::StringArray(
                (0..*items)
                    .map(|index| {
                        let start = byte_count * index;
                        let end = start + byte_count;
                        Ok(from_utf8(&data[start..end])
                            .map_err(|e| {
                                crate::Error::DeserializationError(format!(
                                    "Not valid UTF-8: {}",
                                    e
                                ))
                            })?
                            .to_string())
                    })
                    .collect::<crate::Result<_>>()?,
            )),
            TableColumnFormat::Boolean(item_count) => {
                let end = item_count * 1;
                Ok(Value::Boolean(
                    (data[..end]).into_iter().map(|i| *i != 0).collect(),
                ))
            }
            TableColumnFormat::Bit(item_count) => Ok(Value::U8(data[..*item_count].to_vec())),
            TableColumnFormat::U8(item_count) => Ok(Value::U8(data[..*item_count].to_vec())),
            TableColumnFormat::I8(item_count) => Ok(Value::I8(
                (data[..*item_count])
                    .into_iter()
                    .map(|i| *i as i8)
                    .collect(),
            )),
            TableColumnFormat::U16(item_count) => {
                let end = item_count * 2;
                Ok(Value::U16(
                    (data[..end])
                        .as_chunks::<2>()
                        .0
                        .into_iter()
                        .map(|i| u16::from_be_bytes(*i))
                        .collect(),
                ))
            }
            TableColumnFormat::I16(item_count) => {
                let end = item_count * 2;
                Ok(Value::I16(
                    (data[..end])
                        .as_chunks::<2>()
                        .0
                        .into_iter()
                        .map(|i| i16::from_be_bytes(*i))
                        .collect(),
                ))
            }
            TableColumnFormat::U32(item_count) => {
                let end = item_count * 4;
                Ok(Value::U32(
                    (data[..end])
                        .as_chunks::<4>()
                        .0
                        .into_iter()
                        .map(|i| u32::from_be_bytes(*i))
                        .collect(),
                ))
            }
            TableColumnFormat::I32(item_count) => {
                let end = item_count * 4;
                Ok(Value::I32(
                    (data[..end])
                        .as_chunks::<4>()
                        .0
                        .into_iter()
                        .map(|i| i32::from_be_bytes(*i))
                        .collect(),
                ))
            }
            TableColumnFormat::I64(item_count) => {
                let end = item_count * 8;
                Ok(Value::I64(
                    (data[..end])
                        .as_chunks::<8>()
                        .0
                        .into_iter()
                        .map(|i| i64::from_be_bytes(*i))
                        .collect(),
                ))
            }
            TableColumnFormat::F32(item_count) => {
                let end = item_count * 4;
                Ok(Value::F32(
                    (data[..end])
                        .as_chunks::<4>()
                        .0
                        .into_iter()
                        .map(|i| f32::from_be_bytes(*i))
                        .collect(),
                ))
            }
            TableColumnFormat::F64(item_count) => {
                let end = item_count * 8;
                Ok(Value::F64(
                    (data[..end])
                        .as_chunks::<8>()
                        .0
                        .into_iter()
                        .map(|i| f64::from_be_bytes(*i))
                        .collect(),
                ))
            }
            TableColumnFormat::C32(_item_count) => Ok(Value::String("".to_string())),
            TableColumnFormat::M64(_item_count) => Ok(Value::String("".to_string())),
        }
    }

    pub fn bytes_len(&self) -> usize {
        match self {
            TableColumnFormat::String(byte_count) => *byte_count,
            TableColumnFormat::StringArray(byte_count, items) => byte_count * items,
            TableColumnFormat::Boolean(item_count) => *item_count,
            TableColumnFormat::Bit(item_count) => *item_count,
            TableColumnFormat::U8(item_count) => *item_count,
            TableColumnFormat::I8(item_count) => *item_count,
            TableColumnFormat::U16(item_count) => 2 * item_count,
            TableColumnFormat::I16(item_count) => 2 * item_count,
            TableColumnFormat::U32(item_count) => 4 * item_count,
            TableColumnFormat::I32(item_count) => 4 * item_count,
            TableColumnFormat::I64(item_count) => 8 * item_count,
            TableColumnFormat::F32(item_count) => 4 * item_count,
            TableColumnFormat::F64(item_count) => 8 * item_count,
            TableColumnFormat::C32(item_count) => 4 * item_count,
            TableColumnFormat::M64(item_count) => 8 * item_count,
        }
    }
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
