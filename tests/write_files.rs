//! Writing a FITS file: a file that is read and written back must come out the
//! same, and data written into an HDU must survive the round trip.

mod common;

use common::{append_extension, fits_file, write_temp_fits};
use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::{HDU, ImageHDU};
use fits_io::image::Image;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

const BLOCK: usize = 2880;

fn open(name: &str, file: &[u8]) -> Result<FsFits, Box<dyn Error + Send + Sync>> {
    let path = write_temp_fits(name, file)?;
    FsFits::open(&path)
}

fn minimal_image() -> Vec<u8> {
    fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "2"),
            ("NAXIS2", "2"),
        ],
        &[1, 2, 3, 4],
    )
}

fn raw_u8(image: &Image) -> Vec<u8> {
    match image {
        Image::U8(data) => data.raw().to_vec(),
        other => panic!("expected an 8-bit image, got {other:?}"),
    }
}

#[test]
fn a_written_file_is_a_whole_number_of_blocks() -> TestResult {
    let fits = open("write-blocks.fits", &minimal_image())?;

    let bytes = fits.to_vec()?;

    assert!(!bytes.is_empty());
    assert_eq!(
        bytes.len() % BLOCK,
        0,
        "a FITS file is made of whole 2880-byte blocks, got {} bytes",
        bytes.len()
    );

    Ok(())
}

#[test]
fn a_file_that_is_read_and_written_back_still_reads_the_same() -> TestResult {
    let fits = open("write-roundtrip.fits", &minimal_image())?;
    let before = raw_u8(&fits.primary_hdu().read_image(0)?.expect("one image"));

    let path = write_temp_fits("written.fits", &fits.to_vec()?)?;
    let reopened = FsFits::open(&path)?;

    let after = raw_u8(&reopened.primary_hdu().read_image(0)?.expect("one image"));

    assert_eq!(before, after);
    assert_eq!(
        reopened.primary_hdu().header().naxis(),
        fits.primary_hdu().header().naxis()
    );

    Ok(())
}

#[test]
fn an_extension_survives_the_round_trip() -> TestResult {
    // The extension has to land on a block boundary, or a reader will not find
    // its header where the primary HDU's size says it should be.
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

    let fits = open("write-extension.fits", &file)?;
    let path = write_temp_fits("written-extension.fits", &fits.to_vec()?)?;
    let reopened = FsFits::open(&path)?;

    assert_eq!(reopened.extension_count(), 1);

    let Some(fits_io::hdu::ExtensionHDU::Image(hdu)) = reopened.extension_hdu(0) else {
        panic!("the extension is an image");
    };
    assert_eq!(
        raw_u8(&hdu.read_image(0)?.expect("one image")),
        vec![9, 8, 7, 6]
    );

    Ok(())
}

#[test]
fn image_data_written_into_an_hdu_survives_the_round_trip() -> TestResult {
    let mut fits = open("write-pixels.fits", &minimal_image())?;

    fits.primary_hdu_mut()
        .set_raw_images_i16(2, 2, &[&[-1, 0, 1, 300]])?;

    // The header must now describe the data that was written, not the data that
    // was read.
    assert_eq!(
        fits.primary_hdu().header().bitpix().map(i64::from),
        Some(16)
    );

    let path = write_temp_fits("written-pixels.fits", &fits.to_vec()?)?;
    let reopened = FsFits::open(&path)?;

    let image = reopened.primary_hdu().read_image(0)?.expect("one image");
    let Image::I16(data) = &image else {
        panic!("expected a 16-bit image, got {image:?}");
    };

    assert_eq!(data.raw(), &[-1, 0, 1, 300]);

    Ok(())
}

#[test]
fn a_three_axis_cube_can_be_written() -> TestResult {
    let mut fits = open("write-cube.fits", &minimal_image())?;

    fits.primary_hdu_mut()
        .set_raw_images_u8(2, 1, &[&[1, 2], &[3, 4], &[5, 6]])?;

    assert_eq!(fits.primary_hdu().image_count(), 3);

    let path = write_temp_fits("written-cube.fits", &fits.to_vec()?)?;
    let reopened = FsFits::open(&path)?;

    assert_eq!(reopened.primary_hdu().image_count(), 3);
    assert_eq!(
        raw_u8(
            &reopened
                .primary_hdu()
                .read_image(2)?
                .expect("a third plane")
        ),
        vec![5, 6]
    );

    Ok(())
}

#[test]
fn clearing_the_images_leaves_a_header_only_hdu() -> TestResult {
    let mut fits = open("write-cleared.fits", &minimal_image())?;

    fits.primary_hdu_mut().clear_images()?;

    let path = write_temp_fits("written-cleared.fits", &fits.to_vec()?)?;
    let reopened = FsFits::open(&path)?;

    assert_eq!(reopened.primary_hdu().image_count(), 0);
    assert!(reopened.primary_hdu().read_image(0)?.is_none());

    Ok(())
}

#[test]
fn a_ragged_set_of_images_is_rejected() -> TestResult {
    let mut fits = open("write-ragged.fits", &minimal_image())?;

    // Accepting this would write a data section that the NAXISn cards no longer
    // describe, producing a file that cannot be read back.
    let error = fits
        .primary_hdu_mut()
        .set_raw_images_u8(2, 2, &[&[1, 2, 3, 4], &[1, 2]])
        .expect_err("every image must be the declared size");

    assert!(error.to_string().contains("pixels"), "got: {error}");

    Ok(())
}

#[test]
fn save_writes_the_file_back_to_its_own_path() -> TestResult {
    let path = write_temp_fits("write-save.fits", &minimal_image())?;
    let mut fits = FsFits::open(&path)?;

    fits.primary_hdu_mut()
        .set_raw_images_u8(2, 2, &[&[10, 20, 30, 40]])?;
    fits.save()?;

    let reopened = FsFits::open(&path)?;

    assert_eq!(
        raw_u8(&reopened.primary_hdu().read_image(0)?.expect("one image")),
        vec![10, 20, 30, 40]
    );

    Ok(())
}

#[test]
fn an_image_buffer_keeps_its_own_height() -> TestResult {
    // The convenience wrappers read the dimensions off the first buffer. Reading
    // the width twice would write a square image from a rectangular one.
    let mut fits = open("write-buffer.fits", &minimal_image())?;

    let buffer = image::ImageBuffer::<image::Luma<u8>, Vec<u8>>::from_raw(4, 2, vec![1; 8])
        .expect("a 4x2 buffer of 8 pixels");

    fits.primary_hdu_mut().set_images_u8(&[&buffer])?;

    assert_eq!(fits.primary_hdu().images_width(), 4);
    assert_eq!(fits.primary_hdu().images_height(), 2);

    Ok(())
}

/// Splits a written file into its HDUs: each is a run of header blocks ending
/// with an END card, followed by the data section the header describes.
fn hdus(bytes: &[u8]) -> Vec<&[u8]> {
    let mut hdus = Vec::new();
    let mut offset = 0;

    while offset < bytes.len() {
        let header_start = offset;

        // Header blocks run until one contains an END card.
        loop {
            let block = &bytes[offset..offset + BLOCK];
            offset += BLOCK;

            if block.chunks(80).any(|card| card.starts_with(b"END ")) {
                break;
            }
        }

        let header = &bytes[header_start..offset];
        let data_len = data_length(header);
        offset += data_len.div_ceil(BLOCK) * BLOCK;

        hdus.push(&bytes[header_start..offset]);
    }

    hdus
}

/// The size of the data section a header describes, from its own cards.
fn data_length(header: &[u8]) -> usize {
    let card = |keyword: &str| -> Option<i64> {
        header.chunks(80).find_map(|card| {
            let text = String::from_utf8_lossy(card);
            let (key, value) = text.split_once('=')?;
            (key.trim() == keyword).then(|| {
                value
                    .split('/')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .parse::<i64>()
                    .ok()
            })?
        })
    };

    let Some(bitpix) = card("BITPIX") else {
        return 0;
    };
    let Some(axes) = card("NAXIS") else {
        return 0;
    };
    if axes <= 0 {
        return 0;
    }

    let mut elements = 1_usize;
    for axis in 1..=axes {
        elements *= card(&format!("NAXIS{axis}")).unwrap_or(0).max(0) as usize;
    }

    let pcount = card("PCOUNT").unwrap_or(0).max(0) as usize;
    let gcount = card("GCOUNT").unwrap_or(1).max(0) as usize;

    (elements + pcount) * gcount * (bitpix.unsigned_abs() as usize / 8)
}

#[test]
fn every_written_hdu_carries_a_correct_checksum() -> TestResult {
    // The convention is arranged so that summing a whole undamaged HDU, its own
    // CHECKSUM card included, gives all ones. That is the real test of the
    // encoding: an off-by-anything shows up here.
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
            ("XTENSION", "'IMAGE   '"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "3"),
            ("NAXIS2", "2"),
            ("PCOUNT", "0"),
            ("GCOUNT", "1"),
        ],
        &[1, 2, 3, 4, 5, 6],
    );

    let fits = open("write-checksum.fits", &file)?;
    let written = fits.to_vec()?;

    let hdus = hdus(&written);
    assert_eq!(hdus.len(), 2, "a primary HDU and one extension");

    for (index, hdu) in hdus.iter().enumerate() {
        assert!(
            fits_io::checksum::verify(hdu),
            "HDU {index} does not verify"
        );
    }

    Ok(())
}

#[test]
fn a_damaged_hdu_does_not_verify() -> TestResult {
    let fits = open("write-damaged.fits", &minimal_image())?;
    let mut written = fits.to_vec()?;

    assert!(fits_io::checksum::verify(hdus(&written)[0]));

    // Flip a bit in the data section.
    let last = written.len() - 1;
    written[last] ^= 0x01;

    assert!(
        !fits_io::checksum::verify(hdus(&written)[0]),
        "a changed byte must break the checksum"
    );

    Ok(())
}

#[test]
fn writing_a_smaller_image_clears_the_axes_it_no_longer_has() -> TestResult {
    // A header describing a four-axis array that is overwritten with a plain
    // image must not keep its NAXIS3 and NAXIS4 cards: they would contradict
    // NAXIS, and a reader is entitled to believe either.
    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "4"),
            ("NAXIS1", "2"),
            ("NAXIS2", "2"),
            ("NAXIS3", "2"),
            ("NAXIS4", "3"),
        ],
        &(1..=24).collect::<Vec<u8>>(),
    );

    let mut fits = open("write-shrink.fits", &file)?;
    fits.primary_hdu_mut()
        .set_raw_images_u8(2, 2, &[&[1, 2, 3, 4]])?;

    let path = write_temp_fits("written-shrink.fits", &fits.to_vec()?)?;
    let reopened = FsFits::open(&path)?;

    let header = reopened.primary_hdu().header();
    assert_eq!(header.naxis(), Some(2));
    assert_eq!(header.naxis_n(2), None, "NAXIS3 should be gone");
    assert_eq!(header.naxis_n(3), None, "NAXIS4 should be gone");
    assert_eq!(reopened.primary_hdu().image_count(), 1);

    Ok(())
}

#[test]
fn a_header_that_contradicts_its_data_is_not_written() -> TestResult {
    // The header is the only thing that says how to read what follows it. One
    // that disagrees produces a file nothing can read back: the array comes out
    // the wrong shape and the next HDU is looked for in the wrong place.
    let mut fits = open("write-lying.fits", &minimal_image())?;

    // Set a 2x2 image, then claim it is 4x2.
    fits.primary_hdu_mut()
        .set_raw_images_u8(2, 2, &[&[1, 2, 3, 4]])?;
    fits.primary_hdu_mut()
        .header_mut()
        .set_naxis_n(0, 4)
        .expect("NAXIS1 can be set");

    let error = fits
        .to_vec()
        .expect_err("a header that contradicts its data must not be written");

    assert!(
        error.to_string().contains("cannot be read back"),
        "got: {error}"
    );

    Ok(())
}

#[test]
fn a_header_that_matches_its_data_still_writes() -> TestResult {
    // The check must not object to the block padding every data section carries.
    let fits = open("write-honest.fits", &minimal_image())?;

    assert!(!fits.to_vec()?.is_empty());

    Ok(())
}
