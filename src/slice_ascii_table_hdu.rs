use crate::bin_table::BinTable;
#[cfg(feature = "tokio")]
use crate::bin_table::Row;
use crate::hdu::{AsciiTableHDU, HDU};
use crate::header::Header;
#[cfg(feature = "tokio")]
use futures::stream::BoxStream;
use std::error::Error;

#[derive(Debug, Clone)]
pub struct SliceAsciiTableHDU {}

impl HDU for SliceAsciiTableHDU {
    fn header(&self) -> &Header {
        todo!()
    }

    fn header_mut(&mut self) -> &mut Header {
        todo!()
    }
}

impl AsciiTableHDU for SliceAsciiTableHDU {
    fn read_table(&self) -> Result<BinTable, Box<dyn Error + Send + Sync>> {
        todo!()
    }

    #[cfg(feature = "tokio")]
    fn stream_table_rows(&self) -> Result<BoxStream<'_, Row<'_>>, Box<dyn Error + Send + Sync>> {
        todo!()
    }
}
