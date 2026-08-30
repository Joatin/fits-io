use crate::hdu::hdu::HDU;
use crate::header::{BayerPattern, ImageType};
use crate::image::{Group, Image};
use image::{ImageBuffer, Luma, Primitive};
use std::error::Error;
use std::fmt;

/// A stream of `(x, y, value)` triples, with `value` normalised to `0.0..=1.0`.
#[cfg(feature = "tokio")]
pub type NormalisedImageStream<'a> = futures::stream::BoxStream<'a, (u32, u32, f64)>;

/// An HDU whose data section is an image, or a stack of them.
pub trait ImageHDU: HDU + fmt::Debug + Send + Sync {
    /// How many images this HDU holds.
    fn image_count(&self) -> usize;
    /// The width of every image here, from NAXIS1.
    fn images_width(&self) -> u32;
    /// The height of every image here, from NAXIS2.
    fn images_height(&self) -> u32;
    /// The colour filter layout over the sensor, or `None` if it was monochrome.
    fn images_bayer_pattern(&self) -> Option<BayerPattern>;
    /// Whether these are light, dark, flat or bias frames.
    fn images_type(&self) -> Option<&ImageType>;
    /// How long the exposure lasted.
    fn images_exposure_time(&self) -> Option<std::time::Duration>;
    /// Reads one image, or `None` past the last one.
    fn read_image(&self, index: usize) -> Result<Option<Image>, Box<dyn Error + Send + Sync>>;

    /// How many groups this HDU holds, under the random-groups convention.
    ///
    /// Zero for an ordinary image HDU, which is nearly all of them; see
    /// [`Group`](crate::image::Group).
    fn group_count(&self) -> usize {
        0
    }

    /// Reads one group, or `None` past the last one.
    fn read_group(&self, index: usize) -> Result<Option<Group>, Box<dyn Error + Send + Sync>>;

    /// Replaces the images with 8-bit ones, taking their size from the first.
    fn set_images_u8(
        &mut self,
        images: &[&ImageBuffer<Luma<u8>, Vec<u8>>],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        get_raw_data_from_image(self, images, Self::set_raw_images_u8)
    }
    /// Replaces the images with signed 16-bit ones.
    fn set_images_i16(
        &mut self,
        images: &[&ImageBuffer<Luma<i16>, Vec<i16>>],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        get_raw_data_from_image(self, images, Self::set_raw_images_i16)
    }
    /// Replaces the images with signed 32-bit ones.
    fn set_images_i32(
        &mut self,
        images: &[&ImageBuffer<Luma<i32>, Vec<i32>>],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        get_raw_data_from_image(self, images, Self::set_raw_images_i32)
    }
    /// Replaces the images with single precision floating point ones.
    fn set_images_f32(
        &mut self,
        images: &[&ImageBuffer<Luma<f32>, Vec<f32>>],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        get_raw_data_from_image(self, images, Self::set_raw_images_f32)
    }
    /// Replaces the images with double precision floating point ones.
    fn set_images_f64(
        &mut self,
        images: &[&ImageBuffer<Luma<f64>, Vec<f64>>],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        get_raw_data_from_image(self, images, Self::set_raw_images_f64)
    }

    /// Removes every image, leaving a header-only HDU.
    fn clear_images(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Replaces the images with raw 8-bit samples, `width` by `height` each.
    ///
    /// The header's BITPIX and NAXISn cards are brought into line with them.
    fn set_raw_images_u8(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[u8]],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    /// Replaces the images with raw signed 16-bit samples.
    fn set_raw_images_i16(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[i16]],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    /// Replaces the images with raw signed 32-bit samples.
    fn set_raw_images_i32(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[i32]],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    /// Replaces the images with raw single precision samples.
    fn set_raw_images_f32(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[f32]],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    /// Replaces the images with raw double precision samples.
    fn set_raw_images_f64(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[f64]],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Streams one image as `(x, y, value)` triples, normalised to `0.0..=1.0`.
    #[cfg(feature = "tokio")]
    fn stream_normalised_image(
        &self,
        index: usize,
    ) -> Result<Option<NormalisedImageStream<'_>>, Box<dyn Error + Send + Sync>>;
    /// How many bytes one image occupies.
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
        let height = images[0].height();

        let data = images
            .iter()
            .map(|image| image.iter().as_slice())
            .collect::<Vec<_>>();

        callback(hdu, width, height, &data)
    }
}
