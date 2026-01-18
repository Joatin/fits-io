use crate::bin_table::value::Value;
use crate::header::TableColumnFormat;
use alloc::string::String;
use alloc::vec::Vec;

/// A data row
#[derive(Debug, Clone)]
pub struct Row<'a> {
    data: &'a [u8],
    pub field_definitions: &'a Vec<(TableColumnFormat, usize, String)>,
}

impl<'a> Row<'a> {
    pub(crate) fn new(
        field_definitions: &'a Vec<(TableColumnFormat, usize, String)>,
        data: &'a [u8],
    ) -> Self {
        Self {
            data,
            field_definitions,
        }
    }

    pub fn get(&self, key: &str) -> crate::Result<Option<Value>> {
        if let Some((format, offset, _)) = self.field_definitions.iter().find(|i| i.2.eq(key)) {
            Ok(Some(format.parse_into_value(&self.data[*offset..])?))
        } else {
            Ok(None)
        }
    }
}
