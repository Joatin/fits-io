//! `FitsSlice` reads a file that is already in memory, with no filesystem
//! involved. It has to agree with the filesystem reader on what a file says.

mod common;

use common::{append_extension, fits_file};
use fits_io::hdu::{BinTableHDU, ExtensionHDU, HDU, ImageHDU};
use fits_io::image::Image;
use fits_io::{Fits, FitsSlice};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

fn raw_u8(image: &Image) -> Vec<u8> {
    match image {
        Image::U8(data) => data.raw().to_vec(),
        other => panic!("expected an 8-bit image, got {other:?}"),
    }
}

#[test]
fn an_image_reads_out_of_a_buffer() -> TestResult {
    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "2"),
            ("NAXIS2", "2"),
        ],
        &[1, 2, 3, 4],
    );

    let fits = FitsSlice::from_slice(&file)?;

    assert_eq!(fits.primary_hdu().image_count(), 1);
    assert_eq!(
        raw_u8(&fits.primary_hdu().read_image(0)?.expect("one image")),
        vec![1, 2, 3, 4]
    );

    Ok(())
}

#[test]
fn extensions_read_out_of_a_buffer() -> TestResult {
    let mut file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "0"),
            ("EXTEND", "T"),
        ],
        &[],
    );

    let mut data = Vec::new();
    for value in [10_i32, 20] {
        data.extend_from_slice(&value.to_be_bytes());
    }

    append_extension(
        &mut file,
        &[
            ("XTENSION", "'BINTABLE'"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "4"),
            ("NAXIS2", "2"),
            ("PCOUNT", "0"),
            ("GCOUNT", "1"),
            ("TFIELDS", "1"),
            ("TFORM1", "'1J'"),
            ("TTYPE1", "'VALUE'"),
        ],
        &data,
    );

    let fits = FitsSlice::from_slice(&file)?;

    assert_eq!(fits.extension_count(), 1);

    let Some(ExtensionHDU::BinTable(hdu)) = fits.extension_hdu(0) else {
        panic!("the extension is a binary table");
    };

    let table = hdu.read_table()?;
    assert_eq!(table.len(), 2);

    Ok(())
}

#[test]
fn a_buffer_round_trips_through_the_writer() -> TestResult {
    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "2"),
            ("NAXIS2", "2"),
        ],
        &[5, 6, 7, 8],
    );

    let fits = FitsSlice::from_slice(&file)?;
    let written = fits.to_vec()?;

    let reopened = FitsSlice::from_slice(&written)?;

    assert_eq!(
        raw_u8(&reopened.primary_hdu().read_image(0)?.expect("one image")),
        vec![5, 6, 7, 8]
    );

    Ok(())
}

#[test]
fn a_file_can_be_built_from_nothing() -> TestResult {
    let mut fits = FitsSlice::new();

    fits.primary_hdu_mut()
        .set_raw_images_u8(2, 2, &[&[1, 2, 3, 4]])?;

    // A primary header needs SIMPLE before anything will read it back.
    assert_eq!(fits.primary_hdu().header().naxis(), Some(2));
    assert_eq!(fits.primary_hdu().image_count(), 1);
    assert_eq!(
        raw_u8(&fits.primary_hdu().read_image(0)?.expect("one image")),
        vec![1, 2, 3, 4]
    );

    Ok(())
}

#[test]
fn a_buffer_that_is_not_fits_is_rejected() {
    // `from_slice` used to accept anything and hand back an empty file.
    assert!(FitsSlice::from_slice(b"this is not a FITS file").is_err());
    assert!(FitsSlice::from_slice(&[]).is_err());
}

#[test]
fn a_file_built_from_nothing_is_readable() -> TestResult {
    // A header assembled by hand has no SIMPLE card and no ordering, and the
    // standard requires both. Writing one out without fixing that produces a
    // file that this crate — and every other reader — refuses to open.
    let mut fits = FitsSlice::new();

    fits.primary_hdu_mut()
        .set_raw_images_u8(2, 2, &[&[1, 2, 3, 4]])?;

    let bytes = fits.to_vec()?;
    let reopened = FitsSlice::from_slice(&bytes)?;

    assert_eq!(
        raw_u8(&reopened.primary_hdu().read_image(0)?.expect("one image")),
        vec![1, 2, 3, 4]
    );

    Ok(())
}

#[test]
fn a_written_header_opens_with_the_mandatory_cards_in_order() -> TestResult {
    let mut fits = FitsSlice::new();
    fits.primary_hdu_mut()
        .set_raw_images_u8(2, 2, &[&[1, 2, 3, 4]])?;

    let bytes = fits.to_vec()?;
    let keywords: Vec<String> = bytes
        .chunks(80)
        .take(5)
        .map(|card| String::from_utf8_lossy(&card[..8]).trim_end().to_string())
        .collect();

    assert_eq!(
        keywords,
        vec!["SIMPLE", "BITPIX", "NAXIS", "NAXIS1", "NAXIS2"]
    );

    Ok(())
}

#[cfg(feature = "gzip")]
#[test]
fn a_gzipped_buffer_is_decompressed_transparently() -> TestResult {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "2"),
            ("NAXIS2", "2"),
        ],
        &[1, 2, 3, 4],
    );

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&file)?;
    let compressed = encoder.finish()?;

    // A FITS file starts with `SIMPLE`, so it can never be confused with one
    // that starts with the gzip marker.
    let fits = FitsSlice::from_slice(&compressed)?;

    assert_eq!(
        raw_u8(&fits.primary_hdu().read_image(0)?.expect("one image")),
        vec![1, 2, 3, 4]
    );

    Ok(())
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn table_rows_stream_out_of_a_buffer() -> TestResult {
    use fits_io::hdu::BinTableHDU;
    use futures::StreamExt;

    let mut file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "0"),
            ("EXTEND", "T"),
        ],
        &[],
    );

    let mut data = Vec::new();
    for value in 0..3_i32 {
        data.extend_from_slice(&value.to_be_bytes());
    }

    append_extension(
        &mut file,
        &[
            ("XTENSION", "'BINTABLE'"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "4"),
            ("NAXIS2", "3"),
            ("PCOUNT", "0"),
            ("GCOUNT", "1"),
            ("TFIELDS", "1"),
            ("TFORM1", "'1J'"),
            ("TTYPE1", "'VALUE'"),
        ],
        &data,
    );

    let fits = FitsSlice::from_slice(&file)?;
    let Some(ExtensionHDU::BinTable(hdu)) = fits.extension_hdu(0) else {
        panic!("the extension is a binary table");
    };

    let rows: Vec<_> = hdu.stream_table_rows_raw()?.collect().await;

    assert_eq!(rows.len(), 3);
    for (index, row) in rows.iter().enumerate() {
        assert!(
            matches!(row.get("VALUE")?, Some(fits_io::bin_table::Value::I32(ref v)) if v == &[index as i32]),
            "row {index} got {:?}",
            row.get("VALUE")?
        );
    }

    Ok(())
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn an_image_streams_out_of_a_buffer() -> TestResult {
    use futures::StreamExt;

    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "2"),
            ("NAXIS2", "2"),
        ],
        &[0, 85, 170, 255],
    );

    let fits = FitsSlice::from_slice(&file)?;
    let pixels: Vec<_> = fits
        .primary_hdu()
        .stream_normalised_image(0)?
        .expect("a two axis HDU holds one image")
        .collect()
        .await;

    assert_eq!(pixels.len(), 4);
    assert_eq!(pixels[0], (0, 0, 0.0));
    assert_eq!(pixels[3], (1, 1, 1.0));

    Ok(())
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn a_buffer_can_be_read_without_blocking_the_runtime() -> TestResult {
    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "2"),
            ("NAXIS2", "2"),
        ],
        &[1, 2, 3, 4],
    );

    let fits = FitsSlice::from_vec_async(file).await?;

    assert_eq!(
        raw_u8(&fits.primary_hdu().read_image(0)?.expect("one image")),
        vec![1, 2, 3, 4]
    );

    Ok(())
}
