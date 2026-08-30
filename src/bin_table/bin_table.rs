use crate::bin_table::{FieldDefinition, Row, Value};
use crate::header::Header;
#[cfg(feature = "rayon")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::error::Error;

/// The decoded contents of a binary table extension.
#[derive(Debug, Clone, Default)]
pub struct BinTable {
    /// The HDU's whole data section: the rows, then the heap.
    data: Vec<u8>,
    field_definitions: Vec<FieldDefinition>,
    rows: usize,
    bytes_per_row: usize,
    /// Where the heap starts within `data`, from THEAP or the end of the rows.
    heap_offset: usize,
}

impl BinTable {
    /// Appends a row, one value per column.
    ///
    /// This is how to build a table without going through `serde` — for a
    /// complex column, say, which no Rust type maps onto on its own.
    ///
    /// # Errors
    ///
    /// Returns an error when `values` is not one per column: a short row would
    /// leave the columns after it holding whatever the padding happened to be.
    pub fn push_row(&mut self, values: &[Value]) -> Result<(), Box<dyn Error + Send + Sync>> {
        if values.len() != self.field_definitions.len() {
            return Err(format!(
                "This table has {} columns, but the row has {} values",
                self.field_definitions.len(),
                values.len()
            )
            .into());
        }

        // The columns were measured when the table was made, so a row is always
        // the same width.
        if self.bytes_per_row == 0 {
            self.bytes_per_row = self
                .field_definitions
                .iter()
                .map(|field| field.format.bytes_len())
                .sum();
        }

        for (field, value) in self.field_definitions.iter().zip(values) {
            crate::bin_table::encode::encode(value, field.format, &mut self.data);
        }

        self.rows += 1;
        self.heap_offset = self.data.len();

        Ok(())
    }

    /// An empty table with the given columns.
    pub fn new(field_definitions: Vec<FieldDefinition>) -> Self {
        Self {
            data: vec![],
            field_definitions,
            rows: 0,
            bytes_per_row: 0,
            heap_offset: 0,
        }
    }

    /// Builds a table from rows that are already encoded.
    ///
    /// `data` is the rows followed by the heap, and `heap_offset` says where the
    /// one ends and the other begins. A table with no variable length array
    /// columns has an empty heap, so its offset is the end of the rows.
    #[cfg(feature = "serde")]
    pub(crate) fn from_parts(
        field_definitions: Vec<FieldDefinition>,
        data: Vec<u8>,
        bytes_per_row: usize,
        rows: usize,
        heap_offset: usize,
    ) -> Self {
        Self {
            heap_offset: heap_offset.min(data.len()),
            data,
            field_definitions,
            bytes_per_row,
            rows,
        }
    }

    /// The columns this table is made of.
    pub fn field_definitions(&self) -> &[FieldDefinition] {
        &self.field_definitions
    }

    /// The width of one row in bytes, as NAXIS1 gives it.
    pub fn bytes_per_row(&self) -> usize {
        self.bytes_per_row
    }

    /// Reads a table out of a header and its data section.
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

            let field_definitions = FieldDefinition::all_from_header(header)?;
            Ok(Self {
                heap_offset: header.table_heap_offset().min(data.len()),
                data,
                field_definitions,
                bytes_per_row,
                rows,
            })
        } else {
            Err("Only two dimensions are supported.".into())
        }
    }

    /// The row at `row`, or `None` past the end of the table.
    pub fn row(&'_ self, row: usize) -> Option<Row<'_>> {
        if row < self.rows {
            let offset = self.bytes_per_row * row;
            let data = &self.data[offset..offset + self.bytes_per_row];
            Some(Row::new(&self.field_definitions, data, self.heap()))
        } else {
            None
        }
    }

    /// The whole data section: the rows, and then the heap.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// The size of the heap, which is what a table's PCOUNT card records.
    pub fn heap_len(&self) -> usize {
        self.data.len().saturating_sub(self.heap_offset)
    }

    /// The table's heap: the values that variable length array columns point at.
    ///
    /// Empty for a table without such columns, which is most of them.
    pub fn heap(&self) -> &[u8] {
        &self.data[self.heap_offset..]
    }

    /// Every row of the table, in order.
    pub fn rows(&'_ self) -> impl Iterator<Item = Row<'_>> + '_ {
        (0..(self.rows)).filter_map(move |row| self.row(row))
    }

    /// Every row of the table, decoded in parallel.
    #[cfg(feature = "rayon")]
    pub fn rows_parallel(&'_ self) -> impl ParallelIterator<Item = Row<'_>> + '_ {
        (0..(self.rows))
            .into_par_iter()
            .filter_map(move |row| self.row(row))
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
