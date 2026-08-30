use crate::ascii_table::{AsciiColumnFormat, AsciiFieldDefinition, AsciiRow, write};
use crate::bin_table::Value;
use crate::header::Header;
#[cfg(feature = "rayon")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::error::Error;

/// The decoded contents of an ASCII table extension.
///
/// An ASCII table stores its values as fixed-width text, one row after another,
/// with each column's position given by its TBCOLn card.
#[derive(Debug, Clone, Default)]
pub struct AsciiTable {
    data: Vec<u8>,
    field_definitions: Vec<AsciiFieldDefinition>,
    rows: usize,
    bytes_per_row: usize,
}

impl AsciiTable {
    /// Reads a table out of a header and its data section.
    pub fn from_u8(header: &Header, data: Vec<u8>) -> Result<Self, Box<dyn Error + Send + Sync>> {
        if header.naxis() != Some(2) {
            return Err("Only two dimensions are supported.".into());
        }

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

        Ok(Self {
            field_definitions: AsciiFieldDefinition::all_from_header(header)?,
            data,
            bytes_per_row,
            rows,
        })
    }

    /// The row at `row`, or `None` past the end of the table.
    pub fn row(&'_ self, row: usize) -> Option<AsciiRow<'_>> {
        if row < self.rows {
            let offset = self.bytes_per_row * row;
            let data = &self.data[offset..offset + self.bytes_per_row];
            Some(AsciiRow::new(&self.field_definitions, data))
        } else {
            None
        }
    }

    /// Every row of the table, in order.
    pub fn rows(&'_ self) -> impl Iterator<Item = AsciiRow<'_>> + '_ {
        (0..(self.rows)).filter_map(move |row| self.row(row))
    }

    /// Every row of the table, decoded in parallel.
    #[cfg(feature = "rayon")]
    pub fn rows_parallel(&'_ self) -> impl ParallelIterator<Item = AsciiRow<'_>> + '_ {
        (0..(self.rows))
            .into_par_iter()
            .filter_map(move |row| self.row(row))
    }

    /// The columns this table is made of.
    pub fn field_definitions(&self) -> &[AsciiFieldDefinition] {
        &self.field_definitions
    }

    /// The columns this table is made of, so that a caller building one can
    /// declare a TNULLn or a scaling on a column before writing rows into it.
    pub fn field_definitions_mut(&mut self) -> &mut [AsciiFieldDefinition] {
        &mut self.field_definitions
    }

    /// The width of one row in characters, as NAXIS1 gives it.
    pub fn bytes_per_row(&self) -> usize {
        self.bytes_per_row
    }

    /// The rows as the characters they are stored as.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// An empty table with the given columns, laid out left to right.
    ///
    /// Columns are separated by a single space, which is the conventional
    /// layout: an ASCII table is meant to be legible as text, and TBCOLn records
    /// wherever the columns actually land.
    pub fn from_columns(columns: &[(String, AsciiColumnFormat)]) -> Self {
        let mut field_definitions = Vec::with_capacity(columns.len());
        let mut offset = 0;

        for (name, format) in columns {
            field_definitions.push(AsciiFieldDefinition {
                format: *format,
                offset,
                name: name.clone(),
                null: None,
                scale: None,
                zero: None,
            });

            offset += format.bytes_len() + 1;
        }

        // The trailing separator is not part of the row.
        let bytes_per_row = offset.saturating_sub(1);

        Self {
            data: Vec::new(),
            field_definitions,
            rows: 0,
            bytes_per_row,
        }
    }

    /// Appends a row, one value per column.
    ///
    /// # Errors
    ///
    /// Returns an error when `values` is not one per column: a short row would
    /// leave the fields after it holding whatever the padding happened to be.
    pub fn push_row(&mut self, values: &[Value]) -> Result<(), Box<dyn Error + Send + Sync>> {
        if values.len() != self.field_definitions.len() {
            return Err(format!(
                "This table has {} columns, but the row has {} values",
                self.field_definitions.len(),
                values.len()
            )
            .into());
        }

        let start = self.data.len();
        // Anything not covered by a column — the separators between them — stays
        // blank, as an ASCII table's filler should be.
        self.data.resize(start + self.bytes_per_row, b' ');

        for (field, value) in self.field_definitions.iter().zip(values) {
            let text = write::encode(value, field.format, field.null.as_deref());
            let at = start + field.offset;

            let width = field
                .format
                .bytes_len()
                .min(self.bytes_per_row - field.offset);
            self.data[at..at + width].copy_from_slice(&text.as_bytes()[..width]);
        }

        self.rows += 1;

        Ok(())
    }

    /// The number of rows in this table.
    pub fn len(&self) -> usize {
        self.rows
    }

    /// Whether the table has no rows at all.
    pub fn is_empty(&self) -> bool {
        self.rows == 0
    }
}
