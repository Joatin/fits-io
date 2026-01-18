use crate::bin_table::{BinTable, Row};
use crate::fs::open_fits_file::open_fits_file;
use crate::hdu::{BinTableHDU, HDU};
use crate::header::Header;
use crate::util::read_bytes;
use futures::stream::BoxStream;
#[cfg(feature = "serde")]
use serde::de::DeserializeOwned;
use std::error::Error;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::prelude::rust_2015::{Box, Vec};

#[derive(Debug, Clone)]
pub struct FsBinTableHDU {
    header: Header,
    hdu_offset: u64,
    path: PathBuf,
}

impl FsBinTableHDU {
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

impl HDU for FsBinTableHDU {
    fn header(&self) -> &Header {
        &self.header
    }

    fn header_mut(&mut self) -> &mut Header {
        &mut self.header
    }
}

impl BinTableHDU for FsBinTableHDU {
    fn table_data_bytes_len(&self) -> u64 {
        (self.header.naxis_n(0).unwrap() * self.header.naxis_n(1).unwrap()) as u64
    }

    fn read_table(&self) -> Result<BinTable, Box<dyn Error + Send + Sync>> {
        let mut reader = open_fits_file(&self.path)?;
        reader.seek(SeekFrom::Start(
            self.hdu_offset + self.header.bytes_len() as u64,
        ))?;

        let bytes = read_bytes(&mut reader, self.table_data_bytes_len());

        BinTable::from_u8(&self.header, bytes)
    }

    #[cfg(feature = "serde")]
    fn read_rows<T: DeserializeOwned>(&self) -> Result<Vec<T>, Box<dyn Error + Send + Sync>> {
        let table = self.read_table()?;
        Ok(crate::bin_table::from_bin_table(&table)?)
    }

    #[cfg(feature = "tokio")]
    fn stream_table_rows_raw(
        &self,
    ) -> Result<BoxStream<'_, Row<'_>>, Box<dyn Error + Send + Sync>> {
        let mut reader = open_fits_file(&self.path)?;
        reader.seek(SeekFrom::Start(
            self.hdu_offset + self.header.bytes_len() as u64,
        ))?;
        todo!()
    }

    #[cfg(feature = "serde")]
    #[cfg(feature = "tokio")]
    fn stream_table_rows<T: DeserializeOwned>(
        &self,
    ) -> Result<BoxStream<'_, T>, Box<dyn Error + Send + Sync>> {
        todo!()
    }
}
