use crate::header::BayerPattern;
use crate::image::Normalizer;
use image::{ImageBuffer, Luma, Primitive, Rgb};
use std::error::Error;
use std::ops::Deref;

/// A single image plane, plus the scaling needed to turn its raw samples into
/// physical values.
#[derive(Debug, Clone)]
pub struct ImageData<T: Primitive> {
    buffer: ImageBuffer<Luma<T>, Vec<T>>,
    normalizer: Normalizer,
    bayer_pattern: Option<BayerPattern>,
    width: u32,
    height: u32,
}

impl<T: Primitive> ImageData<T> {
    /// Builds an image from its pixels and the normaliser for them.
    pub fn from_data(
        width: usize,
        height: usize,
        normalizer: Normalizer,
        bayer_pattern: Option<BayerPattern>,
        data: Vec<T>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let buffer = ImageBuffer::<Luma<T>, Vec<T>>::from_raw(width as u32, height as u32, data)
            .ok_or("Failed to construct image buffer")?;
        Ok(Self {
            buffer,
            normalizer,
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
        self.buffer.as_raw()
    }

    /// The scaling that maps this image's raw samples onto `0.0..=1.0`.
    pub fn normalizer(&self) -> Normalizer {
        self.normalizer
    }

    /// Reads one raw sample as an `f64`, or 0.0 if `(x, y)` lies outside the image.
    fn raw_at(&self, x: u32, y: u32) -> f64 {
        if x >= self.width || y >= self.height {
            return 0.0;
        }

        self.buffer
            .get_pixel(x, y)
            .0
            .first()
            .and_then(|sample| sample.to_f64())
            .unwrap_or(0.0)
    }

    /// Reads one sample normalised to `0.0..=1.0`.
    fn normalized_at(&self, x: u32, y: u32) -> f64 {
        self.normalizer.normalize(self.raw_at(x, y))
    }

    /// Returns a normalised version of the image, where all values are converted
    /// into f64 in the range of 0.0 - 1.0.
    ///
    /// BZERO and BSCALE are applied first, so the result is a normalised view of
    /// the *physical* values rather than of the stored samples. See
    /// [`Normalizer`] for how the black and white points are chosen.
    pub fn normalized(&self) -> ImageBuffer<Luma<f64>, Vec<f64>> {
        ImageBuffer::from_fn(self.width, self.height, |x, y| {
            Luma([self.normalized_at(x, y)])
        })
    }

    /// Performs a superpixel demosaic and returns a normalised version.
    ///
    /// The superpixel algorithm treats each 2x2 Bayer tile as one output pixel:
    /// fast, but it halves the resolution in each direction. The two green
    /// samples in the tile are averaged.
    ///
    /// # Errors
    ///
    /// Returns an error for a monochrome image, which has no Bayer pattern to
    /// demosaic.
    pub fn normalized_superpixel(
        &self,
    ) -> Result<ImageBuffer<Rgb<f64>, Vec<f64>>, Box<dyn Error + Send + Sync>> {
        let bayer_pattern = self
            .bayer_pattern
            .ok_or("Can not perform superpixel demosaic on a non rgb image")?;
        let offsets = bayer_pattern.superpixel_offsets();

        Ok(ImageBuffer::from_fn(
            self.width / 2,
            self.height / 2,
            |x, y| {
                let (x, y) = (x * 2, y * 2);
                let sample = |(dx, dy): (u32, u32)| self.normalized_at(x + dx, y + dy);

                let green = (sample(offsets.green[0]) + sample(offsets.green[1])) / 2.0;

                Rgb([sample(offsets.red), green, sample(offsets.blue)])
            },
        ))
    }
}

impl ImageData<f64> {
    /// Wraps an existing buffer of physical values, deriving the black and white
    /// points from the data itself.
    pub fn from_buffer(buffer: ImageBuffer<Luma<f64>, Vec<f64>>) -> Self {
        let width = buffer.width();
        let height = buffer.height();
        let normalizer = Normalizer::from_samples(0.0, 1.0, buffer.as_raw().iter().copied());

        Self {
            buffer,
            normalizer,
            bayer_pattern: None,
            width,
            height,
        }
    }
}

impl<T: Primitive> Deref for ImageData<T> {
    type Target = ImageBuffer<Luma<T>, Vec<T>>;
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::ImageData;
    use crate::header::BayerPattern;
    use crate::image::Normalizer;

    /// The unsigned-16-bit encoding astro cameras produce: BITPIX 16, BZERO 32768.
    fn unsigned_16_bit(width: usize, height: usize, data: Vec<i16>) -> ImageData<i16> {
        ImageData::from_data(
            width,
            height,
            Normalizer::new(32768.0, 1.0, 0.0, 65535.0),
            None,
            data,
        )
        .expect("buffer dimensions match the data")
    }

    #[test]
    fn normalizing_spreads_samples_over_the_full_range() {
        let image = unsigned_16_bit(2, 2, vec![i16::MIN, -1, 0, i16::MAX]);
        let normalized = image.normalized();

        assert_eq!(normalized.get_pixel(0, 0)[0], 0.0);
        assert_eq!(normalized.get_pixel(1, 1)[0], 1.0);
        assert!((normalized.get_pixel(1, 0)[0] - 0.5).abs() < 1e-4);
    }

    #[test]
    fn normalizing_is_monotonic() {
        // The inverted `MAX / pixel` form used to make darker samples brighter.
        let image = unsigned_16_bit(4, 1, vec![i16::MIN, -16384, 16384, i16::MAX]);
        let normalized = image.normalized();

        let values: Vec<f64> = (0..4).map(|x| normalized.get_pixel(x, 0)[0]).collect();

        for pair in values.windows(2) {
            assert!(
                pair[0] < pair[1],
                "a brighter sample must not normalise darker: {values:?}"
            );
        }
    }

    #[test]
    fn normalized_values_stay_inside_the_unit_range() {
        let image = ImageData::from_data(
            3,
            1,
            Normalizer::new(0.0, 1.0, 0.0, 255.0),
            None,
            vec![0_u8, 128, 255],
        )
        .expect("buffer dimensions match the data");

        for (_, _, pixel) in image.normalized().enumerate_pixels() {
            assert!(
                (0.0..=1.0).contains(&pixel[0]),
                "value out of range: {}",
                pixel[0]
            );
        }
    }

    const RED: u8 = 200;
    const GREEN: u8 = 100;
    const BLUE: u8 = 50;

    /// A 4x4 mosaic tiling the given 2x2 pattern, read left to right and top to
    /// bottom. The tile is spelled out rather than derived from
    /// `superpixel_offsets`, so a wrong offset table cannot hide behind it.
    fn mosaic(pattern: BayerPattern, tile: [u8; 4]) -> ImageData<u8> {
        let mut data = Vec::with_capacity(16);
        for y in 0..4 {
            for x in 0..4 {
                data.push(tile[(y % 2) * 2 + (x % 2)]);
            }
        }

        ImageData::from_data(
            4,
            4,
            Normalizer::new(0.0, 1.0, 0.0, 255.0),
            Some(pattern),
            data,
        )
        .expect("buffer dimensions match the data")
    }

    #[test]
    fn every_bayer_pattern_demosaics_to_the_right_channels() {
        let cases = [
            (BayerPattern::RGGB, [RED, GREEN, GREEN, BLUE]),
            (BayerPattern::BGGR, [BLUE, GREEN, GREEN, RED]),
            (BayerPattern::GRBG, [GREEN, RED, BLUE, GREEN]),
            (BayerPattern::GBRG, [GREEN, BLUE, RED, GREEN]),
        ];

        for (pattern, tile) in cases {
            let demosaiced = mosaic(pattern, tile)
                .normalized_superpixel()
                .expect("a mosaic image can be demosaiced");

            assert_eq!(demosaiced.dimensions(), (2, 2), "{pattern:?}");

            for (x, y, pixel) in demosaiced.enumerate_pixels() {
                let [red, green, blue] = pixel.0;

                assert!(
                    (red - RED as f64 / 255.0).abs() < 1e-9,
                    "{pattern:?} red at ({x}, {y}): {red}"
                );
                assert!(
                    (green - GREEN as f64 / 255.0).abs() < 1e-9,
                    "{pattern:?} green at ({x}, {y}): {green}"
                );
                assert!(
                    (blue - BLUE as f64 / 255.0).abs() < 1e-9,
                    "{pattern:?} blue at ({x}, {y}): {blue}"
                );
            }
        }
    }

    #[test]
    fn demosaicing_a_monochrome_image_is_an_error() {
        let image = unsigned_16_bit(2, 2, vec![0, 1, 2, 3]);

        assert!(image.normalized_superpixel().is_err());
    }

    #[test]
    fn a_large_zero_offset_does_not_overflow_a_narrow_sample_type() {
        // The old code did this arithmetic in u8 and i32, so BZERO of 32768
        // overflowed before it could be applied.
        let image = ImageData::from_data(
            2,
            2,
            Normalizer::new(32768.0, 1.0, 32768.0, 33023.0),
            Some(BayerPattern::RGGB),
            vec![0_u8, 128, 200, 255],
        )
        .expect("buffer dimensions match the data");

        let demosaiced = image
            .normalized_superpixel()
            .expect("a mosaic image can be demosaiced");

        assert_eq!(demosaiced.dimensions(), (1, 1));
        for value in demosaiced.get_pixel(0, 0).0 {
            assert!((0.0..=1.0).contains(&value), "value out of range: {value}");
        }
    }

    #[test]
    fn odd_dimensions_do_not_read_outside_the_image() {
        let image = ImageData::from_data(
            3,
            3,
            Normalizer::new(0.0, 1.0, 0.0, 255.0),
            Some(BayerPattern::RGGB),
            vec![0_u8; 9],
        )
        .expect("buffer dimensions match the data");

        let demosaiced = image
            .normalized_superpixel()
            .expect("a mosaic image can be demosaiced");

        assert_eq!(demosaiced.dimensions(), (1, 1));
    }

    #[test]
    fn from_buffer_derives_its_range_from_the_data() {
        let buffer = image::ImageBuffer::from_raw(2, 2, vec![10.0_f64, 20.0, 30.0, 40.0])
            .expect("buffer dimensions match the data");
        let image = ImageData::from_buffer(buffer);

        let normalized = image.normalized();

        // The old code set BSCALE to 0.0, which flattened every image to nothing.
        assert_eq!(normalized.get_pixel(0, 0)[0], 0.0);
        assert_eq!(normalized.get_pixel(1, 1)[0], 1.0);
        assert!((normalized.get_pixel(1, 0)[0] - 1.0 / 3.0).abs() < 1e-9);
    }
}
