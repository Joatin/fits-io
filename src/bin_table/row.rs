use crate::bin_table::FieldDefinition;
#[cfg(feature = "serde")]
use crate::bin_table::row_columns::RowColumns;
use crate::bin_table::value::Value;

/// A data row
#[derive(Debug, Clone)]
pub struct Row<'a> {
    data: &'a [u8],
    /// The table's heap, where variable length array columns keep their values.
    /// Empty for a table that has none.
    heap: &'a [u8],
    field_definitions: &'a Vec<FieldDefinition>,
}

impl<'a> Row<'a> {
    pub(crate) fn new(
        field_definitions: &'a Vec<FieldDefinition>,
        data: &'a [u8],
        heap: &'a [u8],
    ) -> Self {
        Self {
            data,
            heap,
            field_definitions,
        }
    }

    /// The columns this row is made of.
    pub fn field_definitions(&self) -> &'a [FieldDefinition] {
        self.field_definitions
    }

    /// Reads the column named `key`, or `None` if this row has no such column.
    pub fn get(&self, key: &str) -> crate::Result<Option<Value>> {
        match self.field_definitions.iter().position(|i| i.name == key) {
            Some(index) => self.get_at(index),
            None => Ok(None),
        }
    }

    /// Reads the column at `index`, or `None` if this row has no such column.
    ///
    /// Unlike [`Row::get`] this works for columns that carry no TTYPEn name, and
    /// distinguishes columns that share one.
    pub fn get_at(&self, index: usize) -> crate::Result<Option<Value>> {
        let Some(field) = self.field_definitions.get(index) else {
            return Ok(None);
        };

        Ok(Some(field.decode(self.data, self.heap)?))
    }
}

#[cfg(feature = "serde")]
impl RowColumns for Row<'_> {
    fn column_count(&self) -> usize {
        self.field_definitions.len()
    }

    fn column_name(&self, index: usize) -> Option<&str> {
        self.field_definitions
            .get(index)
            .map(|field| field.name.as_str())
    }

    fn column_description(&self, index: usize) -> Option<String> {
        self.field_definitions
            .get(index)
            .map(|field| format!("{:?}", field.format))
    }

    fn value_at(&self, index: usize) -> crate::Result<Option<Value>> {
        self.get_at(index)
    }

    fn column_dimensions(&self, index: usize) -> &[usize] {
        self.field_definitions
            .get(index)
            .map(|field| field.dimensions.as_slice())
            .unwrap_or_default()
    }
}
