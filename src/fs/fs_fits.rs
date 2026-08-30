use crate::fits::Fits;
use crate::fs::fs_ascii_table_hdu::FsAsciiTableHDU;
use crate::fs::fs_bin_table_hdu::FsBinTableHDU;
use crate::fs::fs_image_hdu::FsImageHDU;
use crate::fs::is_fits_file;
use crate::fs::open_fits_file::open_fits_file;
use crate::hdu::{ExtensionHDU, HDU};
use crate::header::header::BLOCK_NUM_BYTES;
use crate::header::{ExtensionType, Header};
use log::{debug, info};
use std::error::Error;
use std::fs;
use std::io::Seek;
use std::path::{Path, PathBuf};

/// A FITS file read from the filesystem.
#[derive(Debug, Clone)]
pub struct FsFits {
    path: PathBuf,
    primary_hdu: FsImageHDU,
    extension_hdus: Vec<ExtensionHDU<Self>>,
}

impl FsFits {
    /// Opens a new fits file
    pub fn open(path: &Path) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Self::assert_file_type(path)?;
        debug!("Opening FITS file: {:?}", path);
        let mut reader = open_fits_file(path)?;

        let header =
            Header::from_reader(&mut reader)?.ok_or("Could not read primary FITS header")?;
        debug!("Opened primary header: {:?}", header);
        header.validate_primary()?;

        let primary_hdu = FsImageHDU::new_primary(path, header);

        let mut extension_hdus = vec![];
        let mut offset = primary_hdu.byte_size();

        loop {
            reader.seek(std::io::SeekFrom::Start(offset))?;

            if let Some(header) = Header::from_reader(&mut reader)? {
                header.validate_extension()?;
                debug!("Found extension header: {:?}", header);

                let extension_type = header.extension().ok_or(
                    "This is not a valid fits extension. Card XTENSION is missing or invalid",
                )?;

                match extension_type {
                    ExtensionType::Image => {
                        let extension_hdu = FsImageHDU::new_extension(path, header, offset)?;
                        offset += extension_hdu.byte_size();
                        extension_hdus.push(ExtensionHDU::Image(extension_hdu));
                    }
                    ExtensionType::BinTable => {
                        let extension_hdu = FsBinTableHDU::new(path, header, offset)?;
                        offset += extension_hdu.byte_size();
                        extension_hdus.push(ExtensionHDU::BinTable(extension_hdu));
                    }
                    ExtensionType::AsciiTable => {
                        let extension_hdu = FsAsciiTableHDU::new(path, header, offset)?;
                        offset += extension_hdu.byte_size();
                        extension_hdus.push(ExtensionHDU::AsciiTable(extension_hdu));
                    }
                }
            } else {
                break;
            }
        }
        info!("Opened FITS file: {:?}", path);
        Ok(Self {
            path: path.to_path_buf(),
            primary_hdu,
            extension_hdus,
        })
    }

    /// Opens a file asynchronously, this avoids blocking the tokio runtime
    #[cfg(feature = "tokio")]
    pub async fn open_async(path: &Path) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || Self::open(&path)).await?
    }

    /// An empty FITS file that will be written to `path`.
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            primary_hdu: FsImageHDU::new_primary(path, Header::default()),
            extension_hdus: vec![],
        }
    }

    /// Writes this FITS file back to [`FsFits::path`].
    ///
    /// The file is written to a temporary file beside it and then renamed, so a
    /// failure part way through leaves the original where it was rather than
    /// truncated.
    pub fn save(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.save_as(&self.path)
    }

    /// Writes this FITS file to `path`.
    pub fn save_as(&self, path: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
        let bytes = self.to_vec()?;

        let temporary = path.with_extension(format!(
            "{}.fits-io-tmp",
            path.extension().unwrap_or_default().to_string_lossy()
        ));

        fs::write(&temporary, &bytes)?;
        if let Err(error) = fs::rename(&temporary, path) {
            // Leaving the half-written file behind would be worse than the
            // failure itself.
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }

        info!("Wrote FITS file: {:?}", path);

        Ok(())
    }

    /// Retrieves the path this FITS file belongs to
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn assert_file_type(path: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
        if is_fits_file(path) {
            Ok(())
        } else {
            Err("Invalid file type".into())
        }
    }
}

impl Fits for FsFits {
    type ImageHDU = FsImageHDU;
    type BinTableHDU = FsBinTableHDU;
    type AsciiTableHDU = FsAsciiTableHDU;

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

    /// Serialises this file, primary HDU first and then every extension.
    ///
    /// Each HDU contributes its header followed by its data section, both padded
    /// out to whole 2880-byte blocks.
    fn to_vec(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let mut bytes = Vec::new();

        append_hdu(
            &mut bytes,
            &self.primary_hdu.header().conformed(None),
            &self.primary_hdu.data_bytes()?,
            DataPadding::Zero,
        )?;

        for extension in &self.extension_hdus {
            let (header, data, padding) = match extension {
                ExtensionHDU::Image(hdu) => (
                    hdu.header().conformed(Some(ExtensionType::Image)),
                    hdu.data_bytes()?,
                    DataPadding::Zero,
                ),
                ExtensionHDU::BinTable(hdu) => (
                    hdu.header().conformed(Some(ExtensionType::BinTable)),
                    hdu.data_bytes()?,
                    DataPadding::Zero,
                ),
                // An ASCII table holds characters, and the standard pads it with
                // the blanks that a character field means, not with zero bytes.
                ExtensionHDU::AsciiTable(hdu) => (
                    hdu.header().conformed(Some(ExtensionType::AsciiTable)),
                    hdu.data_bytes()?,
                    DataPadding::Blank,
                ),
            };

            append_hdu(&mut bytes, &header, &data, padding)?;
        }

        Ok(bytes)
    }
}

/// What a data section is padded out to its block boundary with.
#[derive(Debug, Clone, Copy)]
enum DataPadding {
    Zero,
    Blank,
}

impl DataPadding {
    fn byte(self) -> u8 {
        match self {
            DataPadding::Zero => 0,
            DataPadding::Blank => b' ',
        }
    }
}

/// Appends one HDU: its header, its data, and the padding that squares the data
/// off to a whole number of blocks.
///
/// The header is rendered last, because its CHECKSUM card covers the padded data
/// as well as the header itself.
fn append_hdu(
    bytes: &mut Vec<u8>,
    header: &Header,
    data: &[u8],
    padding: DataPadding,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    header.validate_against_data(data.len())?;

    let mut data = data.to_vec();
    let overhang = data.len() % BLOCK_NUM_BYTES;
    if overhang != 0 {
        data.resize(data.len() + BLOCK_NUM_BYTES - overhang, padding.byte());
    }

    bytes.extend_from_slice(&header.checksummed_bytes(&data));
    bytes.extend_from_slice(&data);

    Ok(())
}
