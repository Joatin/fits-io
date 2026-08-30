use crate::bin_table::Value;
use crate::header::TableColumnFormat;

/// Writes one column entry into a row, padded out to the column's full width.
///
/// A row is fixed width, so a column entry shorter than its format declares —
/// the third of three strings in a `30A10` column, say — still has to occupy its
/// whole field. Numeric columns pad with zero and character columns with blanks,
/// as the standard asks.
pub(crate) fn encode(value: &Value, format: TableColumnFormat, out: &mut Vec<u8>) {
    let start = out.len();
    let width = format.bytes_len();

    // A column has one type, and the values in it may not all have arrived as
    // that type — an `i16` and an `i32` field in the same column make a `J`
    // column, and the `i16` has to be widened before it is written. Writing it
    // at its own width and cutting the result to the column's would keep the
    // wrong end of the number.
    let value = &coerce(value, format);

    match value {
        Value::Null => {}

        Value::String(text) => out.extend_from_slice(text.as_bytes()),
        Value::StringArray(values) => {
            for text in values {
                out.extend_from_slice(text.as_bytes());
            }
        }

        Value::Boolean(values) => {
            out.extend(values.iter().map(|value| if *value { b'T' } else { b'F' }));
        }
        Value::U8(values) => out.extend_from_slice(values),
        Value::Bit { bytes, .. } => out.extend_from_slice(bytes),
        Value::I8(values) => out.extend(values.iter().map(|value| *value as u8)),

        Value::U16(values) => extend(out, values, u16::to_be_bytes),
        Value::I16(values) => extend(out, values, i16::to_be_bytes),
        Value::U32(values) => extend(out, values, u32::to_be_bytes),
        Value::I32(values) => extend(out, values, i32::to_be_bytes),
        Value::I64(values) => extend(out, values, i64::to_be_bytes),
        Value::U64(values) => extend(out, values, u64::to_be_bytes),
        Value::F32(values) => extend(out, values, f32::to_be_bytes),
        Value::F64(values) => extend(out, values, f64::to_be_bytes),

        Value::C32(values) => {
            for (real, imaginary) in values {
                out.extend_from_slice(&real.to_be_bytes());
                out.extend_from_slice(&imaginary.to_be_bytes());
            }
        }
        Value::M64(values) => {
            for (real, imaginary) in values {
                out.extend_from_slice(&real.to_be_bytes());
                out.extend_from_slice(&imaginary.to_be_bytes());
            }
        }
    }

    // A value wider than its column would push every following column out of
    // place, so it is cut rather than allowed to overflow.
    out.truncate(start + width);

    let padding = match format {
        TableColumnFormat::String(_) | TableColumnFormat::StringArray(..) => b' ',
        _ => 0,
    };
    out.resize(start + width, padding);
}

/// Re-types a value into the column it is being written to.
///
/// Only the numeric types convert; anything else is left as it is, and a value
/// that genuinely does not belong in the column falls through to the width
/// guard in [`encode`].
fn coerce(value: &Value, format: TableColumnFormat) -> Value {
    let Some(integers) = as_integers(value) else {
        return match (value, format) {
            (value, TableColumnFormat::F32(_)) => match as_floats(value) {
                Some(floats) => Value::F32(floats.into_iter().map(|v| v as f32).collect()),
                None => value.clone(),
            },
            (value, TableColumnFormat::F64(_)) => match as_floats(value) {
                Some(floats) => Value::F64(floats),
                None => value.clone(),
            },
            (value, _) => value.clone(),
        };
    };

    match format {
        TableColumnFormat::U8(_) => Value::U8(integers.iter().map(|v| *v as u8).collect()),
        TableColumnFormat::I8(_) => Value::I8(integers.iter().map(|v| *v as i8).collect()),
        TableColumnFormat::U16(_) => Value::U16(integers.iter().map(|v| *v as u16).collect()),
        TableColumnFormat::I16(_) => Value::I16(integers.iter().map(|v| *v as i16).collect()),
        TableColumnFormat::U32(_) => Value::U32(integers.iter().map(|v| *v as u32).collect()),
        TableColumnFormat::I32(_) => Value::I32(integers.iter().map(|v| *v as i32).collect()),
        TableColumnFormat::I64(_) => Value::I64(integers.iter().map(|v| *v as i64).collect()),
        TableColumnFormat::F32(_) => Value::F32(integers.iter().map(|v| *v as f32).collect()),
        TableColumnFormat::F64(_) => Value::F64(integers.iter().map(|v| *v as f64).collect()),
        _ => value.clone(),
    }
}

/// The integer values in `value`, or `None` if it holds something else.
fn as_integers(value: &Value) -> Option<Vec<i128>> {
    Some(match value {
        Value::U8(values) => values.iter().map(|v| *v as i128).collect(),
        Value::I8(values) => values.iter().map(|v| *v as i128).collect(),
        Value::U16(values) => values.iter().map(|v| *v as i128).collect(),
        Value::I16(values) => values.iter().map(|v| *v as i128).collect(),
        Value::U32(values) => values.iter().map(|v| *v as i128).collect(),
        Value::I32(values) => values.iter().map(|v| *v as i128).collect(),
        Value::I64(values) => values.iter().map(|v| *v as i128).collect(),
        Value::U64(values) => values.iter().map(|v| *v as i128).collect(),
        _ => return None,
    })
}

/// The floating point values in `value`, widening integers on the way.
fn as_floats(value: &Value) -> Option<Vec<f64>> {
    match value {
        Value::F32(values) => Some(values.iter().map(|v| *v as f64).collect()),
        Value::F64(values) => Some(values.clone()),
        other => as_integers(other).map(|values| values.into_iter().map(|v| v as f64).collect()),
    }
}

fn extend<T: Copy, const N: usize>(
    out: &mut Vec<u8>,
    values: &[T],
    to_be_bytes: impl Fn(T) -> [u8; N],
) {
    for value in values {
        out.extend_from_slice(&to_be_bytes(*value));
    }
}
