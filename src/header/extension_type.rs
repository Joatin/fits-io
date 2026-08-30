use std::error::Error;

/// Which kind of extension an XTENSION card names.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExtensionType {
    /// `IMAGE`: an array, like the primary HDU's.
    Image,
    /// `BINTABLE`: a table of binary fields.
    BinTable,
    /// `TABLE`: a table of fixed-width text.
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
