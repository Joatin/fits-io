use crate::bin_table::BinTable;
#[cfg(feature = "tokio")]
use crate::bin_table::Row;
use crate::hdu::{BinTableHDU, HDU};
use crate::header::Header;
#[cfg(feature = "tokio")]
use futures::stream::BoxStream;
#[cfg(feature = "serde")]
use serde::de::DeserializeOwned;
use std::error::Error;

#[derive(Debug, Clone)]
pub struct SliceBinTableHDU {}

impl HDU for SliceBinTableHDU {
    fn header(&self) -> &Header {
        todo!()
    }

    fn header_mut(&mut self) -> &mut Header {
        todo!()
    }
}

impl BinTableHDU for SliceBinTableHDU {
    fn table_data_bytes_len(&self) -> u64 {
        todo!()
    }

    fn read_table(&self) -> Result<BinTable, Box<dyn Error + Send + Sync>> {
        todo!()
    }

    #[cfg(feature = "serde")]
    fn read_rows<T: DeserializeOwned + Send + Sync>(
        &self,
    ) -> Result<Vec<T>, Box<dyn Error + Send + Sync>> {
        todo!()
    }

    #[cfg(feature = "tokio")]
    fn stream_table_rows_raw(
        &self,
    ) -> Result<BoxStream<'_, Row<'_>>, Box<dyn Error + Send + Sync>> {
        todo!()
    }

    #[cfg(feature = "serde")]
    #[cfg(feature = "tokio")]
    fn stream_table_rows<T: DeserializeOwned + Send + Sync>(
        &self,
    ) -> Result<BoxStream<'_, T>, Box<dyn Error + Send + Sync>> {
        todo!()
    }
}
