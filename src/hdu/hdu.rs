use crate::header::Header;

pub trait HDU {
    fn header(&self) -> &Header;
    fn header_mut(&mut self) -> &mut Header;

    /// Total size of this HDU in bytes. Including both header and data, and aligned to the Fits
    /// blocks.
    fn byte_size(&self) -> u64 {
        let header = self.header();
        (header.bytes_len() + header.data_block_len()) as u64
    }
}
