//! Writing an image out tile-compressed, and reading it back.
//!
//! What goes in must come out: an image compressed here and read back through
//! the ordinary image path is the image that went in, pixel for pixel, unless
//! it was quantised — and then it is within a step of it.

mod common;

use common::write_temp_fits;
use fits_io::hdu::{ExtensionHDU, HDU, ImageHDU};
use fits_io::image::Image;
use fits_io::image::compression::{Compression, CompressionOptions, Quantization, Quantize};
use fits_io::{Fits, FitsSlice, SliceImageHDU};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

const ALGORITHMS: [Compression; 6] = [
    Compression::Rice,
    Compression::Gzip,
    Compression::ShuffledGzip,
    Compression::Hcompress { scale: 0 },
    Compression::Plio,
    Compression::None,
];

/// A file whose primary HDU holds `values` as a `width` by `height` image.
fn image_file(
    width: u32,
    height: u32,
    values: &[i16],
) -> Result<FitsSlice, Box<dyn Error + Send + Sync>> {
    let mut fits = FitsSlice::new();
    fits.primary_hdu_mut()
        .set_raw_images_i16(width, height, &[values])?;

    Ok(fits)
}

/// The HDU holding the file's image.
///
/// A compressed image cannot sit in the primary HDU, so a file that was written
/// with one there has it in its first extension instead.
fn image_hdu(fits: &FitsSlice) -> &SliceImageHDU {
    if fits.primary_hdu().image_count() > 0 {
        return fits.primary_hdu();
    }

    match fits.extension_hdu(0) {
        Some(ExtensionHDU::Image(hdu)) => hdu,
        other => panic!("expected an image extension, got {other:?}"),
    }
}

/// The pixels of the file's first image, as 16-bit values.
fn pixels(fits: &FitsSlice) -> Result<Vec<i16>, Box<dyn Error + Send + Sync>> {
    let image = image_hdu(fits)
        .read_image(0)?
        .expect("the HDU holds an image");

    match image {
        Image::I16(data) => Ok(data.raw().to_vec()),
        other => panic!("expected a 16-bit image, got {other:?}"),
    }
}

/// A gradient with a bright spot in it, which compresses well and has something
/// in it that would be noticed if it moved.
///
/// Every value is positive and small, so that the algorithms that only take a
/// mask — PLIO — can be given the same image as the rest.
fn gradient(width: u32, height: u32) -> Vec<i16> {
    (0..(width * height))
        .map(|index| {
            let (x, y) = (index % width, index / width);
            let value = 100 + (x as i16) * 3 + (y as i16) * 7;

            if x == width / 2 && y == height / 2 {
                value + 900
            } else {
                value
            }
        })
        .collect()
}

/// Writes `fits` out and reads it back, so that what a test asserts is what a
/// reader would see rather than what is still in memory.
fn round_trip(name: &str, fits: &FitsSlice) -> Result<FitsSlice, Box<dyn Error + Send + Sync>> {
    let bytes = fits.to_vec()?;
    // Through a real file, so that the header written is the header read.
    let path = write_temp_fits(name, &bytes)?;

    FitsSlice::from_slice(&std::fs::read(path)?)
}

#[test]
fn every_algorithm_gives_back_the_image_it_was_given() -> TestResult {
    let values = gradient(16, 12);

    for compression in ALGORITHMS {
        let mut fits = image_file(16, 12, &values)?;

        fits.primary_hdu_mut()
            .compress(&CompressionOptions::new(compression))?;
        assert!(fits.primary_hdu().is_compressed(), "{compression:?}");

        let read = round_trip(
            &format!("compress-{}.fits", compression.card_value()),
            &fits,
        )?;

        assert_eq!(pixels(&read)?, values, "{compression:?}");
        assert_eq!(image_hdu(&read).images_width(), 16);
        assert_eq!(image_hdu(&read).images_height(), 12);
    }

    Ok(())
}

#[test]
fn a_compressed_image_is_smaller_than_the_one_it_came_from() -> TestResult {
    // A gradient is about as compressible as an image gets, so this is a low
    // bar — but a compressor that grew its input would clear it.
    let values = gradient(64, 64);

    let plain = image_file(64, 64, &values)?;
    let plain_size = plain.to_vec()?.len();

    let mut compressed = image_file(64, 64, &values)?;
    compressed
        .primary_hdu_mut()
        .compress(&CompressionOptions::new(Compression::Rice).with_tile_size(&[64, 64]))?;

    assert!(
        compressed.to_vec()?.len() < plain_size,
        "{} compressed against {} plain",
        compressed.to_vec()?.len(),
        plain_size
    );

    Ok(())
}

#[test]
fn a_tile_that_does_not_divide_the_image_still_covers_it() -> TestResult {
    // Tiles that do not fit a whole number of times are cut short at the far
    // edge, and the pixels there are the ones most easily lost.
    let values = gradient(17, 13);

    for tile in [[5_u32, 4], [17, 13], [1, 1], [8, 20]] {
        let mut fits = image_file(17, 13, &values)?;
        fits.primary_hdu_mut()
            .compress(&CompressionOptions::new(Compression::Rice).with_tile_size(&tile))?;

        let read = round_trip(
            &format!("compress-tile-{}x{}.fits", tile[0], tile[1]),
            &fits,
        )?;

        assert_eq!(pixels(&read)?, values, "tiles of {tile:?}");
    }

    Ok(())
}

#[test]
fn a_cube_is_compressed_a_plane_at_a_time_and_comes_back_whole() -> TestResult {
    let planes: Vec<Vec<i16>> = (0..4)
        .map(|plane| {
            (0..48)
                .map(|index| (index as i16) + (plane as i16) * 1000)
                .collect()
        })
        .collect();
    let borrowed: Vec<&[i16]> = planes.iter().map(|plane| plane.as_slice()).collect();

    let mut fits = FitsSlice::new();
    fits.primary_hdu_mut().set_raw_images_i16(8, 6, &borrowed)?;

    fits.primary_hdu_mut()
        .compress(&CompressionOptions::new(Compression::Rice).with_tile_size(&[8, 6, 1]))?;

    let read = round_trip("compress-cube.fits", &fits)?;
    let hdu = image_hdu(&read);

    assert_eq!(hdu.image_count(), 4);
    for (index, plane) in planes.iter().enumerate() {
        let image = hdu.read_image(index)?.expect("the cube holds this plane");
        let Image::I16(data) = image else {
            panic!("expected 16-bit planes");
        };

        assert_eq!(data.raw(), plane.as_slice(), "plane {index}");
    }

    Ok(())
}

#[test]
fn a_floating_point_image_survives_gzip_untouched() -> TestResult {
    let values: Vec<f32> = (0..64).map(|index| index as f32 * 0.1 - 3.2).collect();

    let mut fits = FitsSlice::new();
    fits.primary_hdu_mut()
        .set_raw_images_f32(8, 8, &[&values])?;
    fits.primary_hdu_mut()
        .compress(&CompressionOptions::new(Compression::Gzip))?;

    let read = round_trip("compress-float-gzip.fits", &fits)?;
    let image = image_hdu(&read)
        .read_image(0)?
        .expect("the HDU holds an image");

    let Image::F32(data) = image else {
        panic!("expected a 32-bit float image, got {image:?}");
    };

    // Lossless: not "close to", the same bits.
    assert_eq!(data.raw(), values.as_slice());

    Ok(())
}

#[test]
fn a_quantised_image_comes_back_within_one_step() -> TestResult {
    let values: Vec<f32> = (0..1024)
        .map(|index| 500.0 + (index as f32) * 0.037 + ((index % 13) as f32) * 0.11)
        .collect();

    let step = 0.01_f64;

    let mut fits = FitsSlice::new();
    fits.primary_hdu_mut()
        .set_raw_images_f32(32, 32, &[&values])?;
    fits.primary_hdu_mut().compress(
        &CompressionOptions::new(Compression::Rice)
            .with_quantization(Quantize::Step(step))
            .with_tile_size(&[32, 32]),
    )?;

    let read = round_trip("compress-float-quantised.fits", &fits)?;
    let image = image_hdu(&read)
        .read_image(0)?
        .expect("the HDU holds an image");

    let Image::F32(data) = image else {
        panic!("expected a 32-bit float image, got {image:?}");
    };

    for (original, returned) in values.iter().zip(data.raw()) {
        assert!(
            (*original as f64 - *returned as f64).abs() <= step,
            "{original} came back as {returned}, further than the quantisation step of {step}"
        );
    }

    // And it is lossy, so the bits are not the same ones.
    assert_ne!(data.raw(), values.as_slice());

    Ok(())
}

#[test]
fn quantising_by_the_noise_keeps_what_is_above_the_noise() -> TestResult {
    // A smooth source with noise on it: quantising to a quarter of the noise
    // must not move any pixel by as much as the noise itself.
    let values: Vec<f32> = (0..1024)
        .map(|index| {
            let noise = ((index * 2654435761_usize) % 101) as f32 / 100.0 - 0.5;
            1000.0 + (index as f32 / 32.0).sin() * 20.0 + noise
        })
        .collect();

    let mut fits = FitsSlice::new();
    fits.primary_hdu_mut()
        .set_raw_images_f32(32, 32, &[&values])?;
    fits.primary_hdu_mut().compress(
        &CompressionOptions::new(Compression::Rice)
            .with_quantization(Quantize::NoiseLevel(4.0))
            .with_tile_size(&[32, 32]),
    )?;

    let read = round_trip("compress-float-noise.fits", &fits)?;
    let Some(Image::F32(data)) = image_hdu(&read).read_image(0)? else {
        panic!("expected a 32-bit float image");
    };

    for (original, returned) in values.iter().zip(data.raw()) {
        assert!(
            (original - returned).abs() < 1.0,
            "{original} came back as {returned}"
        );
    }

    Ok(())
}

#[test]
fn an_undefined_pixel_stays_undefined_through_quantisation() -> TestResult {
    let mut values: Vec<f32> = (0..256).map(|index| 10.0 + index as f32 * 0.5).collect();
    values[7] = f32::NAN;
    values[200] = f32::NAN;

    let mut fits = FitsSlice::new();
    fits.primary_hdu_mut()
        .set_raw_images_f32(16, 16, &[&values])?;
    fits.primary_hdu_mut().compress(
        &CompressionOptions::new(Compression::Rice)
            .with_quantization(Quantize::Step(0.05))
            .with_tile_size(&[16, 16]),
    )?;

    let read = round_trip("compress-float-blank.fits", &fits)?;
    let Some(Image::F32(data)) = image_hdu(&read).read_image(0)? else {
        panic!("expected a 32-bit float image");
    };

    for (index, value) in data.raw().iter().enumerate() {
        if index == 7 || index == 200 {
            assert!(value.is_nan(), "pixel {index} came back as {value}");
        } else {
            assert!(value.is_finite(), "pixel {index} came back as {value}");
        }
    }

    Ok(())
}

#[test]
fn dither_two_leaves_an_exact_zero_exactly_zero() -> TestResult {
    let mut values: Vec<f32> = (0..256).map(|index| 10.0 + index as f32 * 0.5).collect();
    values[42] = 0.0;

    let mut fits = FitsSlice::new();
    fits.primary_hdu_mut()
        .set_raw_images_f32(16, 16, &[&values])?;
    fits.primary_hdu_mut().compress(
        &CompressionOptions::new(Compression::Rice)
            .with_quantization(Quantize::Step(0.05))
            .with_dithering(Quantization::SubtractiveDither2)
            .with_tile_size(&[16, 16]),
    )?;

    let read = round_trip("compress-float-zero.fits", &fits)?;
    let Some(Image::F32(data)) = image_hdu(&read).read_image(0)? else {
        panic!("expected a 32-bit float image");
    };

    assert_eq!(data.raw()[42], 0.0);

    Ok(())
}

#[test]
fn the_same_data_and_seed_produce_the_same_file() -> TestResult {
    let values: Vec<f32> = (0..256).map(|index| 3.0 + index as f32 * 0.01).collect();

    let write = |seed: i64| -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let mut fits = FitsSlice::new();
        fits.primary_hdu_mut()
            .set_raw_images_f32(16, 16, &[&values])?;
        fits.primary_hdu_mut().compress(
            &CompressionOptions::new(Compression::Rice)
                .with_quantization(Quantize::Step(0.001))
                .with_dither_seed(seed),
        )?;

        fits.to_vec()
    };

    assert_eq!(write(7)?, write(7)?);
    // A different seed dithers differently, so the bytes differ even though the
    // image does not.
    assert_ne!(write(7)?, write(8)?);

    Ok(())
}

#[test]
fn decompressing_gives_back_an_ordinary_image_hdu() -> TestResult {
    let values = gradient(12, 9);

    let mut fits = image_file(12, 9, &values)?;
    fits.primary_hdu_mut()
        .compress(&CompressionOptions::new(Compression::Rice))?;
    fits.primary_hdu_mut().decompress()?;

    assert!(!fits.primary_hdu().is_compressed());

    let read = round_trip("compress-then-decompress.fits", &fits)?;

    assert!(!image_hdu(&read).is_compressed());
    assert_eq!(pixels(&read)?, values);

    Ok(())
}

#[test]
fn compressing_twice_does_not_compress_the_tiles_twice() -> TestResult {
    let values = gradient(12, 9);

    let mut fits = image_file(12, 9, &values)?;
    fits.primary_hdu_mut()
        .compress(&CompressionOptions::new(Compression::Rice))?;
    fits.primary_hdu_mut()
        .compress(&CompressionOptions::new(Compression::Gzip))?;

    let read = round_trip("compress-twice.fits", &fits)?;

    assert_eq!(image_hdu(&read).header().compression_type(), Some("GZIP_1"));
    assert_eq!(pixels(&read)?, values);

    Ok(())
}

#[test]
fn a_compressed_extension_is_written_as_the_table_it_is_stored_as() -> TestResult {
    let values = gradient(10, 10);

    let mut fits = FitsSlice::new();
    let mut hdu = SliceImageHDU::empty();
    hdu.set_raw_images_i16(10, 10, &[&values])?;
    hdu.compress(&CompressionOptions::new(Compression::Rice))?;
    fits.push_extension(ExtensionHDU::Image(hdu));

    let read = round_trip("compress-extension.fits", &fits)?;

    // A reader that knows nothing of the convention must see a valid binary
    // table extension there, not an image extension whose data is a table.
    let Some(ExtensionHDU::Image(hdu)) = read.extension_hdu(0) else {
        panic!("expected the extension to read back as an image");
    };
    assert_eq!(
        hdu.header().extension(),
        Some(fits_io::header::ExtensionType::BinTable)
    );

    let Some(Image::I16(data)) = hdu.read_image(0)? else {
        panic!("expected a 16-bit image");
    };
    assert_eq!(data.raw(), values.as_slice());

    Ok(())
}

#[test]
fn compressing_keeps_the_cards_that_describe_the_image() -> TestResult {
    let values = gradient(8, 8);

    let mut fits = image_file(8, 8, &values)?;
    fits.primary_hdu_mut()
        .header_mut()
        .set_card("OBJECT", "M42")?;
    fits.primary_hdu_mut()
        .header_mut()
        .set_card("CTYPE1", "RA---TAN")?;

    fits.primary_hdu_mut()
        .compress(&CompressionOptions::new(Compression::Rice))?;

    let read = round_trip("compress-cards.fits", &fits)?;

    // The image's own header, as a caller wanting its WCS would ask for it.
    let image_header = image_hdu(&read).header().uncompressed();
    assert_eq!(image_header.object(), Some("M42"));
    assert_eq!(image_header.coordinate_axis_name(0), Some("RA---TAN"));

    Ok(())
}

#[test]
fn an_hcompress_tile_cannot_reach_along_a_third_axis() -> TestResult {
    let planes: Vec<Vec<i16>> = (0..2).map(|_| (0..48).collect()).collect();
    let borrowed: Vec<&[i16]> = planes.iter().map(|plane| plane.as_slice()).collect();

    let mut fits = FitsSlice::new();
    fits.primary_hdu_mut().set_raw_images_i16(8, 6, &borrowed)?;

    let error = fits
        .primary_hdu_mut()
        .compress(
            &CompressionOptions::new(Compression::Hcompress { scale: 0 })
                .with_tile_size(&[8, 6, 2]),
        )
        .expect_err("the transform works on a plane at a time");

    assert!(
        error.to_string().contains("plane at a time"),
        "got: {error}"
    );

    Ok(())
}

#[test]
fn a_lossy_hcompress_image_comes_back_close_to_what_it_was() -> TestResult {
    let values = gradient(64, 64);

    let mut fits = image_file(64, 64, &values)?;
    fits.primary_hdu_mut().compress(
        &CompressionOptions::new(Compression::Hcompress { scale: 16 }).with_tile_size(&[64, 64]),
    )?;

    let plain = image_file(64, 64, &values)?.to_vec()?.len();
    assert!(
        fits.to_vec()?.len() < plain,
        "a lossily compressed image should be smaller than the one it came from"
    );

    let read = round_trip("compress-hcompress-lossy.fits", &fits)?;

    for (original, returned) in values.iter().zip(pixels(&read)?) {
        assert!(
            (*original - returned).abs() <= 16,
            "{original} came back as {returned}, further than the scale factor"
        );
    }

    Ok(())
}

#[test]
fn a_mask_that_plio_cannot_hold_says_so() -> TestResult {
    // PLIO stores masks: small non-negative numbers. A pixel beyond what its
    // instructions can name has nowhere to go.
    let values: Vec<i32> = vec![1 << 28; 16];

    let mut fits = FitsSlice::new();
    fits.primary_hdu_mut()
        .set_raw_images_i32(4, 4, &[&values])?;

    let error = fits
        .primary_hdu_mut()
        .compress(&CompressionOptions::new(Compression::Plio))
        .expect_err("that value does not fit a line list");

    assert!(
        error.to_string().contains("holds values up to"),
        "got: {error}"
    );

    Ok(())
}

#[test]
fn a_mask_round_trips_through_plio() -> TestResult {
    // What PLIO is for: an integer mask of long runs of a handful of values.
    let values: Vec<i16> = (0..(32 * 32))
        .map(|index| match index % 128 {
            0..=63 => 0,
            64..=95 => 1,
            _ => 5,
        })
        .collect();

    let mut fits = image_file(32, 32, &values)?;
    fits.primary_hdu_mut()
        .compress(&CompressionOptions::new(Compression::Plio).with_tile_size(&[32, 32]))?;

    let read = round_trip("compress-plio-mask.fits", &fits)?;

    assert_eq!(read_hdu_type(&read), Some("PLIO_1".to_string()));
    assert_eq!(pixels(&read)?, values);

    Ok(())
}

/// The algorithm a file's compressed image was written with.
fn read_hdu_type(fits: &FitsSlice) -> Option<String> {
    image_hdu(fits)
        .header()
        .compression_type()
        .map(str::to_string)
}
