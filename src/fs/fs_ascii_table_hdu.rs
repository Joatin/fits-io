use crate::ascii_table::AsciiTable;
#[cfg(feature = "tokio")]
use crate::ascii_table::{AsciiFieldDefinition, OwnedAsciiRow};
use crate::fs::open_fits_file::open_fits_file;
use crate::hdu::{AsciiTableHDU, HDU};
use crate::header::Header;
use crate::util::read_bytes;
#[cfg(feature = "tokio")]
use crate::util::read_bytes_async;
#[cfg(feature = "tokio")]
use futures::StreamExt;
#[cfg(feature = "tokio")]
use futures::stream;
#[cfg(feature = "tokio")]
use futures::stream::BoxStream;
use std::error::Error;
#[cfg(feature = "tokio")]
use std::future::ready;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
#[cfg(feature = "tokio")]
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct FsAsciiTableHDU {
    header: Header,
    hdu_offset: u64,
    path: PathBuf,
    /// A table set through [`AsciiTableHDU::set_table`], which stands in for
    /// whatever the file holds until it is saved.
    pending: Option<Vec<u8>>,
}

impl FsAsciiTableHDU {
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
    pub fn from_table(
        path: &Path,
        table: &AsciiTable,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut hdu = Self {
            header: Header::default(),
            hdu_offset: 0,
            path: path.to_path_buf(),
            pending: None,
        };
        hdu.set_table(table)?;

        Ok(hdu)
    }

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
}

impl HDU for FsAsciiTableHDU {
    fn header(&self) -> &Header {
        &self.header
    }

    fn header_mut(&mut self) -> &mut Header {
        &mut self.header
    }
}

impl AsciiTableHDU for FsAsciiTableHDU {
    fn read_table(&self) -> Result<AsciiTable, Box<dyn Error + Send + Sync>> {
        AsciiTable::from_u8(&self.header, self.data_bytes()?)
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

        // A zero-width row would make the block arithmetic below divide by zero,
        // and describes a table with nothing in it either way.
        if bytes_per_row == 0 {
            return Ok(stream::empty().boxed());
        }

        let field_definitions = Arc::new(AsciiFieldDefinition::all_from_header(&self.header)?);

        let mut reader = open_fits_file(&self.path)?;
        reader.seek(SeekFrom::Start(self.data_offset()))?;

        let blocks = read_bytes_async(reader, self.header.data_bytes_len() as u64);

        // Read blocks do not have to line up with row boundaries, so a partial
        // trailing row is carried over into the next block.
        let rows = blocks
            .scan(Vec::new(), move |carry: &mut Vec<u8>, block| {
                carry.extend_from_slice(&block);

                let complete = carry.len() / bytes_per_row;
                let decoded: Vec<_> = carry
                    .chunks_exact(bytes_per_row)
                    .take(complete)
                    .map(|row| OwnedAsciiRow::new(Arc::clone(&field_definitions), row.to_vec()))
                    .collect();

                carry.drain(..complete * bytes_per_row);

                ready(Some(stream::iter(decoded)))
            })
            .flatten();

        Ok(rows.boxed())
    }
}
