use crate::util::ReadSeek;
use std::io;
use std::io::Read;

const BUFFER_LEN: usize = 2880 * 64;

/// Reads exactly `bytes_to_read` bytes from `reader`.
///
/// Returns [`io::ErrorKind::UnexpectedEof`] if the reader runs out first: a
/// truncated file must surface as an error rather than a short buffer, because
/// callers size their images and tables from the header.
pub fn read_bytes(reader: &mut Box<dyn ReadSeek>, bytes_to_read: u64) -> io::Result<Vec<u8>> {
    let mut buffer = [0_u8; BUFFER_LEN];
    let mut data = Vec::with_capacity(bytes_to_read as usize);

    let mut bytes_read = 0;
    while bytes_read < bytes_to_read {
        let wanted = (bytes_to_read - bytes_read).min(BUFFER_LEN as u64) as usize;
        let bytes = reader.read(&mut buffer[..wanted])?;

        // A zero-length read means end of file. Without this the loop spins
        // forever, because `bytes_read` stops advancing.
        if bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "Expected {} bytes of data but the file ended after {}",
                    bytes_to_read, bytes_read
                ),
            ));
        }

        bytes_read += bytes as u64;
        data.extend_from_slice(&buffer[..bytes]);
    }

    Ok(data)
}
