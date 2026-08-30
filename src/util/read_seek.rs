use std::io::{Read, Seek};
use std::sync::Arc;

pub(crate) trait ReadSeek: Read + Seek + Send + Sync {}
impl<T: Read + Seek + Send + Sync> ReadSeek for T {}

/// A buffer that several readers share, for [`std::io::Cursor`] to read over.
///
/// `Cursor` needs its contents to be `AsRef<[u8]>`, which `Arc<Vec<u8>>` is not,
/// and going through `Vec` instead would copy the whole file to read its header.
pub(crate) struct SharedBytes(pub(crate) Arc<Vec<u8>>);

impl AsRef<[u8]> for SharedBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
