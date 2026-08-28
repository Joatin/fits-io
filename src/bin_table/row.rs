use crate::bin_table::FieldDefinition;
use crate::bin_table::value::Value;

/// A data row
#[derive(Debug, Clone)]
pub struct Row<'a> {
    data: &'a [u8],
    pub field_definitions: &'a Vec<FieldDefinition>,
}

impl<'a> Row<'a> {
    pub(crate) fn new(field_definitions: &'a Vec<FieldDefinition>, data: &'a [u8]) -> Self {
        Self {
            data,
            field_definitions,
        }
    }

    /// Reads the column named `key`, or `None` if this row has no such column.
    pub fn get(&self, key: &str) -> crate::Result<Option<Value>> {
        match self.field_definitions.iter().position(|i| i.2.eq(key)) {
            Some(index) => self.get_at(index),
            None => Ok(None),
        }
    }

    /// Reads the column at `index`, or `None` if this row has no such column.
    ///
    /// Unlike [`Row::get`] this works for columns that carry no TTYPEn name, and
    /// distinguishes columns that share one.
    pub fn get_at(&self, index: usize) -> crate::Result<Option<Value>> {
        let Some((format, offset, _)) = self.field_definitions.get(index) else {
            return Ok(None);
        };

        let data = self.data.get(*offset..).ok_or_else(|| {
            crate::Error::DeserializationError(format!(
                "Column {} starts at byte {} of a {} byte row",
                index,
                offset,
                self.data.len()
            ))
        })?;

        Ok(Some(format.parse_into_value(data)?))
    }
}
