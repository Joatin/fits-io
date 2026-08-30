use std::fmt::{Display, Formatter};

/// What an exposure was for, from the IMAGETYP card.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ImageType {
    /// A frame of the sky itself.
    Light,
    /// A zero-length frame, to measure the sensor's readout offset.
    Bias,
    /// A frame taken with the shutter closed, to measure sensor noise.
    Dark,
    /// An evenly lit frame, to measure the optics' response across the field.
    Flat,
    /// A bias frame combined from many.
    MasterBias,
    /// A dark frame combined from many.
    MasterDark,
    /// A flat frame combined from many.
    MasterFlat,
    /// Anything else the card said.
    Unknown(String),
}

impl From<&String> for ImageType {
    fn from(value: &String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<String> for ImageType {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<&str> for ImageType {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "light" => Self::Light,
            "bias" => Self::Bias,
            "dark" => Self::Dark,
            "flat" => Self::Flat,
            "masterbias" => Self::MasterBias,
            "masterdark" => Self::MasterDark,
            "masterflat" => Self::MasterFlat,
            _ => Self::Unknown(value.to_string()),
        }
    }
}

impl Display for ImageType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageType::Light => write!(f, "Light"),
            ImageType::Bias => write!(f, "Bias"),
            ImageType::Dark => write!(f, "Dark"),
            ImageType::Flat => write!(f, "Flat"),
            ImageType::MasterBias => write!(f, "MasterBias"),
            ImageType::MasterDark => write!(f, "MasterDark"),
            ImageType::MasterFlat => write!(f, "MasterFlat"),
            ImageType::Unknown(v) => write!(f, "{}", v),
        }
    }
}
