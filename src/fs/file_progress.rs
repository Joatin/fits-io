#[derive(Debug, Clone, Copy)]
pub struct FileProgress {
    pub total_bytes: u64,
    pub bytes_read: u64,
}
