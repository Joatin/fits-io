#[cfg(feature = "tokio")]
use crate::hdu::NormalisedImageStream;
use crate::hdu::{HDU, ImageHDU};
use crate::header::card::Card;
use crate::header::{BayerPattern, Bitpix, Header, ImageType};
#[cfg(feature = "tokio")]
use crate::image::Normalizer;
use crate::image::{Group, Image};
#[cfg(feature = "tokio")]
use futures::StreamExt;
#[cfg(feature = "tokio")]
use futures::stream;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

/// An image HDU backed by a buffer rather than a file.
#[derive(Debug, Clone)]
pub struct SliceImageHDU {
    header: Header,
    /// The whole FITS buffer, shared by every HDU in it.
    data: Arc<Vec<u8>>,
    /// Where this HDU's data section starts within `data`.
    data_offset: usize,
    /// Data set through the `set_raw_images_*` methods.
    pending: Option<Vec<u8>>,
}

impl SliceImageHDU {
    pub(crate) fn new(header: Header, data: Arc<Vec<u8>>, data_offset: usize) -> Self {
        Self {
            header,
            data,
            data_offset,
            pending: None,
        }
    }

    /// An HDU holding nothing yet, for a FITS file being built from nothing.
    pub fn empty() -> Self {
        Self {
            header: Header::default(),
            data: Arc::new(Vec::new()),
            data_offset: 0,
            pending: Some(Vec::new()),
        }
    }

    /// This HDU's whole data section.
    pub(crate) fn data_bytes(&self) -> &[u8] {
        if let Some(pending) = &self.pending {
            return pending;
        }

        let len = self.header.data_bytes_len();
        self.data
            .get(self.data_offset..)
            .and_then(|data| data.get(..len))
            .unwrap_or_default()
    }

    fn image_bytes(&self, index: usize) -> &[u8] {
        let size = self.image_data_size() as usize;
        let start = size.saturating_mul(index);

        self.data_bytes()
            .get(start..)
            .and_then(|data| data.get(..size))
            .unwrap_or_default()
    }

    /// The bytes of one group of a random-groups HDU.
    fn group_bytes(
        &self,
        index: usize,
        len: usize,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let start = len * index;

        Ok(self
            .data_bytes()
            .get(start..)
            .and_then(|rest| rest.get(..len))
            .unwrap_or_default()
            .to_vec())
    }

    fn set_raw_images<T: Copy, const N: usize>(
        &mut self,
        bitpix: Bitpix,
        width: u32,
        height: u32,
        images: &[&[T]],
        to_be_bytes: impl Fn(T) -> [u8; N],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if images.is_empty() {
            return self.clear_images();
        }

        let pixels = (width as usize)
            .checked_mul(height as usize)
            .ok_or("Image dimensions overflow the address space")?;

        for (index, image) in images.iter().enumerate() {
            if image.len() != pixels {
                return Err(format!(
                    "Image {} has {} pixels, but a {}x{} image has {}",
                    index,
                    image.len(),
                    width,
                    height,
                    pixels
                )
                .into());
            }
        }

        let mut data = Vec::with_capacity(images.len() * pixels * N);
        for image in images {
            for value in image.iter() {
                data.extend_from_slice(&to_be_bytes(*value));
            }
        }

        self.header.set(Card::Bitpix {
            value: bitpix,
            comment: None,
        });

        let mut axes = vec![width as i64, height as i64];
        if images.len() > 1 {
            axes.push(images.len() as i64);
        }
        self.header.set(Card::NAxis {
            value: axes.len() as i64,
            comment: None,
        });
        // A leftover NAXISn from a larger array would contradict NAXIS, so every
        // one of them goes before the new set is written.
        self.header
            .remove_prefixed(crate::header::card_keys::PREFIX_NAXIS_N);

        for (index, length) in axes.iter().enumerate() {
            self.header.set(Card::NAxisN {
                index,
                value: *length,
                comment: None,
            });
        }

        self.pending = Some(data);

        Ok(())
    }
}

impl HDU for SliceImageHDU {
    fn header(&self) -> &Header {
        &self.header
    }

    fn header_mut(&mut self) -> &mut Header {
        &mut self.header
    }
}

impl ImageHDU for SliceImageHDU {
    fn image_count(&self) -> usize {
        // A random-groups HDU's data section is groups of parameters, not a run
        // of images; `read_group` is how that is read.
        if self.header.is_random_groups() {
            return 0;
        }

        self.header.image_plane_count()
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
        if index >= self.image_count() {
            return Ok(None);
        }

        let bytes = self.image_bytes(index).to_vec();

        Ok(Some(Image::from_data_and_header(bytes, &self.header)?))
    }

    /// How many groups this HDU holds, under the random-groups convention.
    fn group_count(&self) -> usize {
        if !self.header.is_random_groups() {
            return 0;
        }

        self.header.group_count().unwrap_or(0).max(0) as usize
    }

    fn read_group(&self, index: usize) -> Result<Option<Group>, Box<dyn Error + Send + Sync>> {
        if index >= ImageHDU::group_count(self) {
            return Ok(None);
        }

        let Some(bitpix) = self.header.bitpix() else {
            return Ok(None);
        };

        // Each group is its parameters followed by its array, both in the
        // array's own type.
        let parameters = self.header.pcount().unwrap_or(0).max(0) as usize;
        let len = (parameters + self.header.group_array_len()) * bitpix.byte_size();

        let bytes = self.group_bytes(index, len)?;

        Ok(crate::image::decode_group(&self.header, &bytes))
    }

    fn clear_images(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.header.set(Card::NAxis {
            value: 0,
            comment: None,
        });
        self.header
            .remove_prefixed(crate::header::card_keys::PREFIX_NAXIS_N);

        self.pending = Some(Vec::new());

        Ok(())
    }

    fn set_raw_images_u8(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[u8]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.set_raw_images(Bitpix::U8, width, height, images, u8::to_be_bytes)
    }

    fn set_raw_images_i16(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[i16]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.set_raw_images(Bitpix::I16, width, height, images, i16::to_be_bytes)
    }

    fn set_raw_images_i32(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[i32]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.set_raw_images(Bitpix::I32, width, height, images, i32::to_be_bytes)
    }

    fn set_raw_images_f32(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[f32]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.set_raw_images(Bitpix::F32, width, height, images, f32::to_be_bytes)
    }

    fn set_raw_images_f64(
        &mut self,
        width: u32,
        height: u32,
        images: &[&[f64]],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.set_raw_images(Bitpix::F64, width, height, images, f64::to_be_bytes)
    }

    #[cfg(feature = "tokio")]
    fn stream_normalised_image(
        &self,
        index: usize,
    ) -> Result<Option<NormalisedImageStream<'_>>, Box<dyn Error + Send + Sync>> {
        if index >= self.image_count() {
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

        // The whole image is already in memory, so there is nothing to read
        // incrementally; the stream exists to match the file-backed API.
        let pixels: Vec<_> = self
            .image_bytes(index)
            .chunks_exact(pixel_len)
            .enumerate()
            .filter_map(|(index, raw)| {
                let value = bitpix.read_be(raw)?;
                let x = (index as u64 % width as u64) as u32;
                let y = (index as u64 / width as u64) as u32;

                Some((x, y, normalizer.normalize(value)))
            })
            .collect();

        Ok(Some(stream::iter(pixels).boxed()))
    }

    fn image_data_size(&self) -> u64 {
        let Some(bitpix) = self.header.bitpix() else {
            return 0;
        };

        self.images_width() as u64 * self.images_height() as u64 * bitpix.byte_size() as u64
    }
}
