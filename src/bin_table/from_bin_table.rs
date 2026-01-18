use crate::bin_table::BinTable;
use crate::bin_table::from_bin_table_row::from_bin_table_row;
use alloc::vec::Vec;
use log::info;
use serde::de::DeserializeOwned;

pub fn from_bin_table<T: DeserializeOwned>(bin_table: &BinTable) -> crate::Result<Vec<T>> {
    let mut result = Vec::with_capacity(bin_table.len());
    for (index, row) in bin_table.rows().enumerate() {
        info!("Deserializing row {}...", index + 1);
        result.push(from_bin_table_row(&row)?)
    }
    Ok(result)
}
