//! Structs for working with FITS Bin Tables

mod row;
mod value;

mod bin_table;

#[cfg(feature = "serde")]
mod from_bin_table;

#[cfg(feature = "serde")]
mod from_bin_table_row;

#[cfg(feature = "serde")]
mod to_bin_table;

pub use self::bin_table::BinTable;
pub use self::row::Row;
pub use self::value::Value;

#[cfg(feature = "serde")]
pub use self::from_bin_table::from_bin_table;
#[cfg(feature = "serde")]
pub use self::to_bin_table::to_bin_table;
