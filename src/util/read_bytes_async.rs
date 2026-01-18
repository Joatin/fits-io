use crate::util::ReadSeek;
use alloc::boxed::Box;
use futures::stream::BoxStream;
use futures::{StreamExt, stream};
use tokio::sync::mpsc::channel;
use tokio_stream::wrappers::ReceiverStream;

pub fn read_bytes_async(
    mut reader: Box<dyn ReadSeek>,
    bytes_to_read: u64,
) -> BoxStream<'static, u8> {
    let (sender, receiver) = channel(100);
    tokio::task::spawn_blocking(move || {
        let mut buffer = [0_u8; 2880 * 64];

        let mut bytes_read = 0;
        while bytes_read < bytes_to_read {
            let bytes =
                reader.read(&mut buffer[..(bytes_to_read - bytes_read).min(2880 * 64) as usize])?;
            bytes_read += bytes as u64;
            sender.blocking_send(buffer[..bytes].to_vec())?;
        }
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    let stream = ReceiverStream::new(receiver);

    stream.flat_map(|data| stream::iter(data)).boxed()
}
