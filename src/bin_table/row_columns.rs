use crate::bin_table::Value;

/// A row of a table, seen as the columns it is made of.
///
/// The two kinds of FITS table store their values quite differently — one as
/// binary fields, the other as fixed-width text — but a row of either is a
/// sequence of named columns that decode to a [`Value`]. That is all the serde
/// deserializer needs, so it works over this rather than over one table kind.
pub(crate) trait RowColumns {
    fn column_count(&self) -> usize;

    /// The column's TTYPEn name, or `None` past the last column.
    fn column_name(&self, index: usize) -> Option<&str>;

    /// How the column's type reads in an error message.
    fn column_description(&self, index: usize) -> Option<String>;

    /// The decoded contents of the column, or `None` past the last column.
    fn value_at(&self, index: usize) -> crate::Result<Option<Value>>;

    /// The column's TDIMn shape, fastest-varying axis first, or empty for a
    /// column that is a plain run of elements.
    fn column_dimensions(&self, _index: usize) -> &[usize] {
        &[]
    }
}
