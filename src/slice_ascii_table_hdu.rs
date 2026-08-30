use crate::ascii_table::AsciiTable;
#[cfg(feature = "tokio")]
use crate::ascii_table::{AsciiFieldDefinition, OwnedAsciiRow};
use crate::hdu::{AsciiTableHDU, HDU};
use crate::header::Header;
#[cfg(feature = "tokio")]
use futures::StreamExt;
#[cfg(feature = "tokio")]
use futures::stream;
#[cfg(feature = "tokio")]
use futures::stream::BoxStream;
use std::error::Error;
use std::sync::Arc;

/// An ASCII table HDU backed by a buffer rather than a file.
#[derive(Debug, Clone)]
pub struct SliceAsciiTableHDU {
    header: Header,
    data: Arc<Vec<u8>>,
    data_offset: usize,
    /// A table set through [`AsciiTableHDU::set_table`], which stands in for
    /// whatever the buffer holds.
    pending: Option<Vec<u8>>,
}

impl SliceAsciiTableHDU {
    pub(crate) fn new(header: Header, data: Arc<Vec<u8>>, data_offset: usize) -> Self {
        Self {
            header,
            data,
            data_offset,
            pending: None,
        }
    }

    /// An HDU holding `table` and nothing read from a buffer.
    pub fn from_table(table: &AsciiTable) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut hdu = Self {
            header: Header::default(),
            data: Arc::new(Vec::new()),
            data_offset: 0,
            pending: None,
        };
        hdu.set_table(table)?;

        Ok(hdu)
    }

    /// This HDU's whole data section.
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

impl HDU for SliceAsciiTableHDU {
    fn header(&self) -> &Header {
        &self.header
    }

    fn header_mut(&mut self) -> &mut Header {
        &mut self.header
    }
}

impl AsciiTableHDU for SliceAsciiTableHDU {
    fn read_table(&self) -> Result<AsciiTable, Box<dyn Error + Send + Sync>> {
        AsciiTable::from_u8(&self.header, self.data_bytes().to_vec())
    }

    fn set_table(&mut self, table: &AsciiTable) -> Result<(), Box<dyn Error + Send + Sync>> {
        crate::ascii_table::write::apply_to_header(table, &mut self.header);
        self.pending = Some(table.data().to_vec());

        Ok(())
    }

    #[cfg(feature = "tokio")]
    fn stream_table_rows_raw(
        &self,
    ) -> Result<BoxStream<'_, OwnedAsciiRow>, Box<dyn Error + Send + Sync>> {
        let bytes_per_row = self.header.naxis_n(0).unwrap_or(0);
        let bytes_per_row = usize::try_from(bytes_per_row)
            .map_err(|_| format!("NAXIS1 must not be negative, but was {}", bytes_per_row))?;

        if bytes_per_row == 0 {
            return Ok(stream::empty().boxed());
        }

        let field_definitions = Arc::new(AsciiFieldDefinition::all_from_header(&self.header)?);

        let rows: Vec<_> = self
            .data_bytes()
            .chunks_exact(bytes_per_row)
            .map(|row| OwnedAsciiRow::new(Arc::clone(&field_definitions), row.to_vec()))
            .collect();

        Ok(stream::iter(rows).boxed())
    }
}
