use crate::util::ReadSeek;
use alloc::boxed::Box;
use std::fs::OpenOptions;
use std::io;
use std::io::Read;
use std::path::Path;

pub fn open_fits_file(path: &Path) -> Result<Box<dyn ReadSeek>, io::Error> {
    let file = OpenOptions::new().read(true).open(&path)?;

    #[cfg(feature = "gzip")]
    if path.ends_with(".gz") {
        let mut decoder = flate2::read::GzDecoder::new(file);

        let mut data = alloc::vec![];
        decoder.read_to_end(&mut data)?;

        Ok(Box::new(std::io::Cursor::new(data)))
    } else {
        Ok(Box::new(file))
    }

    #[cfg(not(feature = "gzip"))]
    Ok(Box::new(file))
}
