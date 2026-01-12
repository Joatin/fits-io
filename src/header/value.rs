use crate::header::card::Card;
use alloc::string::String;
use std::format;
use std::prelude::rust_2015::ToString;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer {
        value: i64,
        comment: Option<String>,
    },
    Float {
        value: f64,
        comment: Option<String>,
    },
    Logical {
        value: bool,
        comment: Option<String>,
    },
    String {
        value: String,
        comment: Option<String>,
    },
    Undefined,
    Invalid(String),
}

impl Value {
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
            Card::TableNullValueN { value, comment, .. } => Value::String { value, comment },
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
            Card::Continuation { .. } => Value::Undefined,
            Card::Hierarch { .. } => Value::Undefined,
            Card::Space => Value::Undefined,
            Card::Undefined(_) => Value::Undefined,
            Card::TableFormatN { value, comment, .. } => Value::String {
                value: value.into(),
                comment,
            },
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
