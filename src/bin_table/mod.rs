//! Structs for working with FITS Bin Tables

mod row;
#[cfg(feature = "serde")]
pub(crate) mod row_columns;
mod value;

mod bin_table;
#[cfg(feature = "serde")]
mod encode;
mod field_definition;
#[cfg(feature = "tokio")]
mod owned_row;
pub(crate) mod write;

#[cfg(feature = "serde")]
mod from_bin_table;

#[cfg(feature = "serde")]
pub(crate) mod from_bin_table_row;

#[cfg(feature = "serde")]
pub(crate) mod to_bin_table;

pub use self::bin_table::BinTable;
pub use self::field_definition::FieldDefinition;
#[cfg(feature = "tokio")]
pub use self::owned_row::OwnedRow;
pub use self::row::Row;
pub use self::value::Value;

#[cfg(feature = "serde")]
pub use self::from_bin_table::from_bin_table;
#[cfg(feature = "serde")]
pub use self::from_bin_table_row::from_bin_table_row;
#[cfg(feature = "serde")]
pub use self::to_bin_table::to_bin_table;
