use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use core::error::Error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExtensionType {
    Image,
    BinTable,
    AsciiTable,
}

impl From<ExtensionType> for String {
    fn from(value: ExtensionType) -> Self {
        match value {
            ExtensionType::Image => "IMAGE".to_string(),
            ExtensionType::BinTable => "BINTABLE".to_string(),
            ExtensionType::AsciiTable => "TABLE".to_string(),
        }
    }
}

impl TryFrom<String> for ExtensionType {
    type Error = Box<dyn Error + Send + Sync>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_uppercase().as_str() {
            "IMAGE" => Ok(ExtensionType::Image),
            "TABLE" => Ok(ExtensionType::AsciiTable),
            "BINTABLE" => Ok(ExtensionType::BinTable),
            _ => Err(format!("Unknown extension type: {}", value).into()),
        }
    }
}
