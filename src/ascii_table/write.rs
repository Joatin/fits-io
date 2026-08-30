use crate::ascii_table::{AsciiColumnFormat, AsciiFieldDefinition, AsciiTable};
use crate::bin_table::Value;
use crate::header::card::Card;
use crate::header::{Bitpix, ExtensionType, Header, TableNullValue};

/// Renders one column entry as the fixed-width text an ASCII table stores.
///
/// The field is right-justified for numbers and left-justified for text, which
/// is how the standard's display formats read, and a value too wide for its
/// field is written as blanks rather than as a number with its digits cut off.
pub(crate) fn encode(value: &Value, format: AsciiColumnFormat, null: Option<&str>) -> String {
    let width = format.bytes_len();
    let text = render(value, format, null);

    match format {
        // Text is left-justified, and a string too long for its field is cut:
        // a shortened name is still recognisably itself.
        AsciiColumnFormat::Character(_) => {
            format!("{:<width$}", truncated(&text, width), width = width)
        }

        // A number that will not fit cannot be cut down without changing its
        // value, so the field is left blank: undefined is closer to the truth
        // than wrong.
        _ if text.len() > width => " ".repeat(width),

        _ => format!("{:>width$}", text, width = width),
    }
}

/// Renders one value as text, with no padding or justification.
///
/// This is what decides how wide a column has to be, so it is kept separate from
/// the padding that fits it into a field of that width.
pub(crate) fn render(value: &Value, format: AsciiColumnFormat, null: Option<&str>) -> String {
    match (value, format) {
        // An undefined entry is written as its TNULLn text, or as blanks when
        // the column declares none.
        (Value::Null, _) => null.unwrap_or("").to_string(),

        (value, AsciiColumnFormat::Character(_)) => match value {
            Value::String(text) => text.clone(),
            other => display(other),
        },

        (value, AsciiColumnFormat::Integer(_)) => match as_integer(value) {
            Some(number) => number.to_string(),
            None => String::new(),
        },

        (value, AsciiColumnFormat::Fixed(_, decimals)) => match as_float(value) {
            Some(number) => format!("{:.*}", decimals, number),
            None => String::new(),
        },

        (value, AsciiColumnFormat::Exponential(_, decimals)) => match as_float(value) {
            Some(number) => format!("{:.*E}", decimals, number),
            None => String::new(),
        },

        // A double is written with `D` as its exponent marker, which is what
        // FITS expects and what the reader looks for.
        (value, AsciiColumnFormat::Double(_, decimals)) => match as_float(value) {
            Some(number) => format!("{:.*E}", decimals, number).replace('E', "D"),
            None => String::new(),
        },
    }
}

fn truncated(text: &str, width: usize) -> &str {
    match text.char_indices().nth(width) {
        Some((index, _)) => &text[..index],
        None => text,
    }
}

fn display(value: &Value) -> String {
    match as_integer(value) {
        Some(number) => number.to_string(),
        None => match as_float(value) {
            Some(number) => number.to_string(),
            None => String::new(),
        },
    }
}

fn as_integer(value: &Value) -> Option<i64> {
    Some(match value {
        Value::U8(values) => *values.first()? as i64,
        Value::I8(values) => *values.first()? as i64,
        Value::U16(values) => *values.first()? as i64,
        Value::I16(values) => *values.first()? as i64,
        Value::U32(values) => *values.first()? as i64,
        Value::I32(values) => *values.first()? as i64,
        Value::I64(values) => *values.first()?,
        Value::U64(values) => *values.first()? as i64,
        _ => return None,
    })
}

fn as_float(value: &Value) -> Option<f64> {
    match value {
        Value::F32(values) => values.first().map(|value| *value as f64),
        Value::F64(values) => values.first().copied(),
        other => as_integer(other).map(|value| value as f64),
    }
}

/// Rewrites `header` to describe `table`.
///
/// An ASCII table needs a TBCOLn for every column as well as a TFORMn: unlike a
/// binary table, nothing else says where in the row a column starts.
pub(crate) fn apply_to_header(table: &AsciiTable, header: &mut Header) {
    crate::bin_table::write::clear_column_cards(header);

    header.set(Card::Xtension {
        value: ExtensionType::AsciiTable,
        comment: None,
    });
    header.set(Card::Bitpix {
        value: Bitpix::U8,
        comment: None,
    });
    header.set(Card::NAxis {
        value: 2,
        comment: None,
    });
    header.set(Card::NAxisN {
        index: 0,
        value: table.bytes_per_row() as i64,
        comment: Some("width of a row in characters".into()),
    });
    header.set(Card::NAxisN {
        index: 1,
        value: table.len() as i64,
        comment: Some("number of rows".into()),
    });
    header.set(Card::ParameterCount {
        value: 0,
        comment: None,
    });
    header.set(Card::GroupCount {
        value: 1,
        comment: None,
    });
    header.set(Card::TableFields {
        value: table.field_definitions().len() as i64,
        comment: None,
    });

    for (index, field) in table.field_definitions().iter().enumerate() {
        apply_column(header, index, field);
    }
}

fn apply_column(header: &mut Header, index: usize, field: &AsciiFieldDefinition) {
    header.set(Card::TableFormatN {
        index,
        value: String::from(field.format),
        comment: None,
    });

    // TBCOLn counts from 1, the offset from 0.
    header.set(Card::TableColumnN {
        index,
        value: field.offset as i64 + 1,
        comment: None,
    });

    if !field.name.is_empty() {
        header.set(Card::TableTypeN {
            index,
            value: field.name.clone(),
            comment: None,
        });
    }
    if let Some(scale) = field.scale {
        header.set(Card::TableScalingFactorN {
            index,
            value: scale,
            comment: None,
        });
    }
    if let Some(zero) = field.zero {
        header.set(Card::TableScalingZeroPointN {
            index,
            value: zero,
            comment: None,
        });
    }
    if let Some(null) = &field.null {
        header.set(Card::TableNullValueN {
            index,
            value: TableNullValue::Text(null.clone()),
            comment: Some("text marking an undefined entry".into()),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::encode;
    use crate::ascii_table::AsciiColumnFormat;
    use crate::bin_table::Value;

    #[test]
    fn numbers_are_right_justified_in_their_field() {
        assert_eq!(
            encode(&Value::I64(vec![42]), AsciiColumnFormat::Integer(6), None),
            "    42"
        );
        assert_eq!(
            encode(
                &Value::F64(vec![-12.5]),
                AsciiColumnFormat::Fixed(8, 2),
                None
            ),
            "  -12.50"
        );
    }

    #[test]
    fn text_is_left_justified_and_padded() {
        assert_eq!(
            encode(
                &Value::String("M31".into()),
                AsciiColumnFormat::Character(6),
                None
            ),
            "M31   "
        );
    }

    #[test]
    fn a_double_uses_the_d_exponent_marker() {
        // A reader looking for FITS's `D` would not recognise `1.5000E2`.
        let written = encode(
            &Value::F64(vec![150.0]),
            AsciiColumnFormat::Double(12, 4),
            None,
        );

        assert!(written.contains('D'), "got {written:?}");
        assert!(!written.contains('E'), "got {written:?}");
    }

    #[test]
    fn an_undefined_entry_writes_its_null_text() {
        assert_eq!(
            encode(&Value::Null, AsciiColumnFormat::Integer(6), Some("---")),
            "   ---"
        );
        assert_eq!(
            encode(&Value::Null, AsciiColumnFormat::Integer(6), None),
            "      "
        );
    }

    #[test]
    fn a_number_too_wide_for_its_field_is_left_blank() {
        // Cutting digits off a number changes its value, so a field that cannot
        // hold it says nothing rather than something false.
        assert_eq!(
            encode(
                &Value::I64(vec![1234567]),
                AsciiColumnFormat::Integer(4),
                None
            ),
            "    "
        );
    }

    #[test]
    fn text_too_wide_for_its_field_is_cut() {
        // Unlike a number, a shortened string is still recognisably itself, and
        // the standard's character fields are fixed width.
        assert_eq!(
            encode(
                &Value::String("Betelgeuse".into()),
                AsciiColumnFormat::Character(4),
                None
            ),
            "Bete"
        );
    }
}
