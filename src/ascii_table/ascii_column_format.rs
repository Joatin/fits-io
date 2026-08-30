use crate::bin_table::Value;
use std::error::Error;
use std::str::from_utf8;

/// The type and width of one ASCII table column, from its TFORMn card.
///
/// An ASCII table stores every value as human-readable text in a fixed-width
/// field, so a format says how wide the field is and how to read the characters
/// in it — not how many bytes a number occupies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AsciiColumnFormat {
    /// `Aw`: `w` characters of text.
    Character(usize),
    /// `Iw`: an integer written in `w` characters.
    Integer(usize),
    /// `Fw.d`: a fixed-point number in `w` characters, `d` of them after the
    /// decimal point.
    Fixed(usize, usize),
    /// `Ew.d`: a number in exponential notation.
    Exponential(usize, usize),
    /// `Dw.d`: exponential notation for a double, written with `D` rather than
    /// `E` as the exponent marker.
    Double(usize, usize),
}

impl AsciiColumnFormat {
    /// Width of this column's field, in characters.
    pub fn bytes_len(&self) -> usize {
        match self {
            AsciiColumnFormat::Character(width)
            | AsciiColumnFormat::Integer(width)
            | AsciiColumnFormat::Fixed(width, _)
            | AsciiColumnFormat::Exponential(width, _)
            | AsciiColumnFormat::Double(width, _) => *width,
        }
    }

    /// Decodes one field of this column.
    ///
    /// `field` is the column's characters from the row, already cut to width.
    /// An all-blank field is undefined, which the standard allows for any
    /// column, and reads as [`Value::Null`].
    pub fn parse_into_value(&self, field: &[u8]) -> crate::Result<Value> {
        let text = from_utf8(field).map_err(|error| {
            crate::Error::DeserializationError(format!("Column is not valid UTF-8: {}", error))
        })?;

        // Character columns keep their padding as significant only up to the
        // trailing blanks, which the standard says to ignore.
        if let AsciiColumnFormat::Character(_) = self {
            return Ok(Value::String(text.trim_end().to_string()));
        }

        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(Value::Null);
        }

        match self {
            AsciiColumnFormat::Character(_) => unreachable!("handled above"),

            AsciiColumnFormat::Integer(_) => trimmed
                .parse::<i64>()
                .map(|value| Value::I64(vec![value]))
                .map_err(|_| self.invalid(trimmed)),

            // `D` is the FITS spelling of an exponent marker on a double, and
            // some writers use it in `E` fields too, so both are accepted here.
            AsciiColumnFormat::Fixed(..)
            | AsciiColumnFormat::Exponential(..)
            | AsciiColumnFormat::Double(..) => trimmed
                .replace(['D', 'd'], "E")
                .parse::<f64>()
                .map(|value| Value::F64(vec![value]))
                .map_err(|_| self.invalid(trimmed)),
        }
    }

    fn invalid(&self, text: &str) -> crate::Error {
        crate::Error::DeserializationError(format!(
            "Column of format {} cannot read {:?}",
            String::from(*self),
            text
        ))
    }
}

impl From<AsciiColumnFormat> for String {
    fn from(value: AsciiColumnFormat) -> String {
        match value {
            AsciiColumnFormat::Character(width) => format!("A{}", width),
            AsciiColumnFormat::Integer(width) => format!("I{}", width),
            AsciiColumnFormat::Fixed(width, decimals) => format!("F{}.{}", width, decimals),
            AsciiColumnFormat::Exponential(width, decimals) => format!("E{}.{}", width, decimals),
            AsciiColumnFormat::Double(width, decimals) => format!("D{}.{}", width, decimals),
        }
    }
}

impl TryFrom<String> for AsciiColumnFormat {
    type Error = Box<dyn Error + Send + Sync>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value = value.trim();
        let mut chars = value.chars();

        let code = chars
            .next()
            .ok_or_else(|| format!("Empty ASCII table format: {:?}", value))?;

        let rest = chars.as_str();
        let (width, decimals) = match rest.split_once('.') {
            Some((width, decimals)) => (width, Some(decimals)),
            None => (rest, None),
        };

        let width = width
            .trim()
            .parse::<usize>()
            .map_err(|_| format!("ASCII table format {:?} has no field width", value))?;

        let decimals = match decimals {
            Some(decimals) => decimals
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("ASCII table format {:?} has an invalid scale", value))?,
            None => 0,
        };

        match code {
            'A' => Ok(AsciiColumnFormat::Character(width)),
            'I' => Ok(AsciiColumnFormat::Integer(width)),
            'F' => Ok(AsciiColumnFormat::Fixed(width, decimals)),
            'E' => Ok(AsciiColumnFormat::Exponential(width, decimals)),
            'D' => Ok(AsciiColumnFormat::Double(width, decimals)),
            _ => Err(From::from(format!(
                "Invalid ASCII table format: {:?}",
                value
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AsciiColumnFormat;
    use crate::bin_table::Value;

    fn format(text: &str) -> AsciiColumnFormat {
        AsciiColumnFormat::try_from(text.to_string()).expect("a valid ASCII table format")
    }

    #[test]
    fn formats_parse_with_and_without_a_scale() {
        assert_eq!(format("A20"), AsciiColumnFormat::Character(20));
        assert_eq!(format("I10"), AsciiColumnFormat::Integer(10));
        assert_eq!(format("F12.5"), AsciiColumnFormat::Fixed(12, 5));
        assert_eq!(format("E15.7"), AsciiColumnFormat::Exponential(15, 7));
        assert_eq!(format("D25.17"), AsciiColumnFormat::Double(25, 17));
    }

    #[test]
    fn binary_table_codes_are_not_ascii_table_formats() {
        // `J` and `X` mean nothing here, and accepting them would let a binary
        // table be read as an ASCII one.
        assert!(AsciiColumnFormat::try_from("1J".to_string()).is_err());
        assert!(AsciiColumnFormat::try_from("16X".to_string()).is_err());
    }

    #[test]
    fn numbers_are_read_out_of_their_fixed_width_fields() {
        assert!(matches!(
            format("I10").parse_into_value(b"      1234"),
            Ok(Value::I64(ref v)) if v == &[1234]
        ));
        assert!(matches!(
            format("F8.2").parse_into_value(b"  -12.50"),
            Ok(Value::F64(ref v)) if v == &[-12.5]
        ));
    }

    #[test]
    fn a_d_exponent_marker_reads_as_a_double() {
        // FITS writes doubles as `1.5D+02` where most languages expect `1.5E+02`.
        assert!(matches!(
            format("D12.4").parse_into_value(b"   1.5D+02"),
            Ok(Value::F64(ref v)) if v == &[150.0]
        ));
    }

    #[test]
    fn a_blank_numeric_field_is_undefined() {
        // The standard lets any ASCII column leave a field entirely blank.
        assert!(matches!(
            format("I10").parse_into_value(b"          "),
            Ok(Value::Null)
        ));
    }

    #[test]
    fn a_character_field_keeps_its_text_and_drops_its_padding() {
        assert!(matches!(
            format("A8").parse_into_value(b"NGC 4565"),
            Ok(Value::String(ref text)) if text == "NGC 4565"
        ));
        assert!(matches!(
            format("A8").parse_into_value(b"M31     "),
            Ok(Value::String(ref text)) if text == "M31"
        ));
    }

    #[test]
    fn a_field_that_is_not_a_number_is_an_error() {
        let error = format("I10")
            .parse_into_value(b"      abcd")
            .expect_err("`abcd` is not an integer");

        assert!(error.to_string().contains("abcd"), "got: {error}");
    }
}
