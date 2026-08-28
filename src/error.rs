#[cfg(feature = "serde")]
use std::fmt::Display;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

#[cfg(feature = "serde")]
impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self::DeserializationError(msg.to_string())
    }
}
