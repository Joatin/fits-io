use crate::bin_table::Row;
use crate::header::{Header, TableColumnFormat};
use alloc::string::ToString;
use alloc::vec;
#[cfg(feature = "rayon")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::error::Error;
use std::format;
use std::prelude::rust_2015::{Box, String, Vec};

#[derive(Debug, Clone, Default)]
pub struct BinTable {
    data: Vec<u8>,
    field_definitions: Vec<(TableColumnFormat, usize, String)>,
    rows: usize,
    bytes_per_row: usize,
}

impl BinTable {
    pub fn new(field_definitions: Vec<(TableColumnFormat, usize, String)>) -> Self {
        Self {
            data: vec![],
            field_definitions,
            rows: 0,
            bytes_per_row: 0,
        }
    }

    pub fn from_u8(header: &Header, data: Vec<u8>) -> Result<Self, Box<dyn Error + Send + Sync>> {
        if header.naxis() == 2 {
            let bytes_per_row = header.naxis_n(0).unwrap() as usize;
            let rows = header.naxis_n(1).unwrap() as usize;

            if data.len() < rows * bytes_per_row {
                return Err(format!(
                    "Data vec is too short, expected {} bytes, but data was only {} bytes long",
                    rows * bytes_per_row,
                    data.len()
                )
                .into());
            }

            let field_definitions = Self::get_table_column_formats(&header)?;
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
        (0..(self.rows)).map(move |row| self.row(row).unwrap())
    }

    #[cfg(feature = "rayon")]
    pub fn rows_parallel(&'_ self) -> impl ParallelIterator<Item = Row<'_>> + '_ {
        (0..(self.rows))
            .into_par_iter()
            .filter_map(move |row| self.row(row))
    }

    fn get_table_column_formats(
        header: &Header,
    ) -> Result<Vec<(TableColumnFormat, usize, String)>, Box<dyn Error + Send + Sync>> {
        let table_fields = header.table_fields().unwrap() as usize;
        let mut field_offset = 0;

        let field_definitions: Vec<_> = (0..table_fields)
            .map(|index| {
                let field_form = header.table_column_format(index).unwrap();
                let field_type = header.table_column_type(index).unwrap();

                let offset = field_offset;
                field_offset += field_form.bytes_len();

                Ok::<_, Box<dyn Error + Send + Sync>>((field_form, offset, field_type.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(field_definitions)
    }

    pub fn len(&self) -> usize {
        self.rows
    }
}
