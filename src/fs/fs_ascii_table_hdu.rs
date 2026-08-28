use crate::bin_table::BinTable;
#[cfg(feature = "tokio")]
use crate::bin_table::Row;
use crate::hdu::{AsciiTableHDU, HDU};
use crate::header::Header;
#[cfg(feature = "tokio")]
use futures::stream::BoxStream;
use std::error::Error;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FsAsciiTableHDU {
    header: Header,
    // Where the table data lives; read once `read_table` is implemented.
    #[allow(dead_code)]
    hdu_offset: u64,
    #[allow(dead_code)]
    path: PathBuf,
}

impl FsAsciiTableHDU {
    pub fn new(
        path: &Path,
        header: Header,
        hdu_offset: u64,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            header,
            hdu_offset,
            path: path.to_path_buf(),
        })
    }
}

impl HDU for FsAsciiTableHDU {
    fn header(&self) -> &Header {
        &self.header
    }

    fn header_mut(&mut self) -> &mut Header {
        &mut self.header
    }
}

impl AsciiTableHDU for FsAsciiTableHDU {
    fn read_table(&self) -> Result<BinTable, Box<dyn Error + Send + Sync>> {
        Err("Reading ASCII tables is not implemented yet".into())
    }

    #[cfg(feature = "tokio")]
    fn stream_table_rows(&self) -> Result<BoxStream<'_, Row<'_>>, Box<dyn Error + Send + Sync>> {
        Err("Reading ASCII tables is not implemented yet".into())
    }
}
