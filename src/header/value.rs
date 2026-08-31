use crate::header::card::Card;
use crate::header::table_null_value::TableNullValue;

/// The value a header card carries, with the comment written beside it.
///
/// FITS gives a card one of four kinds of value — an integer, a floating point
/// number, a logical `T`/`F`, or a quoted string — or no value at all. Build one
/// from the Rust type it corresponds to and hand it to [`Header::set_card`]:
///
/// ```
/// # use fits_io::header::{Header, Value};
/// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// let mut header = Header::default();
/// header.set_card("FILTER", "Halpha")?;
/// header.set_card("GAIN", Value::from(1.5).with_comment("e-/ADU"))?;
/// # Ok(())
/// # }
/// ```
///
/// [`Header::set_card`]: crate::header::Header::set_card
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A whole number, written right-justified against column 30.
    Integer {
        /// The number itself.
        value: i64,
        /// The comment after the value, if the card carried one.
        comment: Option<String>,
    },
    /// A floating point number, written right-justified against column 30.
    Float {
        /// The number itself.
        value: f64,
        /// The comment after the value, if the card carried one.
        comment: Option<String>,
    },
    /// A logical, written as the single character `T` or `F`.
    Logical {
        /// The truth of it.
        value: bool,
        /// The comment after the value, if the card carried one.
        comment: Option<String>,
    },
    /// Text, written in single quotes with any quote inside it doubled.
    String {
        /// The text, unquoted and with its doubled quotes read back as one.
        value: String,
        /// The comment after the value, if the card carried one.
        comment: Option<String>,
    },
    /// A keyword written with an empty value field, which the standard allows.
    Undefined,
    /// A value field this crate could not read, kept as the text it held so
    /// that nothing is lost.
    Invalid(String),
}

impl Value {
    /// The comment beside this value, or an empty string where there is none.
    pub fn comment_to_string(&self) -> String {
        let comment = match self {
            Value::Integer { comment, .. } => comment,
            Value::Float { comment, .. } => comment,
            Value::Logical { comment, .. } => comment,
            Value::String { comment, .. } => comment,
            Value::Undefined => &None,
            Value::Invalid(_) => &None,
        };

        if let Some(comment) = comment {
            comment.to_string()
        } else {
            "".to_string()
        }
    }

    /// This value as the text a card writes it as, without its quotes.
    ///
    /// An undefined or unreadable value has no text, and comes back empty.
    pub fn value_to_string(&self) -> String {
        match self {
            Value::Integer { value, .. } => {
                format!("{}", value)
            }
            Value::Float { value, .. } => {
                format!("{}", value)
            }
            Value::Logical { value, .. } => match value {
                true => "T".to_string(),
                false => "F".to_string(),
            },
            Value::String { value, .. } => value.to_string(),
            Value::Undefined => "".to_string(),
            Value::Invalid(_) => "".to_string(),
        }
    }
}

impl From<&Card> for Value {
    fn from(card: &Card) -> Self {
        card.clone().into()
    }
}

impl From<Card> for Value {
    fn from(value: Card) -> Self {
        match value {
            Card::Author { value, comment } => Value::String { value, comment },
            Card::Bitpix { value, comment } => Value::Integer {
                value: value.into(),
                comment,
            },
            Card::Blank { value, comment } => Value::Integer { value, comment },
            Card::Blocked { value, comment } => Value::Logical { value, comment },
            Card::BScale { value, comment } => Value::Float { value, comment },
            Card::BUnit { value, comment } => Value::String { value, comment },
            Card::BZero { value, comment } => Value::Float { value, comment },
            Card::CoordinateDeltaN { value, comment, .. } => Value::Float { value, comment },
            Card::CoordinateRotationN { value, comment, .. } => Value::Float { value, comment },
            Card::CoordinateReferencePixelN { value, comment, .. } => {
                Value::Float { value, comment }
            }
            Card::CoordinateValueAtPixelN { value, comment, .. } => Value::Float { value, comment },
            Card::CoordinateAxisNameN { value, comment, .. } => Value::String { value, comment },
            Card::Comment(_) => Value::Undefined,
            Card::DataMax { value, comment } => Value::Float { value, comment },
            Card::DataMin { value, comment } => Value::Float { value, comment },
            Card::Date { value, comment } => Value::String {
                value: value.to_rfc3339(),
                comment,
            },
            Card::DateObserved { value, comment } => Value::String {
                value: value.to_rfc3339(),
                comment,
            },
            Card::End => Value::Undefined,
            Card::Epoch { value, comment } => Value::Float { value, comment },
            Card::Equinox { value, comment } => Value::Float { value, comment },
            Card::Extend { value, comment } => Value::Logical { value, comment },
            Card::ExtensionLevel { value, comment } => Value::Integer { value, comment },
            Card::ExtensionName { value, comment } => Value::String { value, comment },
            Card::ExtensionVersion { value, comment } => Value::Integer { value, comment },
            Card::GroupCount { value, comment } => Value::Integer { value, comment },
            Card::Groups { value, comment } => Value::Logical { value, comment },
            Card::History(_) => Value::Undefined,
            Card::Instrument { value, comment } => Value::String { value, comment },
            Card::NAxis { value, comment } => Value::Integer { value, comment },
            Card::NAxisN { value, comment, .. } => Value::Integer { value, comment },
            Card::Object { value, comment } => Value::String { value, comment },
            Card::Observer { value, comment } => Value::String { value, comment },
            Card::Origin { value, comment } => Value::String { value, comment },
            Card::ParameterCount { value, comment } => Value::Integer { value, comment },
            Card::ParameterScalingFactorN { value, comment, .. } => Value::Float { value, comment },
            Card::ParameterTypeN { value, comment, .. } => Value::String { value, comment },
            Card::ParameterScalingZeroPointN { value, comment, .. } => {
                Value::Float { value, comment }
            }
            Card::Reference { value, comment } => Value::String { value, comment },
            Card::Simple { value, comment } => Value::Logical { value, comment },
            Card::TableColumnN { value, comment, .. } => Value::Integer { value, comment },
            Card::TableDimensionsN { value, comment, .. } => Value::String { value, comment },
            Card::TableDisplayFormatN { value, comment, .. } => Value::String { value, comment },
            Card::Telescope { value, comment } => Value::String { value, comment },
            Card::TableFields { value, comment } => Value::Integer { value, comment },
            Card::TableHeap { value, comment } => Value::Integer { value, comment },
            Card::TableNullValueN { value, comment, .. } => match value {
                TableNullValue::Integer(value) => Value::Integer { value, comment },
                TableNullValue::Text(value) => Value::String { value, comment },
            },
            Card::TableScalingFactorN { value, comment, .. } => Value::Float { value, comment },
            Card::TableTypeN { value, comment, .. } => Value::String { value, comment },
            Card::TableUnitN { value, comment, .. } => Value::String { value, comment },
            Card::TableScalingZeroPointN { value, comment, .. } => Value::Float { value, comment },
            Card::Xtension { value, comment } => Value::String {
                value: value.into(),
                comment,
            },
            Card::FocalLength { value, comment } => Value::Float { value, comment },
            Card::ExposureTime { value, comment } => Value::Float {
                value: value.as_secs_f64(),
                comment,
            },
            Card::CCDTemperature { value, comment } => Value::Float { value, comment },
            Card::BayerPattern { value, comment } => Value::String {
                value: value.into(),
                comment,
            },
            Card::Value { value, .. } => value,
            // A continuation carries a piece of the string its own card began.
            Card::Continuation { string, comment } => match string {
                Some(value) => Value::String { value, comment },
                None => Value::Undefined,
            },
            Card::Hierarch { value, .. } => value,
            Card::Space => Value::Undefined,
            Card::Undefined(_) => Value::Undefined,
            Card::TableFormatN { value, comment, .. } => Value::String { value, comment },
            Card::Creator { value, comment } => Value::String { value, comment },
            Card::SubframeXPositionInBinnedPixels { value, comment } => {
                Value::Integer { value, comment }
            }
            Card::SubframeYPositionInBinnedPixels { value, comment } => {
                Value::Integer { value, comment }
            }
            Card::BinnedPixelsX { value, comment } => Value::Integer { value, comment },
            Card::BinnedPixelsY { value, comment } => Value::Integer { value, comment },
            Card::CCDBinnedPixelsX { value, comment } => Value::Integer { value, comment },
            Card::CCDBinnedPixelsY { value, comment } => Value::Integer { value, comment },
            Card::PixelSizeXWithBinningInMicrons { value, comment } => {
                Value::Float { value, comment }
            }
            Card::PixelSizeYWithBinningInMicrons { value, comment } => {
                Value::Float { value, comment }
            }
            Card::ImageType { value, comment } => Value::String {
                value: value.to_string(),
                comment,
            },
            Card::Exposure { value, comment } => Value::Float {
                value: value.as_secs_f64(),
                comment,
            },
            Card::Ra { value, comment } => Value::Float { value, comment },
            Card::Dec { value, comment } => Value::Float { value, comment },
            Card::GuideCam { value, comment } => Value::String { value, comment },
            Card::FocusPosition { value, comment } => Value::Integer { value, comment },
            Card::SiteLongitude { value, comment } => Value::Float { value, comment },
            Card::SiteLatitude { value, comment } => Value::Float { value, comment },
            Card::ImageWidth { value, comment } => Value::Integer { value, comment },
            Card::ImageHeight { value, comment } => Value::Integer { value, comment },
        }
    }
}

/// The comment on a value, replaced rather than merged.
impl Value {
    /// This value carrying `comment` beside it.
    ///
    /// A comment on an [`Value::Undefined`] or [`Value::Invalid`] value has
    /// nowhere to live and is dropped: neither of them is written with a value
    /// field for a comment to follow.
    #[must_use]
    pub fn with_comment(self, comment: impl Into<String>) -> Self {
        let comment = Some(comment.into());
        match self {
            Value::Integer { value, .. } => Value::Integer { value, comment },
            Value::Float { value, .. } => Value::Float { value, comment },
            Value::Logical { value, .. } => Value::Logical { value, comment },
            Value::String { value, .. } => Value::String { value, comment },
            other => other,
        }
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Integer {
            value,
            comment: None,
        }
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::from(i64::from(value))
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Value::from(i64::from(value))
    }
}

impl From<usize> for Value {
    fn from(value: usize) -> Self {
        // A count that will not fit in a FITS integer is saturated rather than
        // wrapped, which would write a negative length.
        Value::from(i64::try_from(value).unwrap_or(i64::MAX))
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Float {
            value,
            comment: None,
        }
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::from(f64::from(value))
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Logical {
            value,
            comment: None,
        }
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String {
            value,
            comment: None,
        }
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::from(value.to_string())
    }
}

impl From<&String> for Value {
    fn from(value: &String) -> Self {
        Value::from(value.clone())
    }
}

/// An absent value is an undefined one: the keyword is written with an empty
/// value field, which is how FITS spells "this card has no value".
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => value.into(),
            None => Value::Undefined,
        }
    }
}
