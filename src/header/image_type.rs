use alloc::string::String;
use core::fmt::{Display, Formatter};
use std::prelude::rust_2015::ToString;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ImageType {
    Light,
    Bias,
    Dark,
    Flat,
    MasterBias,
    MasterDark,
    MasterFlat,
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
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
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
