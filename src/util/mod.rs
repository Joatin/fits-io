#[cfg(feature = "fs")]
mod read_bytes;
mod read_seek;

// Only the filesystem readers stream bytes; the in-memory ones already have
// them all.
#[cfg(all(feature = "tokio", feature = "fs"))]
mod read_bytes_async;

#[cfg(feature = "fs")]
pub(crate) use self::read_bytes::read_bytes;
#[cfg(all(feature = "tokio", feature = "fs"))]
pub(crate) use self::read_bytes_async::read_bytes_async;
pub(crate) use self::read_seek::{ReadSeek, SharedBytes};
