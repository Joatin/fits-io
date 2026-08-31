//! Writing an image as a tile-compressed table.
//!
//! This is the other half of the tiled image convention: an image is cut into
//! tiles, each tile is compressed on its own, and the results become the rows of
//! a binary table whose header describes, in keywords beginning `Z`, the image
//! it stands for. A reader that knows nothing of compression sees a table; one
//! that does sees the image.

use crate::header::card_keys;
use crate::header::{Bitpix, Header};
use crate::image::compression::dither::{Dither, Quantization};
use crate::image::compression::rice::BytesPerValue;
use crate::image::compression::{dither, hcompress, plio, rice};
use std::error::Error;

/// Which algorithm the tiles are compressed with.
///
/// The convention defines several, and they suit different data. Rice is the
/// usual choice for astronomical images: it is fast and, on data where each
/// pixel is close to the one before it, it is the smallest of these. Gzip
/// compresses anything at all, including floating point values that have not
/// been quantised, which is what makes it the one lossless choice for a
/// floating point image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compression {
    /// `RICE_1`: the differences between neighbouring pixels, coded so that
    /// small ones are short. Integers only — a floating point image has to be
    /// quantised first.
    #[default]
    Rice,
    /// `GZIP_1`: deflate over the tile's bytes as they stand.
    Gzip,
    /// `GZIP_2`: deflate after gathering the first byte of every value, then
    /// the second, and so on, which usually compresses better because the high
    /// bytes of neighbouring values are alike.
    ShuffledGzip,
    /// `HCOMPRESS_1`: an image transform that gathers each two by two block
    /// into a sum and three differences, coded as a quadtree. It compresses
    /// smooth images better than Rice, and at a `scale` above one it does so by
    /// throwing away the low bits of each coefficient — which is lossy, and
    /// which [`ImageHDU::read_image`] can smooth back over.
    ///
    /// A scale of zero or one keeps every bit. Tiles are two-dimensional, so
    /// this cannot be used with a tile that reaches along a third axis.
    ///
    /// [`ImageHDU::read_image`]: crate::hdu::ImageHDU::read_image
    Hcompress {
        /// What the transform's coefficients are divided by.
        scale: i64,
    },
    /// `PLIO_1`: run-length coding for the integer masks IRAF writes, where a
    /// tile is mostly long runs of the same small number. Values must be
    /// non-negative and no wider than twenty-seven bits.
    Plio,
    /// `NOCOMPRESS`: the tiles are stored as they are. Useful for writing a
    /// file in the tiled layout without paying to compress it.
    None,
}

impl Compression {
    /// The name a ZCMPTYPE card writes this algorithm under.
    pub fn card_value(self) -> &'static str {
        match self {
            Compression::Rice => "RICE_1",
            Compression::Gzip => "GZIP_1",
            Compression::ShuffledGzip => "GZIP_2",
            Compression::Hcompress { .. } => "HCOMPRESS_1",
            Compression::Plio => "PLIO_1",
            Compression::None => "NOCOMPRESS",
        }
    }

    /// Whether this algorithm works on integers rather than on bytes.
    fn needs_integers(self) -> bool {
        matches!(
            self,
            Compression::Rice | Compression::Hcompress { .. } | Compression::Plio
        )
    }

    /// How wide each element of the column holding a tile is.
    ///
    /// PLIO's instructions are sixteen bit words, and the column holds them as
    /// such rather than as the bytes they are made of.
    fn element_bytes(self) -> usize {
        match self {
            Compression::Plio => 2,
            _ => 1,
        }
    }

    /// The TFORMn of the column holding a tile.
    fn column_format(self) -> &'static str {
        match self {
            Compression::Plio => "1PI",
            _ => "1PB",
        }
    }
}

/// How a floating point image's values are turned into the integers a
/// compressor can work on.
///
/// Quantising is what makes a floating point image compress at all well, and it
/// is lossy: what comes back is within one step of what went in. The step is the
/// choice being made here.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Quantize {
    /// Do not quantise. The values are compressed as they stand, which only
    /// [`Compression::Gzip`], [`Compression::ShuffledGzip`] and
    /// [`Compression::None`] can do, and nothing is lost.
    #[default]
    Lossless,
    /// Quantise in steps of this many units of the image's own values.
    Step(f64),
    /// Quantise in steps of the tile's own estimated noise divided by this.
    ///
    /// Four is the usual choice: it keeps the quantisation step well below the
    /// noise already in the data, so nothing measurable is lost, while throwing
    /// away the low bits that are noise anyway and would otherwise compress
    /// terribly.
    ///
    /// The noise is estimated from the median absolute third-order difference
    /// between neighbouring pixels, which is what the convention's own reference
    /// implementation does. That implementation takes the smallest of three such
    /// estimates rather than this one alone, so it may settle on a slightly
    /// different step for the same tile; either is a fair reading of "the noise
    /// in this tile".
    NoiseLevel(f64),
}

/// Everything about how an image is to be compressed.
///
/// ```
/// use fits_io::image::compression::{Compression, CompressionOptions, Quantize};
///
/// // Rice coding, in tiles of 100 by 100 pixels.
/// let options = CompressionOptions::new(Compression::Rice).with_tile_size(&[100, 100]);
///
/// // A floating point image, quantised to a quarter of its own noise.
/// let lossy = CompressionOptions::new(Compression::Rice)
///     .with_quantization(Quantize::NoiseLevel(4.0));
///
/// // HCOMPRESS, throwing away the low bits of the transform's coefficients.
/// let smooth = CompressionOptions::new(Compression::Hcompress { scale: 16 });
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CompressionOptions {
    compression: Compression,
    tile: Option<Vec<u32>>,
    quantize: Quantize,
    quantization: Quantization,
    seed: i64,
    block: usize,
}

impl Default for CompressionOptions {
    fn default() -> Self {
        Self::new(Compression::default())
    }
}

impl CompressionOptions {
    /// Compress with `compression`, in the convention's default tiles — one row
    /// of the image each — and without quantising.
    pub fn new(compression: Compression) -> Self {
        Self {
            compression,
            tile: None,
            quantize: Quantize::Lossless,
            // Dithering costs nothing and is what keeps quantisation from
            // laying a pattern over a smooth background, so it is on wherever
            // quantisation is.
            quantization: Quantization::SubtractiveDither1,
            seed: 1,
            block: 32,
        }
    }

    /// Cut the image into tiles of this shape, fastest axis first.
    ///
    /// A shape shorter than the image's axes has the remaining axes tiled one
    /// plane at a time. Bigger tiles compress a little better and cost more to
    /// read a small part of the image from.
    #[must_use]
    pub fn with_tile_size(mut self, tile: &[u32]) -> Self {
        self.tile = Some(tile.to_vec());
        self
    }

    /// Quantise a floating point image before compressing it.
    ///
    /// This has no effect on an image that is already integers.
    #[must_use]
    pub fn with_quantization(mut self, quantize: Quantize) -> Self {
        self.quantize = quantize;
        self
    }

    /// Which dithering the quantisation uses.
    ///
    /// [`Quantization::SubtractiveDither1`] unless this says otherwise, which is
    /// what a floating point image usually wants.
    #[must_use]
    pub fn with_dithering(mut self, quantization: Quantization) -> Self {
        self.quantization = quantization;
        self
    }

    /// Where in the dithering sequence the first tile starts, from 1 to 10000.
    ///
    /// Two files written from the same data with the same seed come out
    /// identical, which is what makes a compressed file reproducible.
    #[must_use]
    pub fn with_dither_seed(mut self, seed: i64) -> Self {
        self.seed = seed.rem_euclid(dither::SEQUENCE_LENGTH as i64).max(1);
        self
    }

    /// How many values Rice coding fits its split point to at a time.
    ///
    /// Thirty-two unless this says otherwise, which is what the convention's
    /// reference implementation uses.
    #[must_use]
    pub fn with_block_size(mut self, block: usize) -> Self {
        self.block = block.max(1);
        self
    }

    /// The algorithm the tiles are compressed with.
    pub fn compression(&self) -> Compression {
        self.compression
    }
}

/// The value standing for a pixel the image does not define, and the ten values
/// above it that the convention reserves alongside it.
const NULL_VALUE: i64 = -2147483647;
const RESERVED_VALUES: f64 = 10.0;

/// The columns a compressed table is written with.
const COMPRESSED_DATA: &str = "COMPRESSED_DATA";
const SCALE: &str = "ZSCALE";
const ZERO: &str = "ZZERO";

/// Compresses an image into the header and data of the table that stands for it.
///
/// `header` describes the image as it is now, and `data` is its pixels as they
/// sit in the file. What comes back is the header and data section of a binary
/// table extension carrying the same image, compressed.
pub(crate) fn compress(
    header: &Header,
    data: &[u8],
    options: &CompressionOptions,
) -> Result<(Header, Vec<u8>), Box<dyn Error + Send + Sync>> {
    let bitpix = header
        .bitpix()
        .ok_or("An image needs a BITPIX card before it can be compressed")?;

    let shape = shape_of(header);
    if shape.is_empty() {
        return Err("An image with no axes has nothing to compress".into());
    }

    let tile = tile_shape(options, &shape);
    let quantizing = matches!(bitpix, Bitpix::F32 | Bitpix::F64)
        && !matches!(options.quantize, Quantize::Lossless);

    if bitpix.is_floating() && options.compression.needs_integers() && !quantizing {
        return Err(format!(
            "{} compresses integers, and this image holds floating point values. Either quantise \
             it, which loses the low bits of every pixel, or compress it with GZIP_1, which does \
             not.",
            options.compression.card_value()
        )
        .into());
    }

    let pixels = read_pixels(data, bitpix, &shape)?;

    // A quantised tile holds 32-bit integers whatever the image's own type is.
    let stored = if quantizing { Bitpix::I32 } else { bitpix };

    let tiles: Vec<usize> = shape
        .iter()
        .zip(&tile)
        .map(|(length, tile)| length.div_ceil(*tile))
        .collect();
    let tile_count: usize = tiles.iter().product();

    let mut rows = Vec::new();
    let mut heap = Vec::new();
    let mut any_blank = false;

    let elements = options.compression.element_bytes();

    for index in 0..tile_count {
        let (values, extent) = gather(&pixels, &shape, &tile, &tiles, index);

        let (integers, scale, zero) = if quantizing {
            let (integers, scale, zero) = quantize_tile(&values, options, index);
            any_blank |= values.iter().any(|value| !value.is_finite());
            (integers, Some(scale), Some(zero))
        } else {
            (
                values.iter().map(|value| *value as i64).collect(),
                None,
                None,
            )
        };

        let compressed = encode(&integers, &values, stored, &extent, options)?;

        // Each row carries a descriptor saying how long its array is and where
        // in the heap it sits; the bytes themselves all go in the heap. The
        // length is counted in the column's own elements, which are not always
        // bytes.
        rows.extend_from_slice(&((compressed.len() / elements) as u32).to_be_bytes());
        rows.extend_from_slice(&(heap.len() as u32).to_be_bytes());
        heap.extend_from_slice(&compressed);

        if let (Some(scale), Some(zero)) = (scale, zero) {
            rows.extend_from_slice(&scale.to_be_bytes());
            rows.extend_from_slice(&zero.to_be_bytes());
        }
    }

    let row_bytes = if quantizing { 8 + 16 } else { 8 };

    let mut table = rows;
    table.extend_from_slice(&heap);

    let compressed_header = compressed_header(
        header,
        bitpix,
        &shape,
        &tile,
        options,
        quantizing,
        any_blank,
        tile_count,
        row_bytes,
        heap.len(),
    )?;

    Ok((compressed_header, table))
}

/// The pixels of an image, as the numbers they stand for.
fn read_pixels(
    data: &[u8],
    bitpix: Bitpix,
    shape: &[usize],
) -> Result<Vec<f64>, Box<dyn Error + Send + Sync>> {
    let count: usize = shape.iter().product();
    let width = bitpix.byte_size();

    if data.len() < count * width {
        return Err(format!(
            "This image says it holds {} pixels of {} bytes, and its data section is {} bytes",
            count,
            width,
            data.len()
        )
        .into());
    }

    Ok(data[..count * width]
        .chunks_exact(width)
        .filter_map(|raw| bitpix.read_be(raw))
        .collect())
}

/// The image's shape, fastest axis first.
fn shape_of(header: &Header) -> Vec<usize> {
    let axes = header.naxis().unwrap_or(0).max(0) as usize;

    (0..axes)
        .map(|axis| header.naxis_n(axis).unwrap_or(0).max(0) as usize)
        .collect()
}

/// How far a tile reaches along each axis.
fn tile_shape(options: &CompressionOptions, shape: &[usize]) -> Vec<usize> {
    (0..shape.len())
        .map(|axis| {
            let asked = match &options.tile {
                Some(tile) => tile.get(axis).map(|size| *size as usize),
                // The convention's default is one row of the image.
                None => Some(if axis == 0 { shape[0] } else { 1 }),
            };

            asked.unwrap_or(1).clamp(1, shape[axis].max(1))
        })
        .collect()
}

/// The values of one tile, in the order the tile stores them.
fn gather(
    pixels: &[f64],
    shape: &[usize],
    tile: &[usize],
    tiles: &[usize],
    index: usize,
) -> (Vec<f64>, Vec<usize>) {
    // Where this tile starts along each axis, and how far it reaches before the
    // edge of the image cuts it short.
    let mut origin = vec![0_usize; shape.len()];
    let mut extent = vec![0_usize; shape.len()];

    let mut rest = index;
    for axis in 0..shape.len() {
        origin[axis] = (rest % tiles[axis]) * tile[axis];
        rest /= tiles[axis];
        extent[axis] = tile[axis].min(shape[axis] - origin[axis]);
    }

    let run = extent[0];
    let runs: usize = extent.iter().skip(1).product();
    let mut values = Vec::with_capacity(run * runs);

    let mut within = vec![0_usize; shape.len()];

    for index in 0..runs {
        let mut rest = index;
        for axis in 1..shape.len() {
            within[axis] = rest % extent[axis];
            rest /= extent[axis];
        }

        let mut at = origin[0];
        let mut stride = shape[0];
        for axis in 1..shape.len() {
            at += (origin[axis] + within[axis]) * stride;
            stride *= shape[axis];
        }

        values.extend_from_slice(&pixels[at..at + run]);
    }

    (values, extent)
}

/// Turns one tile's values into the integers a compressor works on, and says
/// what scales them back.
fn quantize_tile(
    values: &[f64],
    options: &CompressionOptions,
    tile: usize,
) -> (Vec<i64>, f64, f64) {
    let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();

    let step = match options.quantize {
        Quantize::Step(step) => step.abs(),
        Quantize::NoiseLevel(level) => noise(&finite) / level.max(f64::MIN_POSITIVE),
        // The caller has already established that this tile is being quantised.
        Quantize::Lossless => 0.0,
    };

    // A tile with no spread at all, or one this crate can find no noise in,
    // is stored in steps of one, which for such a tile loses nothing.
    let step = if step > 0.0 && step.is_finite() {
        step
    } else {
        1.0
    };

    let minimum = finite.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let zero = if !minimum.is_finite() {
        0.0
    } else if finite.len() < values.len()
        || options.quantization == Quantization::SubtractiveDither2
    {
        // Room has to be left below the smallest value for the values the
        // convention reserves, of which the blank value is one.
        minimum - step * (NULL_VALUE as f64 + RESERVED_VALUES)
    } else {
        // Otherwise the smallest value sits at the bottom of the range, on a
        // whole number of steps — so that compressing a file that has already
        // been through this comes out the same way twice.
        let factor = (minimum / step + 0.5).floor();
        factor * step
    };

    let blank = (finite.len() < values.len()).then_some(NULL_VALUE);
    let integers = dither::quantize(
        values,
        step,
        zero,
        options.quantization,
        blank,
        Dither::for_tile(options.seed, tile),
    );

    let _ = maximum;

    (integers, step, zero)
}

/// Estimates the noise in a tile from how much neighbouring pixels differ.
///
/// A third-order difference — twice a pixel less its two neighbours — cancels
/// any smooth gradient the image has, so what is left of it is noise. Taking the
/// median of those rather than their mean keeps a handful of stars from being
/// mistaken for noise, and the constant turns that median into the standard
/// deviation of a normal distribution that would produce it.
fn noise(values: &[f64]) -> f64 {
    /// Turns the median absolute third-order difference into a standard
    /// deviation.
    const MEDIAN_TO_SIGMA: f64 = 0.6052697;

    if values.len() < 3 {
        return 0.0;
    }

    let mut differences: Vec<f64> = values
        .windows(3)
        .map(|window| (2.0 * window[1] - window[0] - window[2]).abs())
        .collect();

    differences.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let middle = differences.len() / 2;
    let median = if differences.len().is_multiple_of(2) {
        (differences[middle - 1] + differences[middle]) / 2.0
    } else {
        differences[middle]
    };

    MEDIAN_TO_SIGMA * median
}

/// Compresses one tile.
///
/// `integers` is the tile as whole numbers, for the algorithms that work on
/// those, and `values` the same tile as it stands, for the ones that work on
/// bytes.
fn encode(
    integers: &[i64],
    values: &[f64],
    stored: Bitpix,
    extent: &[usize],
    options: &CompressionOptions,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    match options.compression {
        Compression::Rice => {
            let width = BytesPerValue::from_count(stored.byte_size() as i64)?;
            Ok(rice::compress(integers, width, options.block))
        }
        Compression::Hcompress { scale } => {
            // The transform is two-dimensional, and a tile that reaches along a
            // third axis is not a plane for it to work on.
            if extent.iter().skip(2).any(|length| *length > 1) {
                return Err(format!(
                    "HCOMPRESS compresses a plane at a time, and this tile is {:?}",
                    extent
                )
                .into());
            }

            let columns = extent.first().copied().unwrap_or(0);
            let rows = extent.get(1).copied().unwrap_or(1);

            // The transform's own axis order is the other way round from the
            // image's: its first dimension is the slow one.
            hcompress::compress(integers, rows, columns, scale)
        }
        Compression::Plio => {
            let words = plio::compress(integers)?;

            Ok(words.iter().flat_map(|word| word.to_be_bytes()).collect())
        }
        Compression::None => Ok(to_be_bytes(integers, values, stored)),
        Compression::Gzip => gzip(&to_be_bytes(integers, values, stored)),
        Compression::ShuffledGzip => {
            let bytes = to_be_bytes(integers, values, stored);
            gzip(&shuffle(&bytes, stored.byte_size()))
        }
    }
}

/// A tile as the bytes it is stored in.
fn to_be_bytes(integers: &[i64], values: &[f64], stored: Bitpix) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(integers.len() * stored.byte_size());

    match stored {
        Bitpix::U8 => bytes.extend(integers.iter().map(|value| *value as u8)),
        Bitpix::I16 => {
            for value in integers {
                bytes.extend_from_slice(&(*value as i16).to_be_bytes());
            }
        }
        Bitpix::I32 => {
            for value in integers {
                bytes.extend_from_slice(&(*value as i32).to_be_bytes());
            }
        }
        // A floating point tile that was not quantised keeps its own values,
        // NaNs and all, rather than the integers they do not have.
        Bitpix::F32 => {
            for value in values {
                bytes.extend_from_slice(&(*value as f32).to_be_bytes());
            }
        }
        Bitpix::F64 => {
            for value in values {
                bytes.extend_from_slice(&value.to_be_bytes());
            }
        }
    }

    bytes
}

/// Gathers the first byte of every value, then the second, and so on.
fn shuffle(bytes: &[u8], width: usize) -> Vec<u8> {
    if width <= 1 {
        return bytes.to_vec();
    }

    let count = bytes.len() / width;
    let mut out = vec![0_u8; count * width];

    for byte in 0..width {
        for value in 0..count {
            out[byte * count + value] = bytes[value * width + byte];
        }
    }

    out
}

#[cfg(feature = "gzip")]
fn gzip(bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    use std::io::Write;

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes)?;

    Ok(encoder.finish()?)
}

#[cfg(not(feature = "gzip"))]
fn gzip(_bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    Err("Compressing with gzip needs the `gzip` feature".into())
}

/// Builds the header of the table an image is compressed into.
#[allow(clippy::too_many_arguments)]
fn compressed_header(
    header: &Header,
    bitpix: Bitpix,
    shape: &[usize],
    tile: &[usize],
    options: &CompressionOptions,
    quantizing: bool,
    any_blank: bool,
    rows: usize,
    row_bytes: usize,
    heap: usize,
) -> Result<Header, Box<dyn Error + Send + Sync>> {
    let mut out = header.clone();

    // The image's own structural cards are replaced by the table's, and its
    // shape moves into the Z keywords.
    out.remove_card(card_keys::NAXIS);
    out.remove_prefixed(card_keys::PREFIX_NAXIS_N);

    out.set_card(card_keys::BITPIX, 8_i64)?;
    out.set_naxis_n(0, row_bytes as i64)?;
    out.set_naxis_n(1, rows as i64)?;
    out.set_card(card_keys::NAXIS, 2_i64)?;
    out.set_card(card_keys::PCOUNT, heap as i64)?;
    out.set_card(card_keys::GCOUNT, 1_i64)?;

    let tiles = options.compression.column_format();

    let columns: Vec<(&str, &str)> = if quantizing {
        vec![(COMPRESSED_DATA, tiles), (SCALE, "1D"), (ZERO, "1D")]
    } else {
        vec![(COMPRESSED_DATA, tiles)]
    };

    out.set_card(card_keys::TFIELDS, columns.len() as i64)?;
    for (index, (name, format)) in columns.iter().enumerate() {
        out.set_card(
            &format!("{}{}", card_keys::PREFIX_TTYPE_N, index + 1),
            *name,
        )?;
        out.set_card(
            &format!("{}{}", card_keys::PREFIX_TFORM_N, index + 1),
            *format,
        )?;
    }

    out.set_card(card_keys::ZIMAGE, true)?;
    out.set_card(card_keys::ZBITPIX, i64::from(bitpix))?;
    out.set_card(card_keys::ZNAXIS, shape.len() as i64)?;

    for (axis, length) in shape.iter().enumerate() {
        out.set_card(&format!("ZNAXIS{}", axis + 1), *length as i64)?;
        out.set_card(&format!("ZTILE{}", axis + 1), tile[axis] as i64)?;
    }

    out.set_card(card_keys::ZCMPTYPE, options.compression.card_value())?;

    // The algorithms take their settings as name and value pairs.
    let mut parameters: Vec<(&str, i64)> = Vec::new();
    match options.compression {
        Compression::Rice => {
            parameters.push(("BLOCKSIZE", options.block as i64));
            parameters.push((
                "BYTEPIX",
                if quantizing {
                    4
                } else {
                    bitpix.byte_size() as i64
                },
            ));
        }
        Compression::Hcompress { scale } => {
            parameters.push(("SCALE", scale));
            // Whether to smooth is the reader's choice, and a file that asks
            // for it says so here.
            parameters.push(("SMOOTH", 0));
        }
        _ => {}
    }

    for (index, (name, value)) in parameters.iter().enumerate() {
        out.set_card(&format!("ZNAME{}", index + 1), *name)?;
        out.set_card(&format!("ZVAL{}", index + 1), *value)?;
    }

    if quantizing {
        out.set_card(card_keys::ZQUANTIZ, options.quantization.card_value())?;
        out.set_card(card_keys::ZDITHER0, options.seed)?;

        if any_blank {
            out.set_card(card_keys::ZBLANK, NULL_VALUE)?;
        }
    }

    Ok(out)
}

/// The BITPIX of a header, as the number the card holds.
impl Bitpix {
    /// Whether this type holds floating point values.
    pub(crate) fn is_floating(self) -> bool {
        matches!(self, Bitpix::F32 | Bitpix::F64)
    }
}

#[cfg(test)]
mod tests {
    use super::{Compression, CompressionOptions, Quantize, compress, noise, shuffle};
    use crate::header::{Bitpix, Header};

    /// A header for a `width` by `height` image of `bitpix`.
    fn header(bitpix: Bitpix, width: usize, height: usize) -> Header {
        let mut header = Header::default();

        header.set_card("BITPIX", i64::from(bitpix)).unwrap();
        header.set_card("NAXIS", 2_i64).unwrap();
        header.set_naxis_n(0, width as i64).unwrap();
        header.set_naxis_n(1, height as i64).unwrap();

        header
    }

    fn i16_data(values: &[i16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    #[test]
    fn a_compressed_header_describes_both_the_table_and_the_image() {
        let values: Vec<i16> = (0..64).collect();
        let (compressed, _) = compress(
            &header(Bitpix::I16, 8, 8),
            &i16_data(&values),
            &CompressionOptions::new(Compression::Rice),
        )
        .expect("an image that can be compressed");

        // The table it is stored as.
        assert_eq!(compressed.bitpix(), Some(Bitpix::U8));
        assert_eq!(compressed.naxis(), Some(2));
        assert_eq!(compressed.table_fields(), Some(1));

        // And the image it stands for.
        assert!(compressed.is_compressed_image());
        assert_eq!(compressed.compressed_bitpix(), Some(Bitpix::I16));
        assert_eq!(compressed.compressed_naxis(), Some(2));
        assert_eq!(compressed.compressed_naxis_n(0), Some(8));
        assert_eq!(compressed.compressed_naxis_n(1), Some(8));
        assert_eq!(compressed.compression_type(), Some("RICE_1"));
        assert_eq!(compressed.compression_parameter("BYTEPIX"), Some(2));
    }

    #[test]
    fn the_default_tile_is_one_row_of_the_image() {
        let values: Vec<i16> = (0..64).collect();
        let (compressed, _) = compress(
            &header(Bitpix::I16, 8, 8),
            &i16_data(&values),
            &CompressionOptions::new(Compression::Rice),
        )
        .expect("an image that can be compressed");

        assert_eq!(compressed.compressed_tile_size(0), 8);
        assert_eq!(compressed.compressed_tile_size(1), 1);
        // One row per tile means one table row per image row.
        assert_eq!(compressed.naxis_n(1), Some(8));
    }

    #[test]
    fn a_tile_size_larger_than_the_image_is_cut_down_to_it() {
        let values: Vec<i16> = (0..64).collect();
        let (compressed, _) = compress(
            &header(Bitpix::I16, 8, 8),
            &i16_data(&values),
            &CompressionOptions::new(Compression::Rice).with_tile_size(&[1000, 1000]),
        )
        .expect("an image that can be compressed");

        assert_eq!(compressed.compressed_tile_size(0), 8);
        assert_eq!(compressed.compressed_tile_size(1), 8);
        assert_eq!(compressed.naxis_n(1), Some(1));
    }

    #[test]
    fn a_floating_point_image_cannot_be_rice_coded_without_being_quantised() {
        let data: Vec<u8> = (0..64).flat_map(|i| (i as f32).to_be_bytes()).collect();

        let error = compress(
            &header(Bitpix::F32, 8, 8),
            &data,
            &CompressionOptions::new(Compression::Rice),
        )
        .expect_err("Rice coding works on integers");

        assert!(error.to_string().contains("quantise"), "got: {error}");
    }

    #[test]
    fn a_quantised_image_says_how_it_was_quantised() {
        let data: Vec<u8> = (0..64)
            .flat_map(|i| (i as f32 * 0.5).to_be_bytes())
            .collect();

        let (compressed, _) = compress(
            &header(Bitpix::F32, 8, 8),
            &data,
            &CompressionOptions::new(Compression::Rice)
                .with_quantization(Quantize::Step(0.01))
                .with_dither_seed(42),
        )
        .expect("a quantised image compresses");

        assert_eq!(
            compressed.quantization_method(),
            Some("SUBTRACTIVE_DITHER_1")
        );
        assert_eq!(compressed.dither_seed(), Some(42));
        assert_eq!(compressed.table_fields(), Some(3));
        assert_eq!(compressed.compression_parameter("BYTEPIX"), Some(4));

        // The image is still a floating point image; only its tiles are
        // integers.
        assert_eq!(compressed.compressed_bitpix(), Some(Bitpix::F32));
        assert_eq!(
            compressed.card("TTYPE2").map(|v| v.value_to_string()),
            Some("ZSCALE".to_string())
        );
    }

    #[test]
    fn the_noise_estimate_follows_the_noise() {
        // Pixels drawn either side of a line: the estimate should follow how
        // far they are drawn from it, and should not be fooled by the line.
        let quiet: Vec<f64> = (0..200)
            .map(|i| i as f64 + if i % 2 == 0 { 0.1 } else { -0.1 })
            .collect();
        let loud: Vec<f64> = (0..200)
            .map(|i| i as f64 + if i % 2 == 0 { 5.0 } else { -5.0 })
            .collect();

        assert!(noise(&quiet) > 0.0);
        assert!(
            noise(&loud) > 10.0 * noise(&quiet),
            "{} vs {}",
            noise(&loud),
            noise(&quiet)
        );

        // A perfectly smooth ramp has no noise in it at all.
        let ramp: Vec<f64> = (0..200).map(|i| i as f64 * 3.0).collect();
        assert_eq!(noise(&ramp), 0.0);
    }

    #[test]
    fn shuffling_gathers_each_byte_of_every_value_together() {
        // Two 16-bit values: the high bytes first, then the low ones.
        assert_eq!(
            shuffle(&[0x12, 0x34, 0x56, 0x78], 2),
            vec![0x12, 0x56, 0x34, 0x78]
        );

        // Single bytes have nothing to gather.
        assert_eq!(shuffle(&[1, 2, 3], 1), vec![1, 2, 3]);
    }
}
