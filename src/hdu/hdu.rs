use crate::header::Header;

/// What every HDU has: a header.
pub trait HDU {
    /// This HDU's header.
    fn header(&self) -> &Header;
    /// This HDU's header, to be changed.
    fn header_mut(&mut self) -> &mut Header;

    /// Total size of this HDU in bytes. Including both header and data, and aligned to the Fits
    /// blocks.
    fn byte_size(&self) -> u64 {
        let header = self.header();
        (header.bytes_len() + header.data_block_len()) as u64
    }
}
