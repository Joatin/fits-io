use crate::hdu::hdu::HDU;
use crate::header::{BayerPattern, ImageType};
use crate::image::Image;
use alloc::boxed::Box;
#[cfg(feature = "image")]
use image::{ImageBuffer, Luma, Primitive};
use std::error::Error;
use std::fmt;
use std::prelude::rust_2015::Vec;

pub trait ImageHDU: HDU + fmt::Debug + Send + Sync {
    fn image_count(&self) -> usize;
    fn images_width(&self) -> u32;
    fn images_height(&self) -> u32;
    fn images_bayer_pattern(&self) -> Option<BayerPattern>;
    fn images_type(&self) -> Option<&ImageType>;
    fn images_exposure_time(&self) -> Option<core::time::Duration>;
    fn read_image(&self, index: usize) -> Result<Option<Image>, Box<dyn Error + Send + Sync>>;

    #[cfg(feature = "image")]
    fn set_images_u8(
        &mut self,
        images: &[&ImageBuffer<Luma<u8>, Vec<u8>>],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        get_raw_data_from_image(self, images, Self::set_raw_images_u8)
    }
    #[cfg(feature = "image")]
    fn set_images_i16(
        &mut self,
        images: &[&ImageBuffer<Luma<i16>, Vec<i16>>],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        get_raw_data_from_image(self, images, Self::set_raw_images_i16)
    }
    #[cfg(feature = "image")]
    fn set_images_i32(
        &mut self,
        images: &[&ImageBuffer<Luma<i32>, Vec<i32>>],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        get_raw_data_from_image(self, images, Self::set_raw_images_i32)
    }
    #[cfg(feature = "image")]
    fn set_images_f32(
        &mut self,
        images: &[&ImageBuffer<Luma<f32>, Vec<f32>>],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        get_raw_data_from_image(self, images, Self::set_raw_images_f32)
    }
    #[cfg(feature = "image")]
    fn set_images_f64(
        &mut self,
        images: &[&ImageBuffer<Luma<f64>, Vec<f64>>],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        get_raw_data_from_image(self, images, Self::set_raw_images_f64)
    }

    fn clear_images(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>;

    fn set_raw_images_u8(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[u8]],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn set_raw_images_i16(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[i16]],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn set_raw_images_i32(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[i32]],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn set_raw_images_f32(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[f32]],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    fn set_raw_images_f64(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[f64]],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    #[cfg(feature = "tokio")]
    fn stream_normalised_image(
        &self,
        index: usize,
    ) -> Result<Option<futures::stream::BoxStream<'_, (u32, u32, f64)>>, Box<dyn Error + Send + Sync>>;
    fn image_data_size(&self) -> u64;
}

fn get_raw_data_from_image<
    'a,
    T: Primitive,
    S: ImageHDU + ?Sized,
    CB: FnOnce(&mut S, u32, u32, &[&[T]]) -> Result<(), Box<dyn Error + Send + Sync>>,
>(
    hdu: &mut S,
    images: &'a [&'a ImageBuffer<Luma<T>, Vec<T>>],
    callback: CB,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if images.is_empty() {
        hdu.clear_images()?;
        Ok(())
    } else {
        let width = images[0].width();
        let height = images[0].width();

        let data = images
            .into_iter()
            .map(|image| image.iter().as_slice())
            .collect::<Vec<_>>();

        callback(hdu, width, height, &data)
    }
}
