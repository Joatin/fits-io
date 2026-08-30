use crate::bin_table::BinTable;
use crate::bin_table::from_bin_table_row::from_bin_table_row;
#[cfg(feature = "rayon")]
use rayon::iter::ParallelIterator;
use serde::de::DeserializeOwned;

/// Deserialises every row of a binary table into `T`.
///
/// Columns are matched to fields by their TTYPEn names.
#[cfg(not(feature = "rayon"))]
pub fn from_bin_table<T: DeserializeOwned>(bin_table: &BinTable) -> crate::Result<Vec<T>> {
    let mut result = Vec::with_capacity(bin_table.len());

    for row in bin_table.rows() {
        result.push(from_bin_table_row(&row)?)
    }

    Ok(result)
}

/// Deserialises every row of a binary table into `T`.
///
/// Columns are matched to fields by their TTYPEn names.
#[cfg(feature = "rayon")]
pub fn from_bin_table<T: DeserializeOwned + Send>(bin_table: &BinTable) -> crate::Result<Vec<T>> {
    bin_table
        .rows_parallel()
        .map(|row| from_bin_table_row(&row))
        .collect::<crate::Result<Vec<T>>>()
}
