use crate::bin_table::BinTable;
#[cfg(feature = "tokio")]
use crate::bin_table::OwnedRow;
use crate::hdu::HDU;
#[cfg(feature = "serde")]
use serde::Serialize;
#[cfg(feature = "serde")]
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt;

/// An HDU whose data section is a binary table.
pub trait BinTableHDU: HDU + fmt::Debug + Send + Sync {
    /// How many bytes the rows occupy, not counting the heap.
    fn table_data_bytes_len(&self) -> u64;

    /// Reads the whole table into memory.
    fn read_table(&self) -> Result<BinTable, Box<dyn Error + Send + Sync>>;

    /// Reads every row into `T`, matching columns to fields by their TTYPEn names.
    #[cfg(feature = "serde")]
    fn read_rows<T: DeserializeOwned + Send + Sync>(
        &self,
    ) -> Result<Vec<T>, Box<dyn Error + Send + Sync>>;

    /// Replaces this HDU's table.
    ///
    /// The header is rewritten to describe the new table — its row width and
    /// count, and a TFORMn and TTYPEn for every column — because those cards are
    /// the only thing that says how to read the bytes back.
    fn set_table(&mut self, table: &BinTable) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Serialises `rows` and stores the result, as [`to_bin_table`] followed by
    /// [`set_table`].
    ///
    /// [`to_bin_table`]: crate::bin_table::to_bin_table
    /// [`set_table`]: BinTableHDU::set_table
    #[cfg(feature = "serde")]
    fn set_rows<T: Serialize>(&mut self, rows: &T) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.set_table(&crate::bin_table::to_bin_table(rows)?)
    }

    /// Streams the table one row at a time, without holding the whole thing in
    /// memory.
    ///
    /// Rows arrive in the order they are stored. A row is decoded lazily, so a
    /// column that cannot be read reports its error from
    /// [`OwnedRow::get`] rather than from the stream.
    ///
    /// A truncated file ends the stream early rather than failing it; compare
    /// the number of rows received against NAXIS2 to tell the two apart.
    #[cfg(feature = "tokio")]
    fn stream_table_rows_raw(
        &self,
    ) -> Result<futures::stream::BoxStream<'_, OwnedRow>, Box<dyn Error + Send + Sync>>;

    /// Streams the table as deserialised rows of `T`.
    ///
    /// Unlike [`stream_table_rows_raw`] each row is decoded as it arrives, so a
    /// row that does not fit `T` yields an `Err` in its place and the stream
    /// carries on.
    ///
    /// [`stream_table_rows_raw`]: BinTableHDU::stream_table_rows_raw
    #[cfg(feature = "serde")]
    #[cfg(feature = "tokio")]
    fn stream_table_rows<T: DeserializeOwned + Send + Sync>(
        &self,
    ) -> Result<futures::stream::BoxStream<'_, crate::Result<T>>, Box<dyn Error + Send + Sync>>;
}
