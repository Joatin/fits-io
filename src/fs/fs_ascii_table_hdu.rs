use crate::bin_table::{BinTable, Row};
use crate::hdu::{AsciiTableHDU, HDU};
use crate::header::Header;
use futures::stream::BoxStream;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::prelude::rust_2015::Box;

#[derive(Debug, Clone)]
pub struct FsAsciiTableHDU {
    header: Header,
    hdu_offset: u64,
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
        todo!()
    }

    fn stream_table_rows(&self) -> Result<BoxStream<'_, Row<'_>>, Box<dyn Error + Send + Sync>> {
        todo!()
    }
}
