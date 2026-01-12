use crate::util::ReadSeek;
use alloc::boxed::Box;
use alloc::vec::Vec;
use std::io::Read;

pub fn read_bytes(reader: &mut Box<dyn ReadSeek>, bytes_to_read: u64) -> Vec<u8> {
    let mut buffer = [0_u8; 2880 * 64];
    let mut data = Vec::with_capacity(bytes_to_read as usize);

    let mut bytes_read = 0;
    while bytes_read < bytes_to_read {
        let bytes = reader
            .read(&mut buffer[..(bytes_to_read - bytes_read).min(2880 * 64) as usize])
            .unwrap();
        bytes_read += bytes as u64;
        data.extend_from_slice(&buffer[..bytes]);
    }
    data
}
