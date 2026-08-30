//! Reading images that are stored compressed, tile by tile, inside a table.
//!
//! The tiled image convention cuts an image into rectangular tiles, compresses
//! each one, and stores the results as the rows of a binary table. The table's
//! header describes the image it stands for with keywords beginning `Z`.

pub(crate) mod hcompress;
pub(crate) mod plio;
pub(crate) mod rice;

use crate::bin_table::{BinTable, Value};
use crate::header::{Bitpix, Header};
use crate::image::Image;
use rice::BytesPerValue;
use std::error::Error;

/// The column holding each tile's compressed bytes.
const COMPRESSED_DATA: &str = "COMPRESSED_DATA";
/// The column holding a tile the coder could not usefully compress.
const UNCOMPRESSED_DATA: &str = "UNCOMPRESSED_DATA";
/// The column holding a tile that fell back to gzip.
const GZIP_COMPRESSED_DATA: &str = "GZIP_COMPRESSED_DATA";
/// The columns scaling a quantised floating point tile back to its own units.
const SCALE: &str = "ZSCALE";
const ZERO: &str = "ZZERO";

/// Decompresses the image a table stands for.
pub(crate) fn read_image(
    header: &Header,
    table: &BinTable,
) -> Result<Image, Box<dyn Error + Send + Sync>> {
    let bitpix = header
        .compressed_bitpix()
        .ok_or("A compressed image needs a ZBITPIX card to say what its values are")?;

    if header.compressed_naxis() != Some(2) {
        return Err(format!(
            "Only two-dimensional compressed images are supported, but ZNAXIS is {:?}",
            header.compressed_naxis()
        )
        .into());
    }

    let width = size(header.compressed_naxis_n(0), "ZNAXIS1")?;
    let height = size(header.compressed_naxis_n(1), "ZNAXIS2")?;
    let tile_width = size(Some(header.compressed_tile_size(0)), "ZTILE1")?.max(1);
    let tile_height = size(Some(header.compressed_tile_size(1)), "ZTILE2")?.max(1);

    let across = width.div_ceil(tile_width);

    // The image is assembled in the type it will be read back as, so that the
    // ordinary image path can take it from here.
    let mut pixels = vec![0_f64; width * height];

    for (index, row) in table.rows().enumerate() {
        let tile_x = (index % across) * tile_width;
        let tile_y = (index / across) * tile_height;

        if tile_y >= height {
            break;
        }

        // Tiles at the right and bottom edges are cut short by the image's size.
        let this_width = tile_width.min(width - tile_x);
        let this_height = tile_height.min(height - tile_y);
        let count = this_width * this_height;

        let values = decode_tile(header, &row, count, bitpix)?;

        for (offset, value) in values.iter().enumerate().take(count) {
            let x = tile_x + offset % this_width;
            let y = tile_y + offset / this_width;

            pixels[y * width + x] = *value;
        }
    }

    // The header of the image this stands for, so that BZERO, BSCALE, DATAMIN
    // and the rest are read exactly as they would be for an ordinary image.
    let image_header = header.uncompressed();

    Image::from_data_and_header(to_be_bytes(&pixels, bitpix), &image_header)
}

fn size(value: Option<i64>, key: &str) -> Result<usize, Box<dyn Error + Send + Sync>> {
    let value = value.ok_or_else(|| format!("A compressed image needs a {} card", key))?;

    usize::try_from(value)
        .map_err(|_| format!("{} must not be negative, but was {}", key, value).into())
}

/// Decodes one tile into the values it holds.
fn decode_tile(
    header: &Header,
    row: &crate::bin_table::Row,
    count: usize,
    bitpix: Bitpix,
) -> Result<Vec<f64>, Box<dyn Error + Send + Sync>> {
    let compressed = bytes_of(row, COMPRESSED_DATA).unwrap_or_default();

    // PLIO's instructions arrive as words rather than bytes, so an empty byte
    // reading does not mean the tile is empty.
    let is_empty = compressed.is_empty() && words_of(row, COMPRESSED_DATA)?.is_empty();

    // A tile the coder could not shrink is stored plainly instead, in one of two
    // fallback columns.
    let values = if is_empty {
        if let Some(plain) = bytes_of(row, UNCOMPRESSED_DATA)
            .ok()
            .filter(|b| !b.is_empty())
        {
            from_be_bytes(&plain, bitpix, count)
        } else {
            let gzipped = bytes_of(row, GZIP_COMPRESSED_DATA)?;
            if gzipped.is_empty() {
                return Err("A compressed image tile holds no data at all".into());
            }
            from_be_bytes(&gunzip(&gzipped)?, bitpix, count)
        }
    } else {
        decompress(header, row, &compressed, count, bitpix)?
    };

    // A floating point image is compressed by quantising it to integers, which
    // these two columns scale back.
    let scale = number_of(row, SCALE);
    let zero = number_of(row, ZERO);

    if scale.is_none() && zero.is_none() {
        return Ok(values);
    }

    let scale = scale.unwrap_or(1.0);
    let zero = zero.unwrap_or(0.0);

    Ok(values
        .into_iter()
        .map(|value| zero + scale * value)
        .collect())
}

/// Runs a tile's bytes through whichever algorithm ZCMPTYPE names.
fn decompress(
    header: &Header,
    row: &crate::bin_table::Row,
    compressed: &[u8],
    count: usize,
    bitpix: Bitpix,
) -> Result<Vec<f64>, Box<dyn Error + Send + Sync>> {
    let algorithm = header
        .compression_type()
        .ok_or("A compressed image needs a ZCMPTYPE card to say how it was compressed")?
        .trim();

    match algorithm {
        "RICE_1" => {
            // Rice works on integers of a width the header states separately;
            // for a quantised floating point image that is not BITPIX's width.
            let width = match header.compression_parameter("BYTEPIX") {
                Some(bytes) => BytesPerValue::from_count(bytes)?,
                None => BytesPerValue::Four,
            };
            let block = header
                .compression_parameter("BLOCKSIZE")
                .unwrap_or(32)
                .max(1) as usize;

            Ok(rice::decompress(compressed, count, width, block)?
                .into_iter()
                .map(|value| value as f64)
                .collect())
        }

        // PLIO stores its instructions as 16-bit words rather than as bytes, so
        // the column holds them already decoded.
        "PLIO_1" => Ok(plio::decompress(&words_of(row, COMPRESSED_DATA)?, count)?
            .into_iter()
            .map(|value| value as f64)
            .collect()),

        "HCOMPRESS_1" => {
            // SMOOTH asks the decompressor to soften the artefacts that lossy
            // compression leaves. Ignoring it would hand back an image that
            // differs from what the file asked for.
            let smooth = header.compression_parameter("SMOOTH").unwrap_or(0) != 0;

            let (values, rows, columns) = hcompress::decompress(compressed, smooth)?;

            if rows * columns < count {
                return Err(format!(
                    "An HCOMPRESS tile holds {} values but the header describes {}",
                    rows * columns,
                    count
                )
                .into());
            }

            Ok(values.into_iter().map(|value| value as f64).collect())
        }

        "GZIP_1" => Ok(from_be_bytes(&gunzip(compressed)?, bitpix, count)),

        // GZIP_2 gathers the first byte of every value, then the second, and so
        // on, which compresses better; the order has to be undone.
        "GZIP_2" => {
            let shuffled = gunzip(compressed)?;
            Ok(from_be_bytes(
                &unshuffle(&shuffled, bitpix.byte_size()),
                bitpix,
                count,
            ))
        }

        "NOCOMPRESS" => Ok(from_be_bytes(compressed, bitpix, count)),

        // Guessing at an algorithm would produce an image made of noise, which
        // is worse than saying plainly that it is not implemented.
        other => Err(From::from(format!(
            "Compressed images using {} are not supported yet; this crate reads RICE_1, PLIO_1, \
             HCOMPRESS_1, GZIP_1, GZIP_2 and NOCOMPRESS",
            other
        ))),
    }
}

/// Undoes the byte shuffling GZIP_2 applies.
fn unshuffle(shuffled: &[u8], width: usize) -> Vec<u8> {
    if width <= 1 {
        return shuffled.to_vec();
    }

    let count = shuffled.len() / width;
    let mut out = vec![0_u8; count * width];

    for byte in 0..width {
        for value in 0..count {
            out[value * width + byte] = shuffled[byte * count + value];
        }
    }

    out
}

#[cfg(feature = "gzip")]
fn gunzip(bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    use std::io::Read;

    // The convention allows either a gzip wrapper or a bare zlib stream.
    let mut out = Vec::new();
    if flate2::read::GzDecoder::new(bytes)
        .read_to_end(&mut out)
        .is_ok()
    {
        return Ok(out);
    }

    out.clear();
    flate2::read::ZlibDecoder::new(bytes).read_to_end(&mut out)?;

    Ok(out)
}

#[cfg(not(feature = "gzip"))]
fn gunzip(_bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    Err("This tile is gzip compressed, which needs the `gzip` feature".into())
}

/// Reads `count` big-endian values of the image's type.
fn from_be_bytes(bytes: &[u8], bitpix: Bitpix, count: usize) -> Vec<f64> {
    bytes
        .chunks_exact(bitpix.byte_size())
        .filter_map(|raw| bitpix.read_be(raw))
        .take(count)
        .collect()
}

/// Writes the assembled values back out in the image's own type, for the
/// ordinary image reader to pick up.
fn to_be_bytes(pixels: &[f64], bitpix: Bitpix) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(pixels.len() * bitpix.byte_size());

    for pixel in pixels {
        match bitpix {
            Bitpix::U8 => bytes.push(*pixel as u8),
            Bitpix::I16 => bytes.extend_from_slice(&(*pixel as i16).to_be_bytes()),
            Bitpix::I32 => bytes.extend_from_slice(&(*pixel as i32).to_be_bytes()),
            Bitpix::F32 => bytes.extend_from_slice(&(*pixel as f32).to_be_bytes()),
            Bitpix::F64 => bytes.extend_from_slice(&pixel.to_be_bytes()),
        }
    }

    bytes
}

/// A column's 16-bit words, which is how a PLIO instruction list arrives.
///
/// The column may be typed as 16-bit integers or as bytes; a byte column is read
/// two at a time, most significant first, as FITS stores them.
fn words_of(
    row: &crate::bin_table::Row,
    name: &str,
) -> Result<Vec<u16>, Box<dyn Error + Send + Sync>> {
    Ok(match row.get(name)? {
        Some(Value::I16(values)) => values.into_iter().map(|value| value as u16).collect(),
        Some(Value::U16(values)) => values,
        Some(Value::U8(bytes)) | Some(Value::Bit { bytes, .. }) => bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_be_bytes(*pair))
            .collect(),
        Some(Value::Null) | None => Vec::new(),
        Some(other) => {
            return Err(format!("Column {} holds {:?}, not 16-bit words", name, other).into());
        }
    })
}

/// A column's bytes, or empty when the row has no such column.
fn bytes_of(
    row: &crate::bin_table::Row,
    name: &str,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    match row.get(name)? {
        Some(Value::U8(bytes)) | Some(Value::Bit { bytes, .. }) => Ok(bytes),
        Some(Value::Null) | None => Ok(Vec::new()),
        Some(other) => Err(format!("Column {} holds {:?}, not bytes", name, other).into()),
    }
}

/// A column's first value as a number, or `None` when there is no such column.
fn number_of(row: &crate::bin_table::Row, name: &str) -> Option<f64> {
    match row.get(name).ok()?? {
        Value::F64(values) => values.first().copied(),
        Value::F32(values) => values.first().map(|value| *value as f64),
        Value::I64(values) => values.first().map(|value| *value as f64),
        Value::I32(values) => values.first().map(|value| *value as f64),
        _ => None,
    }
}
