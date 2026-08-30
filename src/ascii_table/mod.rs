//! Structs for working with FITS ASCII tables.

mod ascii_column_format;
mod ascii_field_definition;
mod ascii_row;
#[allow(clippy::module_inception)]
mod ascii_table;
#[cfg(feature = "serde")]
mod from_ascii_table;
#[cfg(feature = "tokio")]
mod owned_ascii_row;
#[cfg(feature = "serde")]
mod to_ascii_table;
pub(crate) mod write;

pub use self::ascii_column_format::AsciiColumnFormat;
pub use self::ascii_field_definition::AsciiFieldDefinition;
pub use self::ascii_row::AsciiRow;
pub use self::ascii_table::AsciiTable;
#[cfg(feature = "serde")]
pub use self::from_ascii_table::from_ascii_table;
#[cfg(feature = "tokio")]
pub use self::owned_ascii_row::OwnedAsciiRow;
#[cfg(feature = "serde")]
pub use self::to_ascii_table::to_ascii_table;
#[cfg(feature = "serde")]
pub use crate::bin_table::from_bin_table_row::from_ascii_table_row;
