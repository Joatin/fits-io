use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use core::error::Error;

/// The camera bayer pattern
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BayerPattern {
    RGGB,
    BGGR,
    GRBG,
    GBRG,
}

impl From<BayerPattern> for String {
    fn from(pattern: BayerPattern) -> Self {
        match pattern {
            BayerPattern::RGGB => "RGGB".to_string(),
            BayerPattern::BGGR => "BGGR".to_string(),
            BayerPattern::GRBG => "GRBG".to_string(),
            BayerPattern::GBRG => "GBRG".to_string(),
        }
    }
}

impl TryFrom<String> for BayerPattern {
    type Error = Box<dyn Error + Send + Sync>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "rggb" => Ok(BayerPattern::RGGB),
            "bggr" => Ok(BayerPattern::BGGR),
            "grbg" => Ok(BayerPattern::GRBG),
            "gbrg" => Ok(BayerPattern::GBRG),
            _ => Err(From::from(format!("Invalid BAYERPAT value: {}", value))),
        }
    }
}
