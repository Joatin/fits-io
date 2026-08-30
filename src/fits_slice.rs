use crate::fits::Fits;
use crate::hdu::{ExtensionHDU, HDU};
use crate::header::header::BLOCK_NUM_BYTES;
use crate::header::{ExtensionType, Header};
use crate::slice_ascii_table_hdu::SliceAsciiTableHDU;
use crate::slice_bin_table_hdu::SliceBinTableHDU;
use crate::slice_image_hdu::SliceImageHDU;
use log::debug;
use std::error::Error;
use std::io::{Cursor, Seek, SeekFrom};
use std::sync::Arc;

/// A FITS file read from a buffer rather than from the filesystem.
///
/// This is the reader to use where there is no filesystem to read from — a file
/// already in memory, one arriving over a network, or a build with the `fs`
/// feature turned off.
#[derive(Debug, Clone)]
pub struct FitsSlice {
    primary_hdu: SliceImageHDU,
    extension_hdus: Vec<ExtensionHDU<Self>>,
}

impl FitsSlice {
    /// Reads a FITS file out of `data`.
    ///
    /// The buffer is copied once, because the HDUs read from it for as long as
    /// they live and cannot borrow from the caller's slice. Use
    /// [`FitsSlice::from_vec`] to hand over a buffer instead of copying one.
    pub fn from_slice(data: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Self::from_vec(data.to_vec())
    }

    /// Reads a FITS file out of `data`, taking ownership of the buffer.
    ///
    /// A buffer that holds gzip-compressed data is decompressed first. A FITS
    /// file starts with `SIMPLE`, so it can never be mistaken for one that
    /// starts with the gzip marker.
    pub fn from_vec(data: Vec<u8>) -> Result<Self, Box<dyn Error + Send + Sync>> {
        #[cfg(feature = "gzip")]
        let data = if is_gzipped(&data) {
            decompress(&data)?
        } else {
            data
        };

        let data = Arc::new(data);
        let mut reader: Box<dyn crate::util::ReadSeek> =
            Box::new(Cursor::new(crate::util::SharedBytes(Arc::clone(&data))));

        let header =
            Header::from_reader(&mut reader)?.ok_or("Could not read primary FITS header")?;
        header.validate_primary()?;
        debug!("Read primary header: {:?}", header);

        let data_offset = header.bytes_len();
        let primary_hdu = SliceImageHDU::new(header, Arc::clone(&data), data_offset);

        let mut extension_hdus = vec![];
        let mut offset = primary_hdu.byte_size();

        loop {
            if offset as usize >= data.len() {
                break;
            }

            reader.seek(SeekFrom::Start(offset))?;

            let Some(header) = Header::from_reader(&mut reader)? else {
                break;
            };
            header.validate_extension()?;

            let extension_type = header
                .extension()
                .ok_or("This is not a valid fits extension. Card XTENSION is missing or invalid")?;

            let data_offset = offset as usize + header.bytes_len();

            match extension_type {
                ExtensionType::Image => {
                    let hdu = SliceImageHDU::new(header, Arc::clone(&data), data_offset);
                    offset += hdu.byte_size();
                    extension_hdus.push(ExtensionHDU::Image(hdu));
                }
                ExtensionType::BinTable => {
                    let hdu = SliceBinTableHDU::new(header, Arc::clone(&data), data_offset);
                    offset += hdu.byte_size();
                    extension_hdus.push(ExtensionHDU::BinTable(hdu));
                }
                ExtensionType::AsciiTable => {
                    let hdu = SliceAsciiTableHDU::new(header, Arc::clone(&data), data_offset);
                    offset += hdu.byte_size();
                    extension_hdus.push(ExtensionHDU::AsciiTable(hdu));
                }
            }
        }

        Ok(Self {
            primary_hdu,
            extension_hdus,
        })
    }

    /// Reads a FITS file out of `data` without blocking the async runtime.
    ///
    /// Parsing a large buffer is real work even though it touches no
    /// filesystem, so it runs on a blocking worker, as
    /// [`FsFits::open_async`](crate::fs::FsFits::open_async) does.
    #[cfg(feature = "tokio")]
    pub async fn from_vec_async(data: Vec<u8>) -> Result<Self, Box<dyn Error + Send + Sync>> {
        tokio::task::spawn_blocking(move || Self::from_vec(data)).await?
    }

    /// An empty FITS file, for building one from nothing.
    pub fn new() -> Self {
        Self {
            primary_hdu: SliceImageHDU::empty(),
            extension_hdus: vec![],
        }
    }
}

impl Default for FitsSlice {
    fn default() -> Self {
        Self::new()
    }
}

impl Fits for FitsSlice {
    type ImageHDU = SliceImageHDU;
    type BinTableHDU = SliceBinTableHDU;
    type AsciiTableHDU = SliceAsciiTableHDU;

    fn primary_hdu(&self) -> &Self::ImageHDU {
        &self.primary_hdu
    }

    fn primary_hdu_mut(&mut self) -> &mut Self::ImageHDU {
        &mut self.primary_hdu
    }

    fn extension_count(&self) -> usize {
        self.extension_hdus.len()
    }

    fn extension_hdu(&self, index: usize) -> Option<&ExtensionHDU<Self>> {
        self.extension_hdus.get(index)
    }

    fn extension_hdu_mut(&mut self, index: usize) -> Option<&mut ExtensionHDU<Self>> {
        self.extension_hdus.get_mut(index)
    }

    fn extension_hdus(&self) -> impl Iterator<Item = &ExtensionHDU<Self>> {
        self.extension_hdus.iter()
    }

    fn extension_hdus_mut(&mut self) -> impl Iterator<Item = &mut ExtensionHDU<Self>> {
        self.extension_hdus.iter_mut()
    }

    fn push_extension(&mut self, extension: ExtensionHDU<Self>) {
        self.extension_hdus.push(extension);
    }

    fn remove_extension(&mut self, index: usize) -> Option<ExtensionHDU<Self>> {
        (index < self.extension_hdus.len()).then(|| self.extension_hdus.remove(index))
    }

    fn to_vec(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let mut bytes = Vec::new();

        append_hdu(
            &mut bytes,
            &self.primary_hdu.header().conformed(None),
            self.primary_hdu.data_bytes(),
            0,
        );

        for extension in &self.extension_hdus {
            match extension {
                ExtensionHDU::Image(hdu) => append_hdu(
                    &mut bytes,
                    &hdu.header().conformed(Some(ExtensionType::Image)),
                    hdu.data_bytes(),
                    0,
                ),
                ExtensionHDU::BinTable(hdu) => append_hdu(
                    &mut bytes,
                    &hdu.header().conformed(Some(ExtensionType::BinTable)),
                    hdu.data_bytes(),
                    0,
                ),
                // An ASCII table holds characters, and the standard pads it with
                // the blanks that a character field means, not with zero bytes.
                ExtensionHDU::AsciiTable(hdu) => append_hdu(
                    &mut bytes,
                    &hdu.header().conformed(Some(ExtensionType::AsciiTable)),
                    hdu.data_bytes(),
                    b' ',
                ),
            }
        }

        Ok(bytes)
    }
}

/// The two bytes that open every gzip stream.
#[cfg(feature = "gzip")]
fn is_gzipped(data: &[u8]) -> bool {
    data.starts_with(&[0x1f, 0x8b])
}

#[cfg(feature = "gzip")]
fn decompress(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    use std::io::Read;

    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    Ok(decompressed)
}

/// Appends one HDU: its header, its data, and the padding that squares the data
/// off to a whole number of blocks.
///
/// The header is rendered last, because its CHECKSUM card covers the padded data
/// as well as the header itself.
fn append_hdu(bytes: &mut Vec<u8>, header: &Header, data: &[u8], padding: u8) {
    let mut data = data.to_vec();
    let overhang = data.len() % BLOCK_NUM_BYTES;
    if overhang != 0 {
        data.resize(data.len() + BLOCK_NUM_BYTES - overhang, padding);
    }

    bytes.extend_from_slice(&header.checksummed_bytes(&data));
    bytes.extend_from_slice(&data);
}
