use crate::ascii_table::AsciiTable;
use crate::bin_table::from_bin_table_row::from_ascii_table_row;
#[cfg(feature = "rayon")]
use rayon::iter::ParallelIterator;
use serde::de::DeserializeOwned;

/// Deserialises every row of an ASCII table into `T`.
///
/// Columns are matched to fields by their TTYPEn names, exactly as they are for
/// a binary table.
#[cfg(not(feature = "rayon"))]
pub fn from_ascii_table<T: DeserializeOwned>(table: &AsciiTable) -> crate::Result<Vec<T>> {
    let mut result = Vec::with_capacity(table.len());

    for row in table.rows() {
        result.push(from_ascii_table_row(&row)?)
    }

    Ok(result)
}

/// Deserialises every row of an ASCII table into `T`.
///
/// Columns are matched to fields by their TTYPEn names, exactly as they are for
/// a binary table.
#[cfg(feature = "rayon")]
pub fn from_ascii_table<T: DeserializeOwned + Send>(table: &AsciiTable) -> crate::Result<Vec<T>> {
    table
        .rows_parallel()
        .map(|row| from_ascii_table_row(&row))
        .collect::<crate::Result<Vec<T>>>()
}
