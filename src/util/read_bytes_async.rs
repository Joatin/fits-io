use crate::util::ReadSeek;
use futures::StreamExt;
use futures::stream::BoxStream;
use log::warn;
use tokio::sync::mpsc::channel;
use tokio_stream::wrappers::ReceiverStream;

const BUFFER_LEN: usize = 2880 * 64;
const BLOCKS_IN_FLIGHT: usize = 100;

/// Streams `bytes_to_read` bytes from `reader` as blocks, reading on a blocking
/// worker so the runtime is never stalled.
///
/// Blocks are whatever the underlying reader hands back, so consumers must not
/// assume any particular block size or alignment.
///
/// The stream ends early if the file is truncated or the read fails; both are
/// logged at warning level. Callers that need to tell a complete read from a
/// short one should compare the number of bytes they received against the length
/// they asked for.
pub fn read_bytes_async(
    mut reader: Box<dyn ReadSeek>,
    bytes_to_read: u64,
) -> BoxStream<'static, Vec<u8>> {
    let (sender, receiver) = channel(BLOCKS_IN_FLIGHT);

    tokio::task::spawn_blocking(move || {
        let mut buffer = [0_u8; BUFFER_LEN];
        let mut bytes_read = 0;

        while bytes_read < bytes_to_read {
            let wanted = (bytes_to_read - bytes_read).min(BUFFER_LEN as u64) as usize;

            let bytes = match reader.read(&mut buffer[..wanted]) {
                // A zero-length read means end of file. Without this the loop
                // spins forever, because `bytes_read` stops advancing.
                Ok(0) => {
                    warn!(
                        "Expected {} bytes of data but the file ended after {}",
                        bytes_to_read, bytes_read
                    );
                    return;
                }
                Ok(bytes) => bytes,
                Err(error) => {
                    warn!(
                        "Failed to read data after {} of {} bytes: {}",
                        bytes_read, bytes_to_read, error
                    );
                    return;
                }
            };

            bytes_read += bytes as u64;

            // The receiver was dropped, so the consumer has stopped listening.
            if sender.blocking_send(buffer[..bytes].to_vec()).is_err() {
                return;
            }
        }
    });

    ReceiverStream::new(receiver).boxed()
}
