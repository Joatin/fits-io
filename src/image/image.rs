use crate::header::{BayerPattern, Bitpix, Header};
use crate::image::{ImageData, Normalizer};
use image::{ImageBuffer, Luma, Primitive, Rgb, RgbImage};
use std::error::Error;

/// One image, in whatever type its BITPIX card called for.
#[derive(Debug, Clone)]
pub enum Image {
    /// A double precision image, from BITPIX -64.
    F64(ImageData<f64>),
    /// A single precision image, from BITPIX -32.
    F32(ImageData<f32>),
    /// A signed 32-bit image, from BITPIX 32.
    I32(ImageData<i32>),
    /// A signed 16-bit image, from BITPIX 16.
    I16(ImageData<i16>),
    /// An 8-bit image, from BITPIX 8.
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
    pub fn normalized(&self) -> ImageBuffer<Luma<f64>, Vec<f64>> {
        match self {
            Self::F64(image) => image.normalized(),
            Self::F32(image) => image.normalized(),
            Self::I32(image) => image.normalized(),
            Self::I16(image) => image.normalized(),
            Self::U8(image) => image.normalized(),
        }
    }

    /// Performs a superpixel demosaic and returns a normalised version. The superpixel algorithm is fast, but essentially cutting the resolution in half.
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
        let bitpix = header
            .bitpix()
            .ok_or("Cannot read an image from a header without a BITPIX card")?;
        let width = header
            .naxis_n(0)
            .ok_or("Cannot read an image from a header without a NAXIS1 card")?;
        let height = header
            .naxis_n(1)
            .ok_or("Cannot read an image from a header without a NAXIS2 card")?;
        let width = usize::try_from(width)
            .map_err(|_| format!("NAXIS1 must not be negative, but was {}", width))?;
        let height = usize::try_from(height)
            .map_err(|_| format!("NAXIS2 must not be negative, but was {}", height))?;

        let bayer_pattern = header.bayer_pattern();

        let expected = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(bitpix.byte_size()))
            .ok_or("Image dimensions overflow the address space")?;
        if data.len() < expected {
            return Err(format!(
                "Image data is too short, expected {} bytes for a {}x{} {:?} image, but got {}",
                expected,
                width,
                height,
                bitpix,
                data.len()
            )
            .into());
        }

        match bitpix {
            Bitpix::F64 => {
                let image_data = data
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|i| f64::from_be_bytes(*i))
                    .collect::<Vec<_>>();
                Ok(Image::F64(ImageData::<f64>::from_data(
                    width,
                    height,
                    normalizer_for(header, &image_data),
                    bayer_pattern,
                    image_data,
                )?))
            }
            Bitpix::F32 => {
                let image_data = data
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|i| f32::from_be_bytes(*i))
                    .collect::<Vec<_>>();
                Ok(Image::F32(ImageData::<f32>::from_data(
                    width,
                    height,
                    normalizer_for(header, &image_data),
                    bayer_pattern,
                    image_data,
                )?))
            }
            Bitpix::U8 => Ok(Image::U8(ImageData::<u8>::from_data(
                width,
                height,
                normalizer_for(header, &data),
                bayer_pattern,
                data,
            )?)),
            Bitpix::I16 => {
                let image_data = data
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|i| i16::from_be_bytes(*i))
                    .collect::<Vec<_>>();
                Ok(Image::I16(ImageData::<i16>::from_data(
                    width,
                    height,
                    normalizer_for(header, &image_data),
                    bayer_pattern,
                    image_data,
                )?))
            }
            Bitpix::I32 => {
                let image_data = data
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|i| i32::from_be_bytes(*i))
                    .collect::<Vec<_>>();
                Ok(Image::I32(ImageData::<i32>::from_data(
                    width,
                    height,
                    normalizer_for(header, &image_data),
                    bayer_pattern,
                    image_data,
                )?))
            }
        }
    }
}

/// Chooses the scaling for an image whose samples are already in memory.
///
/// Follows [`Normalizer::from_header`] — DATAMIN and DATAMAX first, then the
/// representable range of BITPIX — but adds a fallback the streaming path cannot
/// use: a floating point image has no representable range, and here the whole
/// array is in hand, so its actual extent can be measured.
fn normalizer_for<T: Primitive>(header: &Header, data: &[T]) -> Normalizer {
    Normalizer::from_header(header).unwrap_or_else(|_| {
        // `from_header` only fails for a floating point image with neither
        // DATAMIN nor DATAMAX, and BLANK does not apply to those.
        Normalizer::from_samples(
            header.bzero_or_default(),
            header.bscale_or_default(),
            data.iter().filter_map(|sample| sample.to_f64()),
        )
    })
}

impl From<ImageBuffer<Luma<f64>, Vec<f64>>> for Image {
    fn from(image: ImageBuffer<Luma<f64>, Vec<f64>>) -> Self {
        Image::F64(ImageData::from_buffer(image))
    }
}
