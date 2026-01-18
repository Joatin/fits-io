use crate::fits::Fits;
use crate::hdu::ExtensionHDU;
use crate::header::{BayerPattern, Bitpix};
use crate::image::{Image, ImageData};
use crate::slice_ascii_table_hdu::SliceAsciiTableHDU;
use crate::slice_bin_table_hdu::SliceBinTableHDU;
use crate::slice_image_hdu::SliceImageHDU;
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use std::error::Error;

/// A Fits file created from a buffer
#[derive(Debug, Clone)]
pub struct FitsSlice {
    primary_hdu: SliceImageHDU,
    extension_hdus: Vec<ExtensionHDU<Self>>,
}

impl FitsSlice {
    /// Creates a new empty fits file
    pub fn from_slice(_data: &[u8]) -> Self {
        Self {
            primary_hdu: SliceImageHDU {},
            extension_hdus: vec![],
        }
    }

    // pub(crate) fn parse_extension_hdus(data: &[u8]) -> Result<Vec<ExtensionHDU>, Box<dyn Error + Send + Sync>> {
    //     let mut offset = 0;
    //     let mut extension_hdus = vec![];
    //     while offset < data.len() {
    //         let header = Header::from_u8(data)?;
    //         let bytes_len = header.data_block_len();
    //         offset += header.bytes_len();
    //         let extension_type = header.extension().ok_or_else(|| "Missing extension type".to_string())?;
    //
    //         match extension_type {
    //             ExtensionType::Image => {
    //                 if bytes_len == 0 {
    //                     extension_hdus.push(ExtensionHDU::Image(ImageHDU::new(header, vec![])));
    //                 } else {
    //                     offset += bytes_len;
    //                     let bitpix = header.bitpix();
    //                     let bzero = header.bzero().unwrap();
    //                     let bscale = header.bscale().unwrap();
    //
    //                     let width = header.naxis_n(1).unwrap();
    //                     let height = header.naxis_n(2).unwrap();
    //
    //                     let bayer_pattern = header.bayer_pattern();
    //
    //                     let image = Self::read_image(&data, header.bytes_len(), bitpix, bscale, bzero, bayer_pattern, width as usize, height as usize)?;
    //
    //                     extension_hdus.push(ExtensionHDU::Image(ImageHDU::new(header, vec![image])));
    //                 }
    //             }
    //             ExtensionType::BinTable => {
    //                 extension_hdus.push(ExtensionHDU::BinTable(BinTableHDU::from_u8(header, data[offset..(bytes_len + offset)].to_vec())?));
    //                 offset += bytes_len;
    //             }
    //             ExtensionType::AsciiTable => {}
    //         }
    //     }
    //
    //     Ok(extension_hdus)
    // }

    fn read_image(
        data: &[u8],
        offset: usize,
        bitpix: Bitpix,
        bscale: f64,
        bzero: f64,
        bayer_pattern: Option<BayerPattern>,
        width: usize,
        height: usize,
    ) -> Result<Image, Box<dyn Error + Send + Sync>> {
        let bytes = bitpix.byte_size() * width * height;

        let data = &data[offset..(bytes + offset)];

        match bitpix {
            Bitpix::F64 => {
                let data = data
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|i| f64::from_be_bytes(*i))
                    .collect();
                Ok(Image::F64(ImageData::from_data(
                    width,
                    height,
                    bzero,
                    bscale,
                    bayer_pattern,
                    data,
                )?))
            }
            Bitpix::F32 => {
                let data = data
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|i| f32::from_be_bytes(*i))
                    .collect();
                Ok(Image::F32(ImageData::from_data(
                    width,
                    height,
                    bzero,
                    bscale,
                    bayer_pattern,
                    data,
                )?))
            }
            Bitpix::U8 => {
                let data = data.to_vec();
                Ok(Image::U8(ImageData::from_data(
                    width,
                    height,
                    bzero,
                    bscale,
                    bayer_pattern,
                    data,
                )?))
            }
            Bitpix::I16 => {
                let data = data
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|i| i16::from_be_bytes(*i))
                    .collect();
                Ok(Image::I16(ImageData::from_data(
                    width,
                    height,
                    bzero,
                    bscale,
                    bayer_pattern,
                    data,
                )?))
            }
            Bitpix::I32 => {
                let data = data
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|i| i32::from_be_bytes(*i))
                    .collect();
                Ok(Image::I32(ImageData::from_data(
                    width,
                    height,
                    bzero,
                    bscale,
                    bayer_pattern,
                    data,
                )?))
            }
        }
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
