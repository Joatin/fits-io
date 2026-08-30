/// A result whose error is this crate's [`Error`](crate::Error).
pub type Result<T> = std::result::Result<T, crate::error::Error>;
