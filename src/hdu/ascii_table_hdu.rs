use crate::ascii_table::AsciiTable;
#[cfg(feature = "tokio")]
use crate::ascii_table::OwnedAsciiRow;
use crate::hdu::HDU;
#[cfg(feature = "serde")]
use serde::Serialize;
#[cfg(feature = "serde")]
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt;

/// An HDU whose data section is a table of fixed-width text.
pub trait AsciiTableHDU: HDU + fmt::Debug + Send + Sync {
    /// Reads the whole table into memory.
    fn read_table(&self) -> Result<AsciiTable, Box<dyn Error + Send + Sync>>;

    /// Reads every row into `T`, matching columns to fields by their TTYPEn
    /// names.
    #[cfg(feature = "serde")]
    fn read_rows<T: DeserializeOwned + Send + Sync>(
        &self,
    ) -> Result<Vec<T>, Box<dyn Error + Send + Sync>> {
        Ok(crate::ascii_table::from_ascii_table(&self.read_table()?)?)
    }

    /// Serialises `rows` and stores the result, as [`to_ascii_table`] followed
    /// by [`set_table`].
    ///
    /// [`to_ascii_table`]: crate::ascii_table::to_ascii_table
    /// [`set_table`]: AsciiTableHDU::set_table
    #[cfg(feature = "serde")]
    fn set_rows<T: Serialize>(&mut self, rows: &T) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.set_table(&crate::ascii_table::to_ascii_table(rows)?)
    }

    /// Replaces this HDU's table.
    ///
    /// The header is rewritten to describe the new table, including the TBCOLn
    /// cards that say where each column starts — nothing else does.
    fn set_table(&mut self, table: &AsciiTable) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Streams the table one row at a time, without holding the whole thing in
    /// memory.
    ///
    /// A truncated file ends the stream early rather than failing it; compare
    /// the number of rows received against NAXIS2 to tell the two apart.
    #[cfg(feature = "tokio")]
    fn stream_table_rows_raw(
        &self,
    ) -> Result<futures::stream::BoxStream<'_, OwnedAsciiRow>, Box<dyn Error + Send + Sync>>;

    /// Streams the table as deserialised rows of `T`.
    ///
    /// Unlike [`stream_table_rows_raw`] each row is decoded as it arrives, so a
    /// row that does not fit `T` yields an `Err` in its place and the stream
    /// carries on.
    ///
    /// [`stream_table_rows_raw`]: AsciiTableHDU::stream_table_rows_raw
    #[cfg(feature = "serde")]
    #[cfg(feature = "tokio")]
    fn stream_table_rows<T: DeserializeOwned + Send + Sync>(
        &self,
    ) -> Result<futures::stream::BoxStream<'_, crate::Result<T>>, Box<dyn Error + Send + Sync>>
    {
        use futures::StreamExt;

        Ok(self
            .stream_table_rows_raw()?
            .map(|row| crate::ascii_table::from_ascii_table_row(&row.row()))
            .boxed())
    }
}
