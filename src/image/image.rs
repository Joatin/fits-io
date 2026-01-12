use crate::header::{BayerPattern, Bitpix, Header};
use crate::image::ImageData;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::error::Error;
use image::{ImageBuffer, Luma, Rgb, RgbImage};
use std::ops::Deref;

#[derive(Debug, Clone)]
pub enum Image {
    F64(ImageData<f64>),
    F32(ImageData<f32>),
    I32(ImageData<i32>),
    I16(ImageData<i16>),
    U8(ImageData<u8>),
}

impl Image {
    /// Image width in pixels
    pub fn width(&self) -> u32 {
        match self {
            Self::F64(image) => image.width(),
            Self::F32(image) => image.width(),
            Self::I32(image) => image.width(),
            Self::I16(image) => image.width(),
            Self::U8(image) => image.width(),
        }
    }

    /// Image height in pixels
    pub fn height(&self) -> u32 {
        match self {
            Self::F64(image) => image.height(),
            Self::F32(image) => image.height(),
            Self::I32(image) => image.height(),
            Self::I16(image) => image.height(),
            Self::U8(image) => image.height(),
        }
    }

    /// The camera bayer pattern or None if the camera is monochrome
    pub fn bayer_pattern(&self) -> &Option<BayerPattern> {
        match self {
            Self::F64(image) => image.bayer_pattern(),
            Self::F32(image) => image.bayer_pattern(),
            Self::I32(image) => image.bayer_pattern(),
            Self::I16(image) => image.bayer_pattern(),
            Self::U8(image) => image.bayer_pattern(),
        }
    }

    /// Returns a normalised version of the image, where all values are converted into f64 in the range of 0.0 - 1.0
    #[cfg(feature = "image")]
    pub fn normalized(&self) -> ImageBuffer<Luma<f64>, Vec<f64>> {
        match self {
            Self::F64(image) => image.deref().clone(),
            Self::F32(image) => image.normalized(),
            Self::I32(image) => image.normalized(),
            Self::I16(image) => image.normalized(),
            Self::U8(image) => image.normalized(),
        }
    }

    /// Performs a superpixel demosaic and returns a normalised version. The superpixel algorithm is fast, but essentially cutting the resolution in half.
    #[cfg(feature = "image")]
    pub fn normalized_superpixel(
        &self,
    ) -> Result<ImageBuffer<Rgb<f64>, Vec<f64>>, Box<dyn Error + Send + Sync>> {
        match self {
            Self::F64(image) => image.normalized_superpixel(),
            Self::F32(image) => image.normalized_superpixel(),
            Self::I32(image) => image.normalized_superpixel(),
            Self::I16(image) => image.normalized_superpixel(),
            Self::U8(image) => image.normalized_superpixel(),
        }
    }

    /// Converts this image into a RgbImage from image-rs
    #[cfg(feature = "image")]
    pub fn rgb_image(&self) -> Result<RgbImage, Box<dyn Error + Send + Sync>> {
        if self.bayer_pattern().is_some() {
            let normalized = self.normalized_superpixel()?;
            let mut buffer = RgbImage::new(normalized.width(), normalized.height());
            for (x, y, pixel) in buffer.enumerate_pixels_mut() {
                let r_pixel = (u8::MAX as f64 * normalized.get_pixel(x, y)[0]) as u8;
                let g_pixel = (u8::MAX as f64 * normalized.get_pixel(x, y)[1]) as u8;
                let b_pixel = (u8::MAX as f64 * normalized.get_pixel(x, y)[2]) as u8;

                pixel[0] = r_pixel;
                pixel[1] = g_pixel;
                pixel[2] = b_pixel;
            }

            Ok(buffer)
        } else {
            let normalized = self.normalized();
            let mut buffer = RgbImage::new(self.width(), self.height());
            for (x, y, pixel) in buffer.enumerate_pixels_mut() {
                let gray_pixel = (u8::MAX as f64 * normalized.get_pixel(x, y)[0]) as u8;
                pixel[0] = gray_pixel;
                pixel[1] = gray_pixel;
                pixel[2] = gray_pixel;
            }
            Ok(buffer)
        }
    }

    pub(crate) fn from_data_and_header(
        data: Vec<u8>,
        header: &Header,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let width = header.naxis_n(0).unwrap();
        let height = header.naxis_n(1).unwrap();
        let zero_offset = header.bzero().unwrap();
        let scale = header.bscale().unwrap();
        let bayer_pattern = header.bayer_pattern();

        match header.bitpix() {
            Bitpix::F64 => {
                let image_data = data
                    .as_chunks::<8>()
                    .0
                    .into_iter()
                    .map(|i| f64::from_be_bytes(*i))
                    .collect::<Vec<_>>();
                Ok(Image::F64(ImageData::<f64>::from_data(
                    width as usize,
                    height as usize,
                    zero_offset,
                    scale,
                    bayer_pattern,
                    image_data,
                )?))
            }
            Bitpix::F32 => {
                let image_data = data
                    .as_chunks::<4>()
                    .0
                    .into_iter()
                    .map(|i| f32::from_be_bytes(*i))
                    .collect::<Vec<_>>();
                Ok(Image::F32(ImageData::<f32>::from_data(
                    width as usize,
                    height as usize,
                    zero_offset,
                    scale,
                    bayer_pattern,
                    image_data,
                )?))
            }
            Bitpix::U8 => Ok(Image::U8(ImageData::<u8>::from_data(
                width as usize,
                height as usize,
                zero_offset,
                scale,
                bayer_pattern,
                data,
            )?)),
            Bitpix::I16 => {
                let image_data = data
                    .as_chunks::<2>()
                    .0
                    .into_iter()
                    .map(|i| i16::from_be_bytes(*i))
                    .collect::<Vec<_>>();
                Ok(Image::I16(ImageData::<i16>::from_data(
                    width as usize,
                    height as usize,
                    zero_offset,
                    scale,
                    bayer_pattern,
                    image_data,
                )?))
            }
            Bitpix::I32 => {
                let image_data = data
                    .as_chunks::<4>()
                    .0
                    .into_iter()
                    .map(|i| i32::from_be_bytes(*i))
                    .collect::<Vec<_>>();
                Ok(Image::I32(ImageData::<i32>::from_data(
                    width as usize,
                    height as usize,
                    zero_offset,
                    scale,
                    bayer_pattern,
                    image_data,
                )?))
            }
        }
    }
}

#[cfg(feature = "image")]
impl From<ImageBuffer<Luma<f64>, Vec<f64>>> for Image {
    fn from(image: ImageBuffer<Luma<f64>, Vec<f64>>) -> Self {
        Image::F64(ImageData::from_buffer(image))
    }
}
