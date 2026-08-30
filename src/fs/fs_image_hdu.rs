use crate::fs::open_fits_file::open_fits_file;
#[cfg(feature = "tokio")]
use crate::hdu::NormalisedImageStream;
use crate::hdu::{HDU, ImageHDU};
use crate::header::card::Card;
use crate::header::{BayerPattern, Bitpix, Header, ImageType};
#[cfg(feature = "tokio")]
use crate::image::Normalizer;
use crate::image::{Group, Image};
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
    /// Data set through [`ImageHDU::set_raw_images_u8`] and its siblings, which
    /// stands in for whatever the file holds until it is saved.
    pending: Option<Vec<u8>>,
}

impl FsImageHDU {
    pub(crate) fn new_primary(path: &Path, header: Header) -> Self {
        Self {
            header,
            hdu_offset: 0,
            path: path.to_path_buf(),
            pending: None,
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
            pending: None,
        })
    }

    /// An image extension holding nothing yet, for a file being built up.
    pub fn new_empty_extension(path: &Path) -> Self {
        Self {
            header: Header::default(),
            hdu_offset: 0,
            path: path.to_path_buf(),
            pending: Some(Vec::new()),
        }
    }

    fn is_image_index_valid(&self, index: usize) -> bool {
        index < self.image_count()
    }

    /// This HDU's whole data section, from memory if it has been written to and
    /// from the file otherwise.
    pub(crate) fn data_bytes(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        if let Some(pending) = &self.pending {
            return Ok(pending.clone());
        }

        let len = self.header.data_bytes_len() as u64;
        if len == 0 {
            return Ok(Vec::new());
        }

        let mut reader = open_fits_file(&self.path)?;
        reader.seek(SeekFrom::Start(
            self.hdu_offset + self.header.bytes_len() as u64,
        ))?;

        Ok(read_bytes(&mut reader, len)?)
    }

    /// The bytes of one image, from memory if this HDU has been written to and
    /// from the file otherwise.
    fn image_bytes(&self, index: usize) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let size = self.image_data_size();
        let offset = size * index as u64;

        if let Some(pending) = &self.pending {
            let start = usize::try_from(offset).unwrap_or(usize::MAX);
            let end = start.saturating_add(usize::try_from(size).unwrap_or(usize::MAX));
            return Ok(pending.get(start..end).unwrap_or_default().to_vec());
        }

        let mut reader = open_fits_file(&self.path)?;
        reader.seek(SeekFrom::Start(
            self.hdu_offset + self.header.bytes_len() as u64 + offset,
        ))?;

        Ok(read_bytes(&mut reader, size)?)
    }

    /// The bytes of one group of a random-groups HDU.
    fn group_bytes(
        &self,
        index: usize,
        len: usize,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let offset = (len * index) as u64;

        if let Some(pending) = &self.pending {
            let start = offset as usize;
            return Ok(pending
                .get(start..)
                .and_then(|rest| rest.get(..len))
                .unwrap_or_default()
                .to_vec());
        }

        let mut reader = open_fits_file(&self.path)?;
        reader.seek(SeekFrom::Start(
            self.hdu_offset + self.header.bytes_len() as u64 + offset,
        ))?;

        Ok(read_bytes(&mut reader, len as u64)?)
    }

    /// Stores `images` as this HDU's data and brings the header into line with
    /// it.
    ///
    /// Every image must be `width` by `height`; a ragged set would produce a
    /// data section that no longer matches the NAXISn cards describing it.
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
        self.set_axes(&[width as i64, height as i64], images.len());

        self.pending = Some(data);

        Ok(())
    }

    /// Writes the NAXIS and NAXISn cards for `axes`, plus a third axis when the
    /// HDU holds more than one image.
    fn set_axes(&mut self, axes: &[i64], images: usize) {
        let mut axes = axes.to_vec();
        if images > 1 {
            axes.push(images as i64);
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
    /// The first two axes are the image itself and every axis beyond them
    /// multiplies the count, so a 2-axis HDU holds one image, a 3-axis HDU holds
    /// NAXIS3, and a 4-axis HDU holds NAXIS3 x NAXIS4. Anything with fewer than
    /// two axes — a header-only primary HDU, for instance — holds none.
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
        if !self.is_image_index_valid(index) {
            return Ok(None);
        }

        let bytes = self.image_bytes(index)?;

        let image = Image::from_data_and_header(bytes, &self.header)?;

        Ok(Some(image))
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

        // Data written but not yet saved lives in memory, and streaming it from
        // the file would hand back what the image used to be.
        let blocks = if self.pending.is_some() {
            stream::once(ready(self.image_bytes(index)?)).boxed()
        } else {
            let mut reader = open_fits_file(&self.path)?;
            reader.seek(SeekFrom::Start(
                self.hdu_offset
                    + self.header.bytes_len() as u64
                    + (self.image_data_size() * index as u64),
            ))?;

            read_bytes_async(reader, self.image_data_size())
        };

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
