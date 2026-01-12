use crate::bin_table::{BinTable, Row};
use crate::hdu::{AsciiTableHDU, HDU};
use crate::header::Header;
use futures::stream::BoxStream;
use std::error::Error;
use std::prelude::rust_2015::Box;

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

    fn stream_table_rows(&self) -> Result<BoxStream<Row>, Box<dyn Error + Send + Sync>> {
        todo!()
    }
}
