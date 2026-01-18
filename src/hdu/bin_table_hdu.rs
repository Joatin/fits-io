use crate::bin_table::{BinTable, Row};
use crate::hdu::HDU;
#[cfg(feature = "serde")]
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt;
use std::prelude::rust_2015::{Box, Vec};

pub trait BinTableHDU: HDU + fmt::Debug + Send + Sync {
    fn table_data_bytes_len(&self) -> u64;

    fn read_table(&self) -> Result<BinTable, Box<dyn Error + Send + Sync>>;

    #[cfg(feature = "serde")]
    fn read_rows<T: DeserializeOwned>(&self) -> Result<Vec<T>, Box<dyn Error + Send + Sync>>;

    #[cfg(feature = "tokio")]
    fn stream_table_rows_raw(
        &self,
    ) -> Result<futures::stream::BoxStream<'_, Row<'_>>, Box<dyn Error + Send + Sync>>;

    #[cfg(feature = "serde")]
    #[cfg(feature = "tokio")]
    fn stream_table_rows<T: DeserializeOwned>(
        &self,
    ) -> Result<futures::stream::BoxStream<'_, T>, Box<dyn Error + Send + Sync>>;
}
