#[cfg(feature = "serde")]
use std::fmt::Display;
use thiserror::Error;

/// What can go wrong converting between FITS values and your own types.
#[derive(Debug, Error)]
pub enum Error {
    /// A value could not be read as the type that was asked for.
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
