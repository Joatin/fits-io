use crate::bin_table::{FieldDefinition, Row, Value};
use std::sync::Arc;

/// A binary table row that owns the bytes it decodes from.
///
/// [`Row`] borrows both its bytes and its column definitions, which ties it to a
/// [`BinTable`](crate::bin_table::BinTable) held in memory. A streamed row has
/// no such table behind it — not holding one is the point of streaming — so it
/// keeps its own copy of the row and shares one set of column definitions with
/// the rest of the stream.
#[derive(Debug, Clone)]
pub struct OwnedRow {
    data: Vec<u8>,
    heap: Arc<Vec<u8>>,
    field_definitions: Arc<Vec<FieldDefinition>>,
}

impl OwnedRow {
    pub(crate) fn new(
        field_definitions: Arc<Vec<FieldDefinition>>,
        heap: Arc<Vec<u8>>,
        data: Vec<u8>,
    ) -> Self {
        Self {
            data,
            heap,
            field_definitions,
        }
    }

    /// Borrows this row, for the parts of the API that take a [`Row`].
    pub fn row(&self) -> Row<'_> {
        Row::new(&self.field_definitions, &self.data, &self.heap)
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
    pub fn field_definitions(&self) -> &[FieldDefinition] {
        &self.field_definitions
    }
}
