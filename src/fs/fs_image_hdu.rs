use crate::fs::open_fits_file::open_fits_file;
#[cfg(feature = "tokio")]
use crate::hdu::NormalisedImageStream;
use crate::hdu::{HDU, ImageHDU};
use crate::header::{BayerPattern, Header, ImageType};
use crate::image::{Image, Normalizer};
use crate::util::read_bytes;
#[cfg(feature = "tokio")]
use crate::util::read_bytes_async;
#[cfg(feature = "tokio")]
use futures::StreamExt;
#[cfg(feature = "tokio")]
use futures::stream;
use std::error::Error;
#[cfg(feature = "tokio")]
use std::future::ready;
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};
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
        index < self.image_count()
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
    /// The number of images in this HDU.
    ///
    /// A 2-axis HDU holds a single image and a 3-axis HDU holds NAXIS3 of them.
    /// Anything with fewer than two axes — a header-only primary HDU, for
    /// instance — holds none. Axes beyond the third are not yet counted, so a
    /// hypercube reports only its NAXIS3 planes.
    fn image_count(&self) -> usize {
        match self.header.naxis() {
            None | Some(..=1) => 0,
            Some(2) => 1,
            Some(_) => self.header.naxis_n(2).unwrap_or(0).max(0) as usize,
        }
    }

    fn images_width(&self) -> u32 {
        self.header
            .naxis_n(0)
            .and_then(|width| u32::try_from(width).ok())
            .unwrap_or(0)
    }

    fn images_height(&self) -> u32 {
        self.header
            .naxis_n(1)
            .and_then(|height| u32::try_from(height).ok())
            .unwrap_or(0)
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

        let bytes = read_bytes(&mut reader, self.image_data_size())?;

        let image = Image::from_data_and_header(bytes, &self.header)?;

        Ok(Some(image))
    }

    fn clear_images(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Err("Writing image data is not implemented yet".into())
    }

    fn set_raw_images_u8(
        &mut self,
        _width: u32,
        _height: u32,
        _images: &[&[u8]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Err("Writing image data is not implemented yet".into())
    }

    fn set_raw_images_i16(
        &mut self,
        _width: u32,
        _height: u32,
        _images: &[&[i16]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Err("Writing image data is not implemented yet".into())
    }

    fn set_raw_images_i32(
        &mut self,
        _width: u32,
        _height: u32,
        _images: &[&[i32]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Err("Writing image data is not implemented yet".into())
    }

    fn set_raw_images_f32(
        &mut self,
        _width: u32,
        _height: u32,
        _images: &[&[f32]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Err("Writing image data is not implemented yet".into())
    }

    fn set_raw_images_f64(
        &mut self,
        _width: u32,
        _height: u32,
        _images: &[&[f64]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Err("Writing image data is not implemented yet".into())
    }

    /// Streams the image as `(x, y, value)` triples, with `value` normalised to
    /// the `0.0..=1.0` range by [`Normalizer`].
    ///
    /// Pixels arrive in the order they are stored, left to right and top to
    /// bottom. Nothing is buffered beyond a single read block, so this stays
    /// cheap for images too large to hold in memory.
    ///
    /// # Errors
    ///
    /// Floating point images need DATAMIN and DATAMAX cards to be normalisable in
    /// a single pass; see [`Normalizer::from_header`].
    #[cfg(feature = "tokio")]
    fn stream_normalised_image(
        &self,
        index: usize,
    ) -> Result<Option<NormalisedImageStream<'_>>, Box<dyn Error + Send + Sync>> {
        if !self.is_image_index_valid(index) {
            return Ok(None);
        }

        let width = self.images_width();
        if width == 0 {
            return Ok(None);
        }

        let bitpix = self
            .header
            .bitpix()
            .ok_or("Cannot stream an image from a header without a BITPIX card")?;
        let normalizer = Normalizer::from_header(&self.header)?;
        let pixel_len = bitpix.byte_size();

        let mut reader = open_fits_file(&self.path)?;
        reader.seek(SeekFrom::Start(
            self.hdu_offset
                + self.header.bytes_len() as u64
                + (self.image_data_size() * index as u64),
        ))?;

        let blocks = read_bytes_async(reader, self.image_data_size());

        // Read blocks do not have to line up with pixel boundaries, so a partial
        // trailing pixel is carried over into the next block.
        let pixels = blocks
            .scan(
                (Vec::new(), 0_u64),
                move |(carry, pixel_index): &mut (Vec<u8>, u64), block| {
                    carry.extend_from_slice(&block);

                    let complete = carry.len() / pixel_len;
                    let mut decoded = Vec::with_capacity(complete);

                    for raw in carry.chunks_exact(pixel_len).take(complete) {
                        let Some(value) = bitpix.read_be(raw) else {
                            continue;
                        };

                        let x = (*pixel_index % width as u64) as u32;
                        let y = (*pixel_index / width as u64) as u32;
                        *pixel_index += 1;

                        decoded.push((x, y, normalizer.normalize(value)));
                    }

                    carry.drain(..complete * pixel_len);

                    ready(Some(stream::iter(decoded)))
                },
            )
            .flatten();

        Ok(Some(pixels.boxed()))
    }

    fn image_data_size(&self) -> u64 {
        let Some(bitpix) = self.header.bitpix() else {
            return 0;
        };

        self.images_width() as u64 * self.images_height() as u64 * bitpix.byte_size() as u64
    }
}
