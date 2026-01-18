use crate::fs::open_fits_file::open_fits_file;
use crate::hdu::{HDU, ImageHDU};
use crate::header::{BayerPattern, Header, ImageType};
use crate::image::Image;
use crate::util::{read_bytes, read_bytes_async};
use futures::StreamExt;
use std::error::Error;
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::prelude::rust_2015::Box;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct FsImageHDU {
    header: Header,
    hdu_offset: u64,
    path: PathBuf,
}

impl FsImageHDU {
    pub(crate) fn new_primary(path: &Path, header: Header) -> Self {
        Self {
            header,
            hdu_offset: 0,
            path: path.to_path_buf(),
        }
    }

    pub fn new_extension(
        path: &Path,
        header: Header,
        hdu_offset: u64,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            header,
            hdu_offset,
            path: path.to_path_buf(),
        })
    }

    fn is_image_index_valid(&self, index: usize) -> bool {
        let naxis = self.header.naxis();
        let max_index = if naxis == 3 {
            self.header.naxis_n(3).unwrap() - 1
        } else {
            0
        };

        if index > max_index as usize {
            return false;
        }
        true
    }
}

impl HDU for FsImageHDU {
    fn header(&self) -> &Header {
        &self.header
    }

    fn header_mut(&mut self) -> &mut Header {
        &mut self.header
    }
}

impl ImageHDU for FsImageHDU {
    fn image_count(&self) -> usize {
        self.header.naxis_n(2).unwrap_or(1) as usize
    }

    fn images_width(&self) -> u32 {
        self.header.naxis_n(0).unwrap_or(0) as u32
    }

    fn images_height(&self) -> u32 {
        self.header.naxis_n(1).unwrap_or(0) as u32
    }

    fn images_bayer_pattern(&self) -> Option<BayerPattern> {
        self.header.bayer_pattern()
    }

    fn images_type(&self) -> Option<&ImageType> {
        self.header.image_type()
    }

    fn images_exposure_time(&self) -> Option<Duration> {
        self.header
            .exposure()
            .or_else(|| self.header.exposure_time())
    }

    fn read_image(&self, index: usize) -> Result<Option<Image>, Box<dyn Error + Send + Sync>> {
        if !self.is_image_index_valid(index) {
            return Ok(None);
        }

        let mut reader = open_fits_file(&self.path)?;
        reader.seek(SeekFrom::Start(
            self.hdu_offset
                + self.header.bytes_len() as u64
                + (self.image_data_size() * index as u64),
        ))?;

        let bytes = read_bytes(&mut reader, self.image_data_size());

        let image = Image::from_data_and_header(bytes, &self.header)?;

        Ok(Some(image))
    }

    fn clear_images(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn set_raw_images_u8(
        &mut self,
        _width: u32,
        _height: u32,
        _images: &[&[u8]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn set_raw_images_i16(
        &mut self,
        _width: u32,
        _height: u32,
        _images: &[&[i16]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn set_raw_images_i32(
        &mut self,
        _width: u32,
        _height: u32,
        _images: &[&[i32]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn set_raw_images_f32(
        &mut self,
        _width: u32,
        _height: u32,
        _images: &[&[f32]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn set_raw_images_f64(
        &mut self,
        _width: u32,
        _height: u32,
        _images: &[&[f64]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn stream_normalised_image(
        &self,
        index: usize,
    ) -> Result<Option<futures::stream::BoxStream<'_, (u32, u32, f64)>>, Box<dyn Error + Send + Sync>>
    {
        if !self.is_image_index_valid(index) {
            return Ok(None);
        }

        let mut reader = open_fits_file(&self.path)?;
        reader.seek(SeekFrom::Start(
            self.hdu_offset
                + self.header.bytes_len() as u64
                + (self.image_data_size() * index as u64),
        ))?;

        let bytes = read_bytes_async(reader, self.image_data_size());

        let width = self.images_width();

        Ok(Some(
            bytes
                .enumerate()
                .map(move |(index, _byte)| {
                    let x = index as u32 % width;
                    let y = index as u32 / width;

                    (x, y, 0.0)
                })
                .boxed(),
        ))
    }

    fn image_data_size(&self) -> u64 {
        self.images_width() as u64
            * self.images_height() as u64
            * self.header.bitpix().byte_size() as u64
    }
}
