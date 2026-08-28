use crate::bin_table::BinTable;
#[cfg(feature = "tokio")]
use crate::bin_table::Row;
use crate::hdu::HDU;
use std::error::Error;
use std::fmt;

pub trait AsciiTableHDU: HDU + fmt::Debug + Send + Sync {
    fn read_table(&self) -> Result<BinTable, Box<dyn Error + Send + Sync>>;

    #[cfg(feature = "tokio")]
    fn stream_table_rows(
        &self,
    ) -> Result<futures::stream::BoxStream<'_, Row<'_>>, Box<dyn Error + Send + Sync>>;
}
