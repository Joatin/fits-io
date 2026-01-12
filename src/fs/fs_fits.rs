use crate::fits::Fits;
use crate::fs::fs_ascii_table_hdu::FsAsciiTableHDU;
use crate::fs::fs_bin_table_hdu::FsBinTableHDU;
use crate::fs::fs_image_hdu::FsImageHDU;
use crate::fs::is_fits_file;
use crate::fs::open_fits_file::open_fits_file;
use crate::hdu::{ExtensionHDU, HDU};
use crate::header::{ExtensionType, Header};
use alloc::vec;
use log::{debug, info};
use std::error::Error;
use std::io::Seek;
use std::path::{Path, PathBuf};
use std::prelude::rust_2015::{Box, Vec};
use std::sync::Arc;

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

        let header = Header::from_reader(&mut reader)?
            .ok_or_else(|| "Could not read primary FITS header")?;
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

                match header.extension().unwrap() {
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

    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            primary_hdu: FsImageHDU::new_primary(path, Header::default()),
            extension_hdus: vec![],
        }
    }

    pub fn save(&self) -> () {
        todo!();
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

    fn extension_count(&mut self) -> usize {
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

    fn to_vec(&self) -> Vec<u8> {
        todo!()
    }
}
