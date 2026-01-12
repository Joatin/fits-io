use crate::header::BayerPattern;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::error::Error;
use core::ops::Deref;
use image::{ImageBuffer, Luma, Primitive, Rgb};

#[derive(Debug, Clone)]
pub struct ImageData<T: Primitive> {
    #[cfg(feature = "image")]
    buffer: ImageBuffer<Luma<T>, Vec<T>>,

    // BZERO
    zero_offset: f64,

    // BSCALE
    scale: f64,

    bayer_pattern: Option<BayerPattern>,
    width: u32,
    height: u32,
}

impl<T: Primitive> ImageData<T> {
    pub fn from_data(
        width: usize,
        height: usize,
        zero_offset: f64,
        scale: f64,
        bayer_pattern: Option<BayerPattern>,
        data: Vec<T>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let buffer = ImageBuffer::<Luma<T>, Vec<T>>::from_raw(width as u32, height as u32, data)
            .ok_or("Failed to construct image buffer")?;
        Ok(Self {
            buffer,
            zero_offset,
            scale,
            bayer_pattern,
            width: width as u32,
            height: height as u32,
        })
    }

    /// The camera bayer pattern or None if the camera is monochrome
    pub fn bayer_pattern(&self) -> &Option<BayerPattern> {
        &self.bayer_pattern
    }

    /// Image width in pixels
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Image height in pixels
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns the raw image data
    pub fn raw(&self) -> &[T] {
        &self.buffer.as_raw()
    }
}

impl ImageData<f64> {
    #[cfg(feature = "image")]
    pub fn normalized_superpixel(
        &self,
    ) -> Result<ImageBuffer<Rgb<f64>, Vec<f64>>, Box<dyn Error + Send + Sync>> {
        let bayer_pattern = self
            .bayer_pattern
            .ok_or_else(|| "Can not perform superpixel demosaic on a non rgb image")?;
        let mut superpixel_image =
            ImageBuffer::<Rgb<f64>, Vec<f64>>::new(self.width() / 2, self.height() / 2);
        for (x, y, pixel) in superpixel_image.enumerate_pixels_mut() {
            match bayer_pattern {
                BayerPattern::RGGB => {
                    let x = x * 2;
                    let y = y * 2;

                    let pixel_r = self.buffer.get_pixel(x, y)[0];
                    let pixel_g1 = self.buffer.get_pixel(x + 1, y)[0];
                    let pixel_g2 = self.buffer.get_pixel(x, y + 1)[0];
                    let pixel_b = self.buffer.get_pixel(x + 1, y + 1)[0];

                    let pixel_g = (pixel_g1 + pixel_g2) / 2.0;

                    pixel[0] = pixel_r;
                    pixel[1] = pixel_g;
                    pixel[2] = pixel_b;
                }
                _ => {}
            }
        }
        Ok(superpixel_image)
    }

    #[cfg(feature = "image")]
    pub fn from_buffer(buffer: ImageBuffer<Luma<f64>, Vec<f64>>) -> Self {
        let width = buffer.width();
        let height = buffer.height();
        Self {
            buffer,
            zero_offset: 0.0,
            scale: 0.0,
            bayer_pattern: None,
            width,
            height,
        }
    }
}

impl ImageData<f32> {
    #[cfg(feature = "image")]
    pub fn normalized(&self) -> ImageBuffer<Luma<f64>, Vec<f64>> {
        let mut normalized_image =
            ImageBuffer::<Luma<f64>, Vec<f64>>::new(self.width(), self.height());
        for (x, y, pixel) in self.buffer.enumerate_pixels() {
            let pixel = pixel[0] as f64;
            normalized_image.get_pixel_mut(x, y)[0] = pixel;
        }
        normalized_image
    }

    #[cfg(feature = "image")]
    pub fn normalized_superpixel(
        &self,
    ) -> Result<ImageBuffer<Rgb<f64>, Vec<f64>>, Box<dyn Error + Send + Sync>> {
        let bayer_pattern = self
            .bayer_pattern
            .ok_or_else(|| "Can not perform superpixel demosaic on a non rgb image")?;
        let mut superpixel_image =
            ImageBuffer::<Rgb<f64>, Vec<f64>>::new(self.width() / 2, self.height() / 2);
        for (x, y, pixel) in superpixel_image.enumerate_pixels_mut() {
            match bayer_pattern {
                BayerPattern::RGGB => {
                    let x = x * 2;
                    let y = y * 2;

                    let pixel_r = self.buffer.get_pixel(x, y)[0];
                    let pixel_g1 = self.buffer.get_pixel(x + 1, y)[0];
                    let pixel_g2 = self.buffer.get_pixel(x, y + 1)[0];
                    let pixel_b = self.buffer.get_pixel(x + 1, y + 1)[0];

                    let pixel_g = (pixel_g1 + pixel_g2) / 2.0;

                    pixel[0] = pixel_r as f64;
                    pixel[1] = pixel_g as f64;
                    pixel[2] = pixel_b as f64;
                }
                _ => {}
            }
        }
        Ok(superpixel_image)
    }
}

impl ImageData<i32> {
    #[cfg(feature = "image")]
    pub fn normalized(&self) -> ImageBuffer<Luma<f64>, Vec<f64>> {
        let mut normalized_image =
            ImageBuffer::<Luma<f64>, Vec<f64>>::new(self.width(), self.height());
        for (x, y, pixel) in self.buffer.enumerate_pixels() {
            let pixel = pixel[0] as f64;
            let offset_pixel = pixel + self.zero_offset;
            let scaled_pixel = offset_pixel * self.scale;
            normalized_image.get_pixel_mut(x, y)[0] =
                (i32::MAX as f64 + self.zero_offset) / scaled_pixel;
        }
        normalized_image
    }

    #[cfg(feature = "image")]
    pub fn normalized_superpixel(
        &self,
    ) -> Result<ImageBuffer<Rgb<f64>, Vec<f64>>, Box<dyn Error + Send + Sync>> {
        let bayer_pattern = self
            .bayer_pattern
            .ok_or_else(|| "Can not perform superpixel demosaic on a non rgb image")?;
        let mut superpixel_image =
            ImageBuffer::<Rgb<f64>, Vec<f64>>::new(self.width() / 2, self.height() / 2);
        for (x, y, pixel) in superpixel_image.enumerate_pixels_mut() {
            match bayer_pattern {
                BayerPattern::RGGB => {
                    let x = x * 2;
                    let y = y * 2;

                    let pixel_r = self.buffer.get_pixel(x, y)[0];
                    let pixel_g1 = self.buffer.get_pixel(x + 1, y)[0];
                    let pixel_g2 = self.buffer.get_pixel(x, y + 1)[0];
                    let pixel_b = self.buffer.get_pixel(x + 1, y + 1)[0];

                    let pixel_r = pixel_r + self.zero_offset as i32;
                    let pixel_r = pixel_r * self.scale as i32;

                    let pixel_g1 = pixel_g1 + self.zero_offset as i32;
                    let pixel_g1 = pixel_g1 * self.scale as i32;

                    let pixel_g2 = pixel_g2 + self.zero_offset as i32;
                    let pixel_g2 = pixel_g2 * self.scale as i32;

                    let pixel_b = pixel_b + self.zero_offset as i32;
                    let pixel_b = pixel_b * self.scale as i32;

                    let pixel_g = (pixel_g1 + pixel_g2) / 2;

                    pixel[0] = (i32::MAX as f64 + self.zero_offset) * pixel_r as f64;
                    pixel[1] = (i32::MAX as f64 + self.zero_offset) * pixel_g as f64;
                    pixel[2] = (i32::MAX as f64 + self.zero_offset) * pixel_b as f64;
                }
                _ => {}
            }
        }
        Ok(superpixel_image)
    }
}

impl ImageData<i16> {
    #[cfg(feature = "image")]
    pub fn normalized(&self) -> ImageBuffer<Luma<f64>, Vec<f64>> {
        let mut normalized_image =
            ImageBuffer::<Luma<f64>, Vec<f64>>::new(self.width(), self.height());
        for (x, y, pixel) in self.buffer.enumerate_pixels() {
            let mut pixel = pixel[0] as f64;
            pixel += self.zero_offset;
            pixel *= self.scale;
            normalized_image.get_pixel_mut(x, y)[0] = pixel / (i16::MAX as f64 + self.zero_offset);
        }
        normalized_image
    }

    #[cfg(feature = "image")]
    pub fn normalized_superpixel(
        &self,
    ) -> Result<ImageBuffer<Rgb<f64>, Vec<f64>>, Box<dyn Error + Send + Sync>> {
        let bayer_pattern = self
            .bayer_pattern
            .ok_or_else(|| "Can not perform superpixel demosaic on a non rgb image")?;
        let mut superpixel_image =
            ImageBuffer::<Rgb<f64>, Vec<f64>>::new(self.width() / 2, self.height() / 2);
        for (x, y, pixel) in superpixel_image.enumerate_pixels_mut() {
            match bayer_pattern {
                BayerPattern::RGGB => {
                    let x = x * 2;
                    let y = y * 2;

                    let pixel_r = self.buffer.get_pixel(x, y)[0] as f64;
                    let pixel_g1 = self.buffer.get_pixel(x + 1, y)[0] as f64;
                    let pixel_g2 = self.buffer.get_pixel(x, y + 1)[0] as f64;
                    let pixel_b = self.buffer.get_pixel(x + 1, y + 1)[0] as f64;

                    let pixel_r = pixel_r + fix_zero_offset(self.zero_offset);
                    let pixel_r = pixel_r * self.scale;

                    let pixel_g1 = pixel_g1 + fix_zero_offset(self.zero_offset);
                    let pixel_g1 = pixel_g1 * self.scale;

                    let pixel_g2 = pixel_g2 + fix_zero_offset(self.zero_offset);
                    let pixel_g2 = pixel_g2 * self.scale;

                    let pixel_b = pixel_b + fix_zero_offset(self.zero_offset);
                    let pixel_b = pixel_b * self.scale;

                    let pixel_g = (pixel_g1 + pixel_g2) / 2.0;

                    pixel[0] = pixel_r / (i16::MAX as f64 + self.zero_offset);
                    pixel[1] = pixel_g / (i16::MAX as f64 + self.zero_offset);
                    pixel[2] = pixel_b / (i16::MAX as f64 + self.zero_offset);
                }
                _ => {}
            }
        }
        Ok(superpixel_image)
    }
}

impl ImageData<u8> {
    #[cfg(feature = "image")]
    pub fn normalized(&self) -> ImageBuffer<Luma<f64>, Vec<f64>> {
        let mut normalized_image =
            ImageBuffer::<Luma<f64>, Vec<f64>>::new(self.width(), self.height());
        for (x, y, pixel) in self.buffer.enumerate_pixels() {
            let pixel = pixel[0] as f64;
            let offset_pixel = pixel + self.zero_offset;
            let scaled_pixel = offset_pixel * self.scale;
            normalized_image.get_pixel_mut(x, y)[0] =
                (u8::MAX as f64 + self.zero_offset) / scaled_pixel;
        }
        normalized_image
    }

    #[cfg(feature = "image")]
    pub fn normalized_superpixel(
        &self,
    ) -> Result<ImageBuffer<Rgb<f64>, Vec<f64>>, Box<dyn Error + Send + Sync>> {
        let bayer_pattern = self
            .bayer_pattern
            .ok_or_else(|| "Can not perform superpixel demosaic on a non rgb image")?;
        let mut superpixel_image =
            ImageBuffer::<Rgb<f64>, Vec<f64>>::new(self.width() / 2, self.height() / 2);
        for (x, y, pixel) in superpixel_image.enumerate_pixels_mut() {
            match bayer_pattern {
                BayerPattern::RGGB => {
                    let x = x * 2;
                    let y = y * 2;

                    let pixel_r = self.buffer.get_pixel(x, y)[0];
                    let pixel_g1 = self.buffer.get_pixel(x + 1, y)[0];
                    let pixel_g2 = self.buffer.get_pixel(x, y + 1)[0];
                    let pixel_b = self.buffer.get_pixel(x + 1, y + 1)[0];

                    let pixel_r = pixel_r + self.zero_offset as u8;
                    let pixel_r = pixel_r * self.scale as u8;

                    let pixel_g1 = pixel_g1 + self.zero_offset as u8;
                    let pixel_g1 = pixel_g1 * self.scale as u8;

                    let pixel_g2 = pixel_g2 + self.zero_offset as u8;
                    let pixel_g2 = pixel_g2 * self.scale as u8;

                    let pixel_b = pixel_b + self.zero_offset as u8;
                    let pixel_b = pixel_b * self.scale as u8;

                    let pixel_g = (pixel_g1 + pixel_g2) / 2;

                    pixel[0] = (u8::MAX as f64 + self.zero_offset) * pixel_r as f64;
                    pixel[1] = (u8::MAX as f64 + self.zero_offset) * pixel_g as f64;
                    pixel[2] = (u8::MAX as f64 + self.zero_offset) * pixel_b as f64;
                }
                _ => {}
            }
        }
        Ok(superpixel_image)
    }
}

#[cfg(feature = "image")]
impl<T: Primitive> Deref for ImageData<T> {
    type Target = ImageBuffer<Luma<T>, Vec<T>>;
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

fn fix_zero_offset(zero_offset: f64) -> f64 {
    if zero_offset >= 0.0 {
        zero_offset
    } else {
        zero_offset * -1.0
    }
}
