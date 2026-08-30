use crate::ascii_table::AsciiFieldDefinition;
use crate::bin_table::Value;
#[cfg(feature = "serde")]
use crate::bin_table::row_columns::RowColumns;

/// One row of an ASCII table.
#[derive(Debug, Clone)]
pub struct AsciiRow<'a> {
    data: &'a [u8],
    field_definitions: &'a Vec<AsciiFieldDefinition>,
}

impl<'a> AsciiRow<'a> {
    pub(crate) fn new(field_definitions: &'a Vec<AsciiFieldDefinition>, data: &'a [u8]) -> Self {
        Self {
            data,
            field_definitions,
        }
    }

    /// The columns this row is made of.
    pub fn field_definitions(&self) -> &'a [AsciiFieldDefinition] {
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
    pub fn get_at(&self, index: usize) -> crate::Result<Option<Value>> {
        let Some(field) = self.field_definitions.get(index) else {
            return Ok(None);
        };

        Ok(Some(field.decode(self.data)?))
    }
}

#[cfg(feature = "serde")]
impl RowColumns for AsciiRow<'_> {
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
}
