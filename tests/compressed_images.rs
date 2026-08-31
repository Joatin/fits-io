//! The tiled image convention: an image cut into tiles, each one compressed and
//! stored as a row of a binary table. This is how `fpack` and most archives
//! distribute large images.
//!
//! The compressed tile bytes here were produced by cfitsio, through astropy's
//! `CompImageHDU`, so these check this crate against the implementation the
//! convention was written around rather than against itself.

mod common;

use common::{append_extension, fits_file, write_temp_fits};
use fits_io::hdu::{BinTableHDU, ExtensionHDU, HDU, ImageHDU};
use fits_io::image::Image;
use fits_io::{Fits, FitsSlice};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

/// Builds a compressed-image extension whose tiles are 16-bit words rather than
/// bytes, which is how PLIO stores its instruction lists.
fn compressed_image_of_words(cards: &[(&str, &str)], tiles: &[&[u16]]) -> Vec<u8> {
    let bytes: Vec<Vec<u8>> = tiles
        .iter()
        .map(|tile| tile.iter().flat_map(|word| word.to_be_bytes()).collect())
        .collect();
    let tiles: Vec<&[u8]> = bytes.iter().map(|tile| tile.as_slice()).collect();

    // `1PI` counts 16-bit elements, so the descriptors hold half as many as
    // there are bytes.
    compressed_image_with(cards, &tiles, "'1PI'", 2)
}

/// Builds a compressed-image extension whose rows hold `tiles`.
///
/// The tile bytes go in a variable length array column, which is how the
/// convention stores them: a descriptor of count and heap offset in the row, and
/// the bytes themselves in the heap after the last row.
fn compressed_image(cards: &[(&str, &str)], tiles: &[&[u8]]) -> Vec<u8> {
    compressed_image_with(cards, tiles, "'1PB'", 1)
}

fn compressed_image_with(
    cards: &[(&str, &str)],
    tiles: &[&[u8]],
    form: &str,
    bytes_per_element: usize,
) -> Vec<u8> {
    let mut rows = Vec::new();
    let mut heap = Vec::new();

    for tile in tiles {
        rows.extend_from_slice(&((tile.len() / bytes_per_element) as i32).to_be_bytes());
        rows.extend_from_slice(&(heap.len() as i32).to_be_bytes());
        heap.extend_from_slice(tile);
    }

    let pcount = heap.len().to_string();
    let count = tiles.len().to_string();

    let mut data = rows;
    data.extend_from_slice(&heap);

    let mut file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "0"),
            ("EXTEND", "T"),
        ],
        &[],
    );

    let mut header = vec![
        ("XTENSION", "'BINTABLE'"),
        ("BITPIX", "8"),
        ("NAXIS", "2"),
        ("NAXIS1", "8"),
        ("NAXIS2", count.as_str()),
        ("PCOUNT", pcount.as_str()),
        ("GCOUNT", "1"),
        ("TFIELDS", "1"),
        ("TFORM1", form),
        ("TTYPE1", "'COMPRESSED_DATA'"),
        ("ZIMAGE", "T"),
    ];
    header.extend_from_slice(cards);

    append_extension(&mut file, &header, &data);

    file
}

fn open(name: &str, file: &[u8]) -> Result<FitsSlice, Box<dyn Error + Send + Sync>> {
    write_temp_fits(name, file)?;
    FitsSlice::from_slice(file)
}

/// The compressed image extension, which the reader hands back as an image.
fn image_hdu(fits: &FitsSlice) -> &impl ImageHDU {
    let Some(ExtensionHDU::Image(hdu)) = fits.extension_hdu(0) else {
        panic!("a compressed image extension reads as an image");
    };
    hdu
}

fn raw_i16(image: &Image) -> Vec<i16> {
    match image {
        Image::I16(data) => data.raw().to_vec(),
        other => panic!("expected a 16-bit image, got {other:?}"),
    }
}

/// The six tiles of an 8x6 image of `i * 3 + j`, in 4x2 tiles.
const GRADIENT_TILES: [&[u8]; 6] = [
    &[0x00, 0x00, 0x38, 0xcc, 0xc3, 0xb3, 0x30],
    &[0x00, 0x0c, 0x38, 0xcc, 0xc3, 0xb3, 0x30],
    &[0x00, 0x02, 0x38, 0xcc, 0xc3, 0xb3, 0x30],
    &[0x00, 0x0e, 0x38, 0xcc, 0xc3, 0xb3, 0x30],
    &[0x00, 0x04, 0x38, 0xcc, 0xc3, 0xb3, 0x30],
    &[0x00, 0x10, 0x38, 0xcc, 0xc3, 0xb3, 0x30],
];

fn gradient_cards() -> Vec<(&'static str, &'static str)> {
    vec![
        ("ZBITPIX", "16"),
        ("ZNAXIS", "2"),
        ("ZNAXIS1", "8"),
        ("ZNAXIS2", "6"),
        ("ZTILE1", "4"),
        ("ZTILE2", "2"),
        ("ZCMPTYPE", "'RICE_1'"),
        ("ZNAME1", "'BLOCKSIZE'"),
        ("ZVAL1", "32"),
        ("ZNAME2", "'BYTEPIX'"),
        ("ZVAL2", "2"),
    ]
}

#[test]
fn a_rice_compressed_image_is_reassembled_from_its_tiles() -> TestResult {
    // Six tiles, two across and three down, none of which lines up with the
    // image on its own: getting the tile geometry wrong scrambles the result.
    let file = compressed_image(&gradient_cards(), &GRADIENT_TILES);
    let fits = open("compressed-rice.fits", &file)?;

    let image = image_hdu(&fits)
        .read_image(0)?
        .expect("the extension holds an image");

    assert_eq!(image.width(), 8);
    assert_eq!(image.height(), 6);

    let expected: Vec<i16> = (0..6)
        .flat_map(|row| (0..8).map(move |column| column * 3 + row))
        .collect();

    assert_eq!(raw_i16(&image), expected);

    Ok(())
}

#[test]
fn a_compressed_extension_reports_the_images_own_shape() -> TestResult {
    // The table it is stored in is 8 bytes by 6 rows; the image is 8 by 6
    // pixels of 16-bit data. Reporting the table's shape would be useless.
    let file = compressed_image(&gradient_cards(), &GRADIENT_TILES);
    let fits = open("compressed-flag.fits", &file)?;

    let hdu = image_hdu(&fits);

    assert_eq!(hdu.image_count(), 1);
    assert_eq!(hdu.images_width(), 8);
    assert_eq!(hdu.images_height(), 6);
    assert_eq!(hdu.image_data_size(), 8 * 6 * 2);

    Ok(())
}

#[test]
fn an_ordinary_table_is_not_a_compressed_image() -> TestResult {
    let mut file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "0"),
            ("EXTEND", "T"),
        ],
        &[],
    );
    append_extension(
        &mut file,
        &[
            ("XTENSION", "'BINTABLE'"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "4"),
            ("NAXIS2", "1"),
            ("PCOUNT", "0"),
            ("GCOUNT", "1"),
            ("TFIELDS", "1"),
            ("TFORM1", "'1J'"),
        ],
        &[0, 0, 0, 1],
    );

    let fits = open("compressed-not.fits", &file)?;

    let Some(ExtensionHDU::BinTable(hdu)) = fits.extension_hdu(0) else {
        panic!("an ordinary table still reads as a table");
    };
    assert!(!hdu.is_compressed_image());
    assert!(hdu.read_compressed_image()?.is_none());

    Ok(())
}

#[test]
fn an_uncompressed_tile_is_read_as_it_stands() -> TestResult {
    // NOCOMPRESS stores the values plainly, which is what the coder falls back
    // to for a tile compression would not shrink.
    let mut tile = Vec::new();
    for value in [10_i16, 20, 30, 40] {
        tile.extend_from_slice(&value.to_be_bytes());
    }

    let file = compressed_image(
        &[
            ("ZBITPIX", "16"),
            ("ZNAXIS", "2"),
            ("ZNAXIS1", "4"),
            ("ZNAXIS2", "1"),
            ("ZTILE1", "4"),
            ("ZTILE2", "1"),
            ("ZCMPTYPE", "'NOCOMPRESS'"),
        ],
        &[&tile],
    );

    let fits = open("compressed-none.fits", &file)?;
    let image = image_hdu(&fits)
        .read_image(0)?
        .expect("the extension holds an image");

    assert_eq!(raw_i16(&image), vec![10, 20, 30, 40]);

    Ok(())
}

#[cfg(feature = "gzip")]
#[test]
fn a_gzip_compressed_tile_is_read() -> TestResult {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut raw = Vec::new();
    for value in [7_i16, -7, 1000, -1000] {
        raw.extend_from_slice(&value.to_be_bytes());
    }

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw)?;
    let tile = encoder.finish()?;

    let file = compressed_image(
        &[
            ("ZBITPIX", "16"),
            ("ZNAXIS", "2"),
            ("ZNAXIS1", "4"),
            ("ZNAXIS2", "1"),
            ("ZTILE1", "4"),
            ("ZTILE2", "1"),
            ("ZCMPTYPE", "'GZIP_1'"),
        ],
        &[&tile],
    );

    let fits = open("compressed-gzip.fits", &file)?;
    let image = image_hdu(&fits)
        .read_image(0)?
        .expect("the extension holds an image");

    assert_eq!(raw_i16(&image), vec![7, -7, 1000, -1000]);

    Ok(())
}

#[test]
fn an_unsupported_algorithm_is_an_error_rather_than_noise() -> TestResult {
    // Decoding one algorithm as another would produce a plausible-looking image
    // made of nonsense.
    let file = compressed_image(
        &[
            ("ZBITPIX", "16"),
            ("ZNAXIS", "2"),
            ("ZNAXIS1", "4"),
            ("ZNAXIS2", "1"),
            ("ZTILE1", "4"),
            ("ZTILE2", "1"),
            ("ZCMPTYPE", "'RICE_2'"),
        ],
        &[&[0x00, 0x01, 0x02, 0x03]],
    );

    let fits = open("compressed-unknown.fits", &file)?;
    let error = image_hdu(&fits)
        .read_image(0)
        .expect_err("RICE_2 is not a thing");

    assert!(error.to_string().contains("RICE_2"), "got: {error}");

    Ok(())
}

/// An 8x8 ramp from 0 to 63, compressed by cfitsio with HCOMPRESS at scale 0.
const HCOMPRESS_GRADIENT: &[u8] = &[
    0xdd, 0x99, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x09, 0x05, 0x00, 0xf6, 0x7f, 0xef, 0x39, 0xed, 0x7f, 0xde,
    0xb3, 0xfe, 0xff, 0xbf, 0xef, 0xfb, 0xfe, 0xff, 0x83, 0xff, 0xff, 0xfe, 0x0f, 0xff, 0xff, 0xfb,
    0xfe, 0xff, 0xbf, 0xe0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

fn hcompress_cards(smooth: &'static str) -> Vec<(&'static str, &'static str)> {
    vec![
        ("ZBITPIX", "32"),
        ("ZNAXIS", "2"),
        ("ZNAXIS1", "8"),
        ("ZNAXIS2", "8"),
        ("ZTILE1", "8"),
        ("ZTILE2", "8"),
        ("ZCMPTYPE", "'HCOMPRESS_1'"),
        ("ZNAME1", "'SCALE'"),
        ("ZVAL1", "0"),
        ("ZNAME2", "'SMOOTH'"),
        ("ZVAL2", smooth),
    ]
}

#[test]
fn an_hcompress_image_is_read() -> TestResult {
    let file = compressed_image(&hcompress_cards("0"), &[HCOMPRESS_GRADIENT]);
    let fits = open("compressed-hcompress.fits", &file)?;

    let image = image_hdu(&fits)
        .read_image(0)?
        .expect("the extension holds an image");

    assert_eq!(image.width(), 8);
    assert_eq!(image.height(), 8);

    let Image::I32(data) = &image else {
        panic!("expected a 32-bit image, got {image:?}");
    };
    assert_eq!(data.raw(), &(0..64).collect::<Vec<i32>>());

    Ok(())
}

#[test]
fn an_hcompress_image_asking_to_be_smoothed_is_smoothed() -> TestResult {
    // This tile was compressed losslessly, so there is nothing for smoothing to
    // put back and the image must come out exactly as it went in. What is being
    // checked is that the SMOOTH flag is honoured rather than refused.
    let file = compressed_image(&hcompress_cards("1"), &[HCOMPRESS_GRADIENT]);
    let fits = open("compressed-hcompress-smooth.fits", &file)?;

    let image = image_hdu(&fits)
        .read_image(0)?
        .expect("the extension holds an image");

    let Image::I32(data) = &image else {
        panic!("expected a 32-bit image, got {image:?}");
    };
    assert_eq!(data.raw(), &(0..64).collect::<Vec<i32>>());

    Ok(())
}

#[test]
fn the_image_header_drops_the_table_and_keeps_the_rest() -> TestResult {
    // A caller wanting the WCS of a compressed image needs the header the image
    // would have had, not the table's.
    let mut cards = gradient_cards();
    cards.push(("CTYPE1", "'RA---TAN'"));
    cards.push(("CRPIX1", "4.0"));

    let file = compressed_image(&cards, &GRADIENT_TILES);
    let fits = open("compressed-header.fits", &file)?;

    let header = image_hdu(&fits).header().uncompressed();

    // The image's own shape, taken from the Z keywords.
    assert_eq!(header.naxis(), Some(2));
    assert_eq!(header.naxis_n(0), Some(8));
    assert_eq!(header.naxis_n(1), Some(6));
    assert_eq!(header.bitpix().map(i64::from), Some(16));

    // The table's structure is gone.
    assert_eq!(header.table_fields(), None);
    assert_eq!(header.table_format(0), None);

    // Anything describing the sky comes across untouched.
    assert_eq!(header.coordinate_axis_name(0), Some("RA---TAN"));
    assert_eq!(header.coordinate_reference_pixel(0), Some(4.0));

    Ok(())
}

#[test]
fn an_extension_after_a_compressed_one_is_still_found() -> TestResult {
    // A compressed HDU occupies as much of the file as its *table* does, not as
    // much as the image it stands for. Sizing it by the image would put every
    // following HDU at the wrong offset -- and the two differ here, the table
    // being far smaller than the 96 bytes of pixels it encodes.
    let mut file = compressed_image(&gradient_cards(), &GRADIENT_TILES);

    append_extension(
        &mut file,
        &[
            ("XTENSION", "'IMAGE   '"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "2"),
            ("NAXIS2", "2"),
            ("PCOUNT", "0"),
            ("GCOUNT", "1"),
        ],
        &[9, 8, 7, 6],
    );

    let fits = open("compressed-then-image.fits", &file)?;

    assert_eq!(fits.extension_count(), 2);

    // The compressed image, unpacked.
    let Some(ExtensionHDU::Image(compressed)) = fits.extension_hdu(0) else {
        panic!("the first extension is the compressed image");
    };
    assert_eq!(compressed.images_width(), 8);

    // And the plain image after it, which is only reachable if the first HDU's
    // size was worked out from its table.
    let Some(ExtensionHDU::Image(plain)) = fits.extension_hdu(1) else {
        panic!("the second extension is a plain image");
    };
    let image = plain.read_image(0)?.expect("one image");
    match &image {
        Image::U8(data) => assert_eq!(data.raw(), &[9, 8, 7, 6]),
        other => panic!("expected an 8-bit image, got {other:?}"),
    }

    Ok(())
}

#[test]
fn a_compressed_extension_is_written_back_untouched() -> TestResult {
    // Writing must not try to re-encode the image; the HDU still holds the table
    // it was read from, and that is what goes back out.
    let file = compressed_image(&gradient_cards(), &GRADIENT_TILES);
    let fits = open("compressed-rewrite.fits", &file)?;

    let reopened = FitsSlice::from_slice(&fits.to_vec()?)?;
    let image = image_hdu(&reopened)
        .read_image(0)?
        .expect("the extension holds an image");

    let expected: Vec<i16> = (0..6)
        .flat_map(|row| (0..8).map(move |column| column * 3 + row))
        .collect();

    assert_eq!(raw_i16(&image), expected);

    Ok(())
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn a_compressed_image_streams_its_pixels() -> TestResult {
    use futures::StreamExt;

    let file = compressed_image(&gradient_cards(), &GRADIENT_TILES);
    let fits = open("compressed-stream.fits", &file)?;

    let pixels: Vec<_> = image_hdu(&fits)
        .stream_normalised_image(0)?
        .expect("the extension holds an image")
        .collect()
        .await;

    assert_eq!(pixels.len(), 8 * 6);

    // The corners of the gradient, normalised across its full range.
    assert_eq!(pixels[0].0, 0);
    assert_eq!(pixels[0].1, 0);
    assert_eq!(pixels[47].0, 7);
    assert_eq!(pixels[47].1, 5);

    Ok(())
}

#[test]
fn a_plio_compressed_mask_is_read() -> TestResult {
    // PLIO stores masks, and its instruction lists are 16-bit words rather than
    // bytes. These were produced by cfitsio: three zeros, three of the high
    // value, two zeros.
    let tile: &[u16] = &[
        0x0000, 0x0007, 0xff9c, 0x000a, 0x0000, 0x0000, 0x0000, 0x0003, 0x4003, 0x0002,
    ];

    let file = compressed_image_of_words(
        &[
            ("ZBITPIX", "32"),
            ("ZNAXIS", "2"),
            ("ZNAXIS1", "8"),
            ("ZNAXIS2", "1"),
            ("ZTILE1", "8"),
            ("ZTILE2", "1"),
            ("ZCMPTYPE", "'PLIO_1'"),
        ],
        &[tile],
    );

    let fits = open("compressed-plio.fits", &file)?;
    let image = image_hdu(&fits)
        .read_image(0)?
        .expect("the extension holds an image");

    let Image::I32(data) = &image else {
        panic!("expected a 32-bit image, got {image:?}");
    };
    assert_eq!(data.raw(), &[0, 0, 0, 1, 1, 1, 0, 0]);

    Ok(())
}

#[test]
fn a_plio_mask_with_several_levels_is_read() -> TestResult {
    // A segmentation mask rather than a boolean one, which exercises the
    // instructions that move the high value.
    let tile: &[u16] = &[
        0x0000, 0x0007, 0xff9c, 0x000d, 0x0000, 0x0000, 0x0000, 0x5002, 0x6001, 0x6001, 0x2002,
        0x0002, 0x4002,
    ];

    let file = compressed_image_of_words(
        &[
            ("ZBITPIX", "32"),
            ("ZNAXIS", "2"),
            ("ZNAXIS1", "8"),
            ("ZNAXIS2", "1"),
            ("ZTILE1", "8"),
            ("ZTILE2", "1"),
            ("ZCMPTYPE", "'PLIO_1'"),
        ],
        &[tile],
    );

    let fits = open("compressed-plio-levels.fits", &file)?;
    let image = image_hdu(&fits)
        .read_image(0)?
        .expect("the extension holds an image");

    let Image::I32(data) = &image else {
        panic!("expected a 32-bit image, got {image:?}");
    };
    assert_eq!(data.raw(), &[0, 1, 2, 3, 0, 0, 5, 5]);

    Ok(())
}

/// An 8x8 patch of a noisy gradient, quantised with SUBTRACTIVE_DITHER_1 and
/// Rice coded by cfitsio through astropy, with the scale and zero point it
/// chose.
const DITHERED_TILE: &[u8] = &[
    0x00, 0x00, 0x00, 0x2d, 0x34, 0x07, 0x61, 0x86, 0xce, 0x1d, 0x46, 0x2a, 0x40, 0x0d, 0x96, 0xa2,
    0x68, 0xa9, 0x31, 0x41, 0x77, 0xc7, 0x18, 0x3c, 0x10, 0x6d, 0xa3, 0xe2, 0x83, 0xeb, 0xdb, 0x1c,
    0x4a, 0x48, 0x63, 0xa5, 0x49, 0x09, 0x1b, 0x82, 0xaa, 0x50, 0x0b, 0x88, 0x30, 0x40, 0x6c, 0xc8,
    0x41, 0xf2, 0x43, 0x0a, 0xbc, 0x12, 0x32, 0xf1, 0x01, 0xa7, 0xe1, 0xb2, 0xcb, 0x45, 0x1a, 0x0a,
    0x80,
];

/// What cfitsio gives back for that tile, value for value.
///
/// These are the exact `f32` values, written out in full: they are what the
/// decoder must produce, not values it must come close to.
#[allow(clippy::excessive_precision)]
const DITHERED_PIXELS: [f32; 64] = [
    100.61231994628906,
    99.72648620605469,
    101.11946868896484,
    101.33004760742188,
    101.86827850341797,
    102.44025421142578,
    102.39576721191406,
    103.42288970947266,
    103.73676300048828,
    105.50331115722656,
    105.05823516845703,
    105.38787078857422,
    105.90711212158203,
    106.2983169555664,
    106.67451477050781,
    107.3752212524414,
    108.14076232910156,
    108.41962432861328,
    109.27894592285156,
    109.43091583251953,
    110.01355743408203,
    110.95561218261719,
    111.1554183959961,
    111.35071563720703,
    111.95259857177734,
    112.65882873535156,
    113.57994079589844,
    113.41844940185547,
    113.93260955810547,
    114.80259704589844,
    114.73983001708984,
    115.40300750732422,
    116.27352142333984,
    116.67444610595703,
    117.03278350830078,
    117.69288635253906,
    117.1422348022461,
    118.80242919921875,
    118.70825958251953,
    119.00385284423828,
    120.08897399902344,
    120.70149230957031,
    120.85721588134766,
    121.18006896972656,
    122.01558685302734,
    122.4863510131836,
    123.419921875,
    123.71931457519531,
    124.06715393066406,
    124.83924102783203,
    124.93643188476562,
    125.21769714355469,
    126.17697143554688,
    126.66522216796875,
    126.93492126464844,
    127.26007843017578,
    128.076416015625,
    127.75227355957031,
    129.21218872070312,
    129.6477508544922,
    129.51173400878906,
    130.52471923828125,
    130.71304321289062,
    131.73193359375,
];

/// Builds a compressed-image extension whose single tile is a quantised
/// floating point one, with the ZSCALE and ZZERO columns that scale it back.
fn quantised_image(cards: &[(&str, &str)], tile: &[u8], scale: f64, zero: f64) -> Vec<u8> {
    let mut rows = Vec::new();
    rows.extend_from_slice(&(tile.len() as i32).to_be_bytes());
    rows.extend_from_slice(&0_i32.to_be_bytes());
    rows.extend_from_slice(&scale.to_be_bytes());
    rows.extend_from_slice(&zero.to_be_bytes());

    let pcount = tile.len().to_string();

    let mut data = rows;
    data.extend_from_slice(tile);

    let mut file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "0"),
            ("EXTEND", "T"),
        ],
        &[],
    );

    let mut header = vec![
        ("XTENSION", "'BINTABLE'"),
        ("BITPIX", "8"),
        ("NAXIS", "2"),
        ("NAXIS1", "24"),
        ("NAXIS2", "1"),
        ("PCOUNT", pcount.as_str()),
        ("GCOUNT", "1"),
        ("TFIELDS", "3"),
        ("TTYPE1", "'COMPRESSED_DATA'"),
        ("TFORM1", "'1PB'"),
        ("TTYPE2", "'ZSCALE'"),
        ("TFORM2", "'1D'"),
        ("TTYPE3", "'ZZERO'"),
        ("TFORM3", "'1D'"),
        ("ZIMAGE", "T"),
    ];
    header.extend_from_slice(cards);

    append_extension(&mut file, &header, &data);

    file
}

#[test]
fn a_dithered_floating_point_image_matches_the_reference_implementation() -> TestResult {
    // Quantisation with subtractive dithering adds a known pseudo-random number
    // to each value before rounding it, and a reader has to take the same
    // number off again. Getting the sequence even slightly wrong gives an image
    // that looks entirely reasonable and is wrong everywhere by a fraction of a
    // quantisation step.
    let file = quantised_image(
        &[
            ("ZBITPIX", "-32"),
            ("ZNAXIS", "2"),
            ("ZNAXIS1", "8"),
            ("ZNAXIS2", "8"),
            ("ZTILE1", "8"),
            ("ZTILE2", "8"),
            ("ZCMPTYPE", "'RICE_1'"),
            ("ZNAME1", "'BLOCKSIZE'"),
            ("ZVAL1", "32"),
            ("ZNAME2", "'BYTEPIX'"),
            ("ZVAL2", "4"),
            ("ZQUANTIZ", "'SUBTRACTIVE_DITHER_1'"),
            ("ZDITHER0", "100"),
        ],
        DITHERED_TILE,
        0.01943078374328613,
        99.7382129542877,
    );

    let fits = open("compressed-dithered.fits", &file)?;
    let image = image_hdu(&fits)
        .read_image(0)?
        .expect("the extension holds an image");

    let Image::F32(data) = &image else {
        panic!("expected a 32-bit float image, got {image:?}");
    };

    assert_eq!(data.raw(), &DITHERED_PIXELS);

    Ok(())
}

#[test]
fn a_header_asking_for_a_quantisation_this_crate_does_not_know_says_so() -> TestResult {
    // Undithering with the wrong sequence produces an image that is wrong
    // everywhere and looks right, which is the worst thing a reader can do.
    let file = quantised_image(
        &[
            ("ZBITPIX", "-32"),
            ("ZNAXIS", "2"),
            ("ZNAXIS1", "8"),
            ("ZNAXIS2", "8"),
            ("ZTILE1", "8"),
            ("ZTILE2", "8"),
            ("ZCMPTYPE", "'RICE_1'"),
            ("ZNAME1", "'BLOCKSIZE'"),
            ("ZVAL1", "32"),
            ("ZNAME2", "'BYTEPIX'"),
            ("ZVAL2", "4"),
            ("ZQUANTIZ", "'SOMETHING_ELSE'"),
        ],
        DITHERED_TILE,
        1.0,
        0.0,
    );

    let fits = open("compressed-unknown-quantisation.fits", &file)?;
    let error = image_hdu(&fits)
        .read_image(0)
        .expect_err("an unknown quantisation method is not guessed at");

    assert!(error.to_string().contains("SOMETHING_ELSE"), "got: {error}");

    Ok(())
}
