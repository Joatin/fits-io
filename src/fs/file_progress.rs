/// How far through a file a read has got.
#[derive(Debug, Clone, Copy)]
pub struct FileProgress {
    /// How many bytes there are to read in all.
    pub total_bytes: u64,
    /// How many have been read so far.
    pub bytes_read: u64,
}
