use crate::bin_table::{BinTable, Row};
use crate::hdu::HDU;
use std::error::Error;
use std::fmt;
use std::prelude::rust_2015::Box;

pub trait AsciiTableHDU: HDU + fmt::Debug + Send + Sync {
    fn read_table(&self) -> Result<BinTable, Box<dyn Error + Send + Sync>>;

    #[cfg(feature = "tokio")]
    fn stream_table_rows(
        &self,
    ) -> Result<futures::stream::BoxStream<'_, Row<'_>>, Box<dyn Error + Send + Sync>>;
}
