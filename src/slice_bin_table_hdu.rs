use crate::bin_table::BinTable;
#[cfg(feature = "tokio")]
use crate::bin_table::{FieldDefinition, OwnedRow};
use crate::hdu::{BinTableHDU, HDU};
use crate::header::Header;
#[cfg(feature = "tokio")]
use futures::StreamExt;
#[cfg(feature = "tokio")]
use futures::stream;
#[cfg(feature = "tokio")]
use futures::stream::BoxStream;
#[cfg(feature = "serde")]
use serde::de::DeserializeOwned;
use std::error::Error;
use std::sync::Arc;

/// A binary table HDU backed by a buffer rather than a file.
#[derive(Debug, Clone)]
pub struct SliceBinTableHDU {
    header: Header,
    data: Arc<Vec<u8>>,
    data_offset: usize,
    /// A table set through [`BinTableHDU::set_table`], which stands in for
    /// whatever the buffer holds.
    pending: Option<Vec<u8>>,
}

impl SliceBinTableHDU {
    pub(crate) fn new(header: Header, data: Arc<Vec<u8>>, data_offset: usize) -> Self {
        Self {
            header,
            data,
            data_offset,
            pending: None,
        }
    }

    /// An HDU holding `table` and nothing read from a buffer.
    pub fn from_table(table: &BinTable) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut hdu = Self {
            header: Header::default(),
            data: Arc::new(Vec::new()),
            data_offset: 0,
            pending: None,
        };
        hdu.set_table(table)?;

        Ok(hdu)
    }

    /// This HDU's whole data section: the rows, then the heap.
    pub(crate) fn data_bytes(&self) -> &[u8] {
        if let Some(pending) = &self.pending {
            return pending;
        }

        let len = self.header.data_bytes_len();
        self.data
            .get(self.data_offset..)
            .and_then(|data| data.get(..len))
            .unwrap_or_default()
    }
}

impl HDU for SliceBinTableHDU {
    fn header(&self) -> &Header {
        &self.header
    }

    fn header_mut(&mut self) -> &mut Header {
        &mut self.header
    }
}

impl BinTableHDU for SliceBinTableHDU {
    fn table_data_bytes_len(&self) -> u64 {
        let (Some(bytes_per_row), Some(rows)) = (self.header.naxis_n(0), self.header.naxis_n(1))
        else {
            return 0;
        };

        (bytes_per_row.max(0) as u64).saturating_mul(rows.max(0) as u64)
    }

    fn read_table(&self) -> Result<BinTable, Box<dyn Error + Send + Sync>> {
        BinTable::from_u8(&self.header, self.data_bytes().to_vec())
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

        if bytes_per_row == 0 {
            return Ok(stream::empty().boxed());
        }

        let field_definitions = Arc::new(FieldDefinition::all_from_header(&self.header)?);

        let data = self.data_bytes();
        let heap_offset = self.header.table_heap_offset().min(data.len());
        let heap = Arc::new(data[heap_offset..].to_vec());

        // Everything is already in memory, so the stream exists to match the
        // file-backed API rather than to keep memory down.
        let rows: Vec<_> = data[..heap_offset]
            .chunks_exact(bytes_per_row)
            .map(|row| {
                OwnedRow::new(
                    Arc::clone(&field_definitions),
                    Arc::clone(&heap),
                    row.to_vec(),
                )
            })
            .collect();

        Ok(stream::iter(rows).boxed())
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
