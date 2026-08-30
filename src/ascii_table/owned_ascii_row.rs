use crate::ascii_table::{AsciiFieldDefinition, AsciiRow};
use crate::bin_table::Value;
use std::sync::Arc;

/// An ASCII table row that owns the bytes it decodes from.
///
/// [`AsciiRow`] borrows both its bytes and its column definitions, which ties it
/// to an [`AsciiTable`](crate::ascii_table::AsciiTable) held in memory. A
/// streamed row has no such table behind it, so it keeps its own copy of the row
/// and shares one set of column definitions with the rest of the stream.
#[derive(Debug, Clone)]
pub struct OwnedAsciiRow {
    data: Vec<u8>,
    field_definitions: Arc<Vec<AsciiFieldDefinition>>,
}

impl OwnedAsciiRow {
    pub(crate) fn new(field_definitions: Arc<Vec<AsciiFieldDefinition>>, data: Vec<u8>) -> Self {
        Self {
            data,
            field_definitions,
        }
    }

    /// Borrows this row, for the parts of the API that take an [`AsciiRow`].
    pub fn row(&self) -> AsciiRow<'_> {
        AsciiRow::new(&self.field_definitions, &self.data)
    }

    /// Reads the column named `key`, or `None` if this row has no such column.
    pub fn get(&self, key: &str) -> crate::Result<Option<Value>> {
        self.row().get(key)
    }

    /// Reads the column at `index`, or `None` if this row has no such column.
    pub fn get_at(&self, index: usize) -> crate::Result<Option<Value>> {
        self.row().get_at(index)
    }

    /// The columns this row is made of.
    pub fn field_definitions(&self) -> &[AsciiFieldDefinition] {
        &self.field_definitions
    }
}
