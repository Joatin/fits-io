use crate::hdu::{HDU, ImageHDU};
use crate::header::{BayerPattern, Header, ImageType};
use crate::image::Image;
use std::error::Error;
use std::prelude::rust_2015::Box;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SliceImageHDU {}

impl HDU for SliceImageHDU {
    fn header(&self) -> &Header {
        todo!()
    }

    fn header_mut(&mut self) -> &mut Header {
        todo!()
    }
}

impl ImageHDU for SliceImageHDU {
    fn image_count(&self) -> usize {
        todo!()
    }

    fn images_width(&self) -> u32 {
        todo!()
    }

    fn images_height(&self) -> u32 {
        todo!()
    }

    fn images_bayer_pattern(&self) -> Option<BayerPattern> {
        todo!()
    }

    fn images_type(&self) -> Option<&ImageType> {
        todo!()
    }

    fn images_exposure_time(&self) -> Option<Duration> {
        todo!()
    }

    fn read_image(&self, _index: usize) -> Result<Option<Image>, Box<dyn Error + Send + Sync>> {
        todo!()
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
        _index: usize,
    ) -> Result<Option<futures::stream::BoxStream<'_, (u32, u32, f64)>>, Box<dyn Error + Send + Sync>>
    {
        todo!()
    }

    fn image_data_size(&self) -> u64 {
        todo!()
    }
}
