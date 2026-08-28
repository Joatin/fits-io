use crate::bin_table::Row;
use crate::header::{Header, TableColumnFormat};
#[cfg(feature = "rayon")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::error::Error;

/// One column's format, its byte offset in the row, and its TTYPEn name.
pub type FieldDefinition = (TableColumnFormat, usize, String);

#[derive(Debug, Clone, Default)]
pub struct BinTable {
    data: Vec<u8>,
    field_definitions: Vec<FieldDefinition>,
    rows: usize,
    bytes_per_row: usize,
}

impl BinTable {
    pub fn new(field_definitions: Vec<FieldDefinition>) -> Self {
        Self {
            data: vec![],
            field_definitions,
            rows: 0,
            bytes_per_row: 0,
        }
    }

    pub fn from_u8(header: &Header, data: Vec<u8>) -> Result<Self, Box<dyn Error + Send + Sync>> {
        if header.naxis() == Some(2) {
            let bytes_per_row = header
                .naxis_n(0)
                .ok_or("Table header is missing its NAXIS1 card")?;
            let rows = header
                .naxis_n(1)
                .ok_or("Table header is missing its NAXIS2 card")?;
            let bytes_per_row = usize::try_from(bytes_per_row)
                .map_err(|_| format!("NAXIS1 must not be negative, but was {}", bytes_per_row))?;
            let rows = usize::try_from(rows)
                .map_err(|_| format!("NAXIS2 must not be negative, but was {}", rows))?;

            let expected = rows
                .checked_mul(bytes_per_row)
                .ok_or("Table dimensions overflow the address space")?;
            if data.len() < expected {
                return Err(format!(
                    "Data vec is too short, expected {} bytes, but data was only {} bytes long",
                    expected,
                    data.len()
                )
                .into());
            }

            let field_definitions = Self::get_table_column_formats(header)?;
            Ok(Self {
                data,
                field_definitions,
                bytes_per_row,
                rows,
            })
        } else {
            Err("Only two dimensions are supported.".into())
        }
    }

    pub fn row(&'_ self, row: usize) -> Option<Row<'_>> {
        if row < self.rows {
            let offset = self.bytes_per_row * row;
            let data = &self.data[offset..offset + self.bytes_per_row];
            Some(Row::new(&self.field_definitions, data))
        } else {
            None
        }
    }

    pub fn rows(&'_ self) -> impl Iterator<Item = Row<'_>> + '_ {
        (0..(self.rows)).filter_map(move |row| self.row(row))
    }

    #[cfg(feature = "rayon")]
    pub fn rows_parallel(&'_ self) -> impl ParallelIterator<Item = Row<'_>> + '_ {
        (0..(self.rows))
            .into_par_iter()
            .filter_map(move |row| self.row(row))
    }

    fn get_table_column_formats(
        header: &Header,
    ) -> Result<Vec<FieldDefinition>, Box<dyn Error + Send + Sync>> {
        let table_fields = header
            .table_fields()
            .ok_or("Table header is missing its TFIELDS card")?;
        let table_fields = usize::try_from(table_fields)
            .map_err(|_| format!("TFIELDS must not be negative, but was {}", table_fields))?;
        let mut field_offset = 0;

        let field_definitions: Vec<_> = (0..table_fields)
            .map(|index| {
                // TFORMn is mandatory: without it the column width, and therefore
                // every following column offset, is unknown.
                let field_form = header.table_column_format(index).ok_or_else(|| {
                    format!("Table header is missing its TFORM{} card", index + 1)
                })?;

                // TTYPEn is optional. An unnamed column still occupies its bytes,
                // it just cannot be looked up by name.
                let field_type = header.table_column_type(index).unwrap_or_default();

                let offset = field_offset;
                field_offset += field_form.bytes_len();

                Ok::<_, Box<dyn Error + Send + Sync>>((field_form, offset, field_type.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(field_definitions)
    }

    /// The number of rows in this table.
    pub fn len(&self) -> usize {
        self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows == 0
    }
}
