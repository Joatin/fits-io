use crate::bin_table::BinTable;
#[cfg(feature = "tokio")]
use crate::bin_table::{FieldDefinition, OwnedRow};
use crate::fs::open_fits_file::open_fits_file;
use crate::hdu::{BinTableHDU, HDU};
use crate::header::Header;
#[cfg(feature = "tokio")]
use crate::header::TableColumnFormat;
use crate::util::read_bytes;
#[cfg(feature = "tokio")]
use crate::util::read_bytes_async;
#[cfg(feature = "tokio")]
use futures::StreamExt;
#[cfg(feature = "tokio")]
use futures::stream;
#[cfg(feature = "tokio")]
use futures::stream::BoxStream;
#[cfg(feature = "serde")]
use serde::de::DeserializeOwned;
use std::error::Error;
#[cfg(feature = "tokio")]
use std::future::ready;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
#[cfg(feature = "tokio")]
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct FsBinTableHDU {
    header: Header,
    hdu_offset: u64,
    path: PathBuf,
    /// A table set through [`BinTableHDU::set_table`], which stands in for
    /// whatever the file holds until it is saved.
    pending: Option<Vec<u8>>,
}

impl FsBinTableHDU {
    /// This HDU's whole data section, as it stands in the file.
    pub(crate) fn data_bytes(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        if let Some(pending) = &self.pending {
            return Ok(pending.clone());
        }

        let len = self.header.data_bytes_len() as u64;
        if len == 0 {
            return Ok(Vec::new());
        }

        let mut reader = open_fits_file(&self.path)?;
        reader.seek(SeekFrom::Start(self.data_offset()))?;

        Ok(read_bytes(&mut reader, len)?)
    }

    /// Byte offset of this HDU's data section within the file.
    fn data_offset(&self) -> u64 {
        self.hdu_offset + self.header.bytes_len() as u64
    }

    pub fn new(
        path: &Path,
        header: Header,
        hdu_offset: u64,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            header,
            hdu_offset,
            path: path.to_path_buf(),
            pending: None,
        })
    }

    /// An HDU holding `table` and nothing read from a file.
    pub fn from_table(path: &Path, table: &BinTable) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut hdu = Self {
            header: Header::default(),
            hdu_offset: 0,
            path: path.to_path_buf(),
            pending: None,
        };
        hdu.set_table(table)?;

        Ok(hdu)
    }
}

impl HDU for FsBinTableHDU {
    fn header(&self) -> &Header {
        &self.header
    }

    fn header_mut(&mut self) -> &mut Header {
        &mut self.header
    }
}

impl BinTableHDU for FsBinTableHDU {
    /// Size of the table data section in bytes: NAXIS1 (row length) x NAXIS2 (row count).
    ///
    /// Returns 0 for a header that does not declare both, which describes an
    /// empty table rather than an unreadable one.
    fn table_data_bytes_len(&self) -> u64 {
        let (Some(bytes_per_row), Some(rows)) = (self.header.naxis_n(0), self.header.naxis_n(1))
        else {
            return 0;
        };

        let bytes_per_row = bytes_per_row.max(0) as u64;
        let rows = rows.max(0) as u64;

        bytes_per_row.saturating_mul(rows)
    }

    fn read_table(&self) -> Result<BinTable, Box<dyn Error + Send + Sync>> {
        // The whole data section, not just the rows: a table with variable
        // length array columns keeps their values in the heap that follows.
        BinTable::from_u8(&self.header, self.data_bytes()?)
    }

    fn set_table(&mut self, table: &BinTable) -> Result<(), Box<dyn Error + Send + Sync>> {
        crate::bin_table::write::apply_to_header(table, &mut self.header);
        self.pending = Some(table.data().to_vec());

        Ok(())
    }

    #[cfg(feature = "serde")]
    fn read_rows<T: DeserializeOwned + Send + Sync>(
        &self,
    ) -> Result<Vec<T>, Box<dyn Error + Send + Sync>> {
        let table = self.read_table()?;
        Ok(crate::bin_table::from_bin_table(&table)?)
    }

    #[cfg(feature = "tokio")]
    fn stream_table_rows_raw(
        &self,
    ) -> Result<BoxStream<'_, OwnedRow>, Box<dyn Error + Send + Sync>> {
        let bytes_per_row = self.header.naxis_n(0).unwrap_or(0);
        let bytes_per_row = usize::try_from(bytes_per_row)
            .map_err(|_| format!("NAXIS1 must not be negative, but was {}", bytes_per_row))?;

        // A zero-width row would make the block arithmetic below divide by zero,
        // and describes a table with nothing in it either way.
        if bytes_per_row == 0 {
            return Ok(stream::empty().boxed());
        }

        // Every row shares one set of column definitions, so they are resolved
        // once here rather than per row.
        let field_definitions = Arc::new(FieldDefinition::all_from_header(&self.header)?);

        // Streaming keeps only one block of rows in memory, but a variable
        // length array column can point anywhere in the heap, so a table with
        // such columns has to hold the heap for as long as the stream runs. A
        // table without them — the common case — holds nothing.
        let heap = Arc::new(self.read_heap(&field_definitions)?);

        let mut reader = open_fits_file(&self.path)?;
        reader.seek(SeekFrom::Start(self.data_offset()))?;

        let blocks = read_bytes_async(reader, self.table_data_bytes_len());

        // Read blocks do not have to line up with row boundaries, so a partial
        // trailing row is carried over into the next block.
        let rows = blocks
            .scan(Vec::new(), move |carry: &mut Vec<u8>, block| {
                carry.extend_from_slice(&block);

                let complete = carry.len() / bytes_per_row;
                let decoded: Vec<_> = carry
                    .chunks_exact(bytes_per_row)
                    .take(complete)
                    .map(|row| {
                        OwnedRow::new(
                            Arc::clone(&field_definitions),
                            Arc::clone(&heap),
                            row.to_vec(),
                        )
                    })
                    .collect();

                carry.drain(..complete * bytes_per_row);

                ready(Some(stream::iter(decoded)))
            })
            .flatten();

        Ok(rows.boxed())
    }

    #[cfg(feature = "serde")]
    #[cfg(feature = "tokio")]
    fn stream_table_rows<T: DeserializeOwned + Send + Sync>(
        &self,
    ) -> Result<BoxStream<'_, crate::Result<T>>, Box<dyn Error + Send + Sync>> {
        Ok(self
            .stream_table_rows_raw()?
            .map(|row| crate::bin_table::from_bin_table_row(&row.row()))
            .boxed())
    }
}

#[cfg(feature = "tokio")]
impl FsBinTableHDU {
    /// Reads the table's heap, or nothing when no column points into it.
    fn read_heap(
        &self,
        field_definitions: &[FieldDefinition],
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let has_arrays = field_definitions
            .iter()
            .any(|field| matches!(field.format, TableColumnFormat::VariableLengthArray { .. }));
        if !has_arrays {
            return Ok(Vec::new());
        }

        let heap_offset = self.header.table_heap_offset();
        let heap_len = self.header.data_bytes_len().saturating_sub(heap_offset);
        if heap_len == 0 {
            return Ok(Vec::new());
        }

        let mut reader = open_fits_file(&self.path)?;
        reader.seek(SeekFrom::Start(self.data_offset() + heap_offset as u64))?;

        Ok(read_bytes(&mut reader, heap_len as u64)?)
    }
}
