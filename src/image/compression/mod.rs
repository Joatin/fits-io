//! Reading images that are stored compressed, tile by tile, inside a table.
//!
//! The tiled image convention cuts an image into rectangular tiles, compresses
//! each one, and stores the results as the rows of a binary table. The table's
//! header describes the image it stands for with keywords beginning `Z`.

pub(crate) mod dither;
pub(crate) mod hcompress;
pub(crate) mod plio;
pub(crate) mod rice;
mod write;

pub use self::dither::Quantization;
pub use self::write::{Compression, CompressionOptions, Quantize};

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

/// The column carrying a tile's own blank value, where the tiles do not share
/// one.
const BLANK: &str = "ZBLANK";

/// Compresses an image into the table that will stand for it.
///
/// `header` and `data` are the image's own; what comes back is the header and
/// data section of the binary table extension carrying it compressed.
pub(crate) fn compress_image(
    header: &Header,
    data: &[u8],
    options: &CompressionOptions,
) -> Result<(Header, Vec<u8>), Box<dyn Error + Send + Sync>> {
    write::compress(header, data, options)
}

/// Unpacks a compressed image back into an ordinary one.
///
/// The header that comes back is the image's own, with the table's keywords
/// gone, and the data is its pixels laid out as an image HDU holds them.
pub(crate) fn decompress_image(
    header: &Header,
    data: &[u8],
) -> Result<(Header, Vec<u8>), Box<dyn Error + Send + Sync>> {
    let table = BinTable::from_u8(header, data.to_vec())?;
    let (data, image_header) = read_data(header, &table)?;

    Ok((image_header, data))
}

/// Decompresses the image a table stands for.
pub(crate) fn read_image(
    header: &Header,
    table: &BinTable,
) -> Result<Image, Box<dyn Error + Send + Sync>> {
    let (data, image_header) = read_data(header, table)?;

    Image::from_data_and_header(data, &image_header)
}

/// Decompresses the image a table stands for, as the bytes and header an
/// ordinary image HDU would hold.
pub(crate) fn read_data(
    header: &Header,
    table: &BinTable,
) -> Result<(Vec<u8>, Header), Box<dyn Error + Send + Sync>> {
    let bitpix = header
        .compressed_bitpix()
        .ok_or("A compressed image needs a ZBITPIX card to say what its values are")?;

    let shape = shape_of(header)?;
    let tile = tile_shape(header, &shape);

    // How many tiles the image is cut into along each axis, and how many pixels
    // it holds altogether.
    let tiles: Vec<usize> = shape
        .iter()
        .zip(&tile)
        .map(|(length, tile)| length.div_ceil(*tile))
        .collect();

    let count = shape.iter().try_fold(1_usize, |total, length| {
        total.checked_mul(*length).ok_or_else(|| {
            format!(
                "A compressed image of {:?} holds more pixels than can be counted",
                shape
            )
        })
    })?;

    // The image is assembled in the type it will be read back as, so that the
    // ordinary image path can take it from here.
    let mut pixels = vec![0_f64; count];

    let quantization = Quantization::from_card(header.quantization_method())?;
    let seed = header.dither_seed().unwrap_or(1);

    // Where each axis starts and how far a tile reaches along it, reused for
    // every tile rather than rebuilt.
    let mut origin = vec![0_usize; shape.len()];
    let mut extent = vec![0_usize; shape.len()];

    for (index, row) in table.rows().enumerate() {
        // The tiles are stored in the order the axes are in, the first axis
        // running fastest, so the row number decomposes into the tile's place
        // along each of them.
        let mut rest = index;
        for (axis, count) in tiles.iter().enumerate() {
            origin[axis] = (rest % count) * tile[axis];
            rest /= count;
        }

        // A table with more rows than the image has tiles describes more image
        // than the header does, and the header is what says how big it is.
        if rest > 0 {
            break;
        }

        // Tiles at the far edge of each axis are cut short by the image's size.
        for axis in 0..shape.len() {
            extent[axis] = tile[axis].min(shape[axis] - origin[axis]);
        }

        let holds = extent.iter().product::<usize>();
        let values = decode_tile(header, &row, holds, bitpix, quantization, seed, index)?;

        scatter(&mut pixels, &shape, &origin, &extent, &values);
    }

    // The header of the image this stands for, so that BZERO, BSCALE, DATAMIN
    // and the rest are read exactly as they would be for an ordinary image.
    let image_header = header.uncompressed();

    Ok((to_be_bytes(&pixels, bitpix), image_header))
}

/// The shape of the image a compressed table stands for, fastest axis first.
fn shape_of(header: &Header) -> Result<Vec<usize>, Box<dyn Error + Send + Sync>> {
    let axes = header
        .compressed_naxis()
        .ok_or("A compressed image needs a ZNAXIS card to say how many axes it has")?;

    if axes < 1 {
        return Err(format!("A compressed image cannot have {} axes", axes).into());
    }

    (0..axes as usize)
        .map(|axis| {
            size(
                header.compressed_naxis_n(axis),
                &format!("ZNAXIS{}", axis + 1),
            )
        })
        .collect()
}

/// How far a tile reaches along each axis.
///
/// The convention's default is a tile one row wide, which is what a header that
/// leaves the ZTILEn cards out means.
fn tile_shape(header: &Header, shape: &[usize]) -> Vec<usize> {
    (0..shape.len())
        .map(|axis| {
            let size = header.compressed_tile_size(axis);

            usize::try_from(size)
                .unwrap_or(1)
                .max(1)
                .min(shape[axis].max(1))
        })
        .collect()
}

/// Copies one tile's values into the image at `origin`.
///
/// A tile is laid out with its own first axis running fastest, the same way the
/// image is, so each run of `extent[0]` values is contiguous in both and the
/// copy walks the tile one such run at a time.
fn scatter(
    pixels: &mut [f64],
    shape: &[usize],
    origin: &[usize],
    extent: &[usize],
    values: &[f64],
) {
    let run = extent[0];
    if run == 0 {
        return;
    }

    let runs = extent.iter().skip(1).product::<usize>();
    let mut within = vec![0_usize; shape.len()];

    for index in 0..runs {
        // Where this run sits along every axis but the first.
        let mut rest = index;
        for axis in 1..shape.len() {
            within[axis] = rest % extent[axis];
            rest /= extent[axis];
        }

        // The image is indexed with the first axis running fastest, so each
        // further axis steps by the product of the ones before it.
        let mut at = origin[0];
        let mut stride = shape[0];
        for axis in 1..shape.len() {
            at += (origin[axis] + within[axis]) * stride;
            stride *= shape[axis];
        }

        let from = index * run;
        let Some(source) = values.get(from..from + run) else {
            return;
        };
        let Some(target) = pixels.get_mut(at..at + run) else {
            return;
        };

        target.copy_from_slice(source);
    }
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
    quantization: Quantization,
    seed: i64,
    tile: usize,
) -> Result<Vec<f64>, Box<dyn Error + Send + Sync>> {
    // A floating point image is compressed by quantising it to integers, which
    // these two columns scale back. Their presence is also what says the tile
    // holds integers rather than the floats ZBITPIX names.
    let scale = number_of(row, SCALE);
    let zero = number_of(row, ZERO);
    let quantised = scale.is_some() || zero.is_some();

    // The width the tile is actually stored in: a quantised float tile holds
    // 32-bit integers however wide ZBITPIX says its values are.
    let stored = if quantised { Bitpix::I32 } else { bitpix };

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
            from_be_bytes(&plain, stored, count)
        } else {
            let gzipped = bytes_of(row, GZIP_COMPRESSED_DATA)?;
            if gzipped.is_empty() {
                return Err("A compressed image tile holds no data at all".into());
            }
            from_be_bytes(&gunzip(&gzipped)?, stored, count)
        }
    } else {
        decompress(header, row, &compressed, count, stored)?
    };

    // A tile may name its own blank value, and otherwise the file's ZBLANK
    // stands for every tile.
    let blank = number_of(row, BLANK).or_else(|| header.compressed_blank().map(|v| v as f64));

    if !quantised {
        // An integer image says which value is undefined with BLANK, which the
        // ordinary image reader applies; a float one has NaN already.
        return Ok(values);
    }

    Ok(dither::unquantize(
        &values,
        scale.unwrap_or(1.0),
        zero.unwrap_or(0.0),
        quantization,
        blank,
        dither::Dither::for_tile(seed, tile),
    ))
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
