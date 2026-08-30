//! The in-memory reader and the filesystem reader must agree. They share the
//! header and decoding code but walk HDUs separately, so a file that reads one
//! way and not the other means one of the two walks is wrong.

mod common;

use common::{append_extension, fits_file, fixture, write_temp_fits};
use fits_io::fs::FsFits;
use fits_io::hdu::{BinTableHDU, ExtensionHDU, HDU, ImageHDU};
use fits_io::image::Image;
use fits_io::{Fits, FitsSlice};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

fn raw(image: &Image) -> Vec<u8> {
    match image {
        Image::U8(data) => data.raw().to_vec(),
        other => panic!("expected an 8-bit image, got {other:?}"),
    }
}

/// A file with a primary image, an image extension and a binary table.
fn multi_hdu_file() -> Vec<u8> {
    let mut file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "2"),
            ("NAXIS2", "2"),
            ("EXTEND", "T"),
        ],
        &[1, 2, 3, 4],
    );

    append_extension(
        &mut file,
        &[
            ("XTENSION", "'IMAGE   '"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "3"),
            ("NAXIS2", "1"),
            ("PCOUNT", "0"),
            ("GCOUNT", "1"),
        ],
        &[9, 8, 7],
    );

    let mut rows = Vec::new();
    for value in 0..2_i32 {
        rows.extend_from_slice(&value.to_be_bytes());
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
        &rows,
    );

    file
}

#[test]
fn both_readers_find_the_same_hdus() -> TestResult {
    let file = multi_hdu_file();
    let path = write_temp_fits("parity-multi.fits", &file)?;

    let from_file = FsFits::open(&path)?;
    let from_buffer = FitsSlice::from_slice(&file)?;

    assert_eq!(from_file.extension_count(), 3 - 1);
    assert_eq!(from_file.extension_count(), from_buffer.extension_count());

    // The primary image.
    assert_eq!(
        raw(&from_file.primary_hdu().read_image(0)?.expect("one image")),
        raw(&from_buffer.primary_hdu().read_image(0)?.expect("one image"))
    );

    // The image extension, which only lines up if both walks agree on where the
    // primary HDU ended.
    let (ExtensionHDU::Image(a), ExtensionHDU::Image(b)) = (
        from_file.extension_hdu(0).expect("an image extension"),
        from_buffer.extension_hdu(0).expect("an image extension"),
    ) else {
        panic!("the first extension is an image in both readers");
    };
    assert_eq!(
        raw(&a.read_image(0)?.expect("one image")),
        raw(&b.read_image(0)?.expect("one image"))
    );

    // And the table after it.
    let (ExtensionHDU::BinTable(a), ExtensionHDU::BinTable(b)) = (
        from_file.extension_hdu(1).expect("a table extension"),
        from_buffer.extension_hdu(1).expect("a table extension"),
    ) else {
        panic!("the second extension is a binary table in both readers");
    };
    assert_eq!(a.read_table()?.len(), b.read_table()?.len());
    assert_eq!(a.header().table_format(0), b.header().table_format(0));

    Ok(())
}

#[test]
fn both_readers_reject_the_same_files() -> TestResult {
    let truncated = {
        let mut file = fits_file(
            &[
                ("SIMPLE", "T"),
                ("BITPIX", "8"),
                ("NAXIS", "2"),
                ("NAXIS1", "100"),
                ("NAXIS2", "100"),
            ],
            &[],
        );
        file.truncate(2880);
        file
    };

    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "no BITPIX",
            fits_file(&[("SIMPLE", "T"), ("NAXIS", "0")], &[]),
        ),
        (
            "no SIMPLE",
            fits_file(&[("BITPIX", "8"), ("NAXIS", "0")], &[]),
        ),
        (
            "negative NAXIS",
            fits_file(&[("SIMPLE", "T"), ("BITPIX", "8"), ("NAXIS", "-1")], &[]),
        ),
        ("not a FITS file", b"hello world".to_vec()),
        ("empty", Vec::new()),
        ("truncated data section", truncated),
    ];

    for (name, bytes) in cases {
        let path = write_temp_fits("parity-bad.fits", &bytes)?;

        let from_file = FsFits::open(&path).is_ok();
        let from_buffer = FitsSlice::from_slice(&bytes).is_ok();

        assert_eq!(
            from_file, from_buffer,
            "{name}: the file reader says {from_file} and the buffer reader {from_buffer}"
        );
    }

    Ok(())
}

#[test]
fn both_readers_agree_on_a_real_file() -> TestResult {
    let Some(path) = fixture("gaia-dr3-mag-gt-12.fits") else {
        return Ok(());
    };

    let bytes = std::fs::read(&path)?;

    let from_file = FsFits::open(&path)?;
    let from_buffer = FitsSlice::from_slice(&bytes)?;

    assert_eq!(from_file.extension_count(), from_buffer.extension_count());

    let (ExtensionHDU::BinTable(a), ExtensionHDU::BinTable(b)) = (
        from_file.extension_hdu(0).expect("a table extension"),
        from_buffer.extension_hdu(0).expect("a table extension"),
    ) else {
        panic!("the first extension is a binary table in both readers");
    };

    let (a, b) = (a.read_table()?, b.read_table()?);
    assert_eq!(a.len(), b.len());
    assert_eq!(a.bytes_per_row(), b.bytes_per_row());

    // Comparing a row proves the two agree on where the data section starts, not
    // merely on how long it is.
    let (first, second) = (a.row(0).expect("a row"), b.row(0).expect("a row"));
    for column in 0..a.field_definitions().len() {
        assert_eq!(
            format!("{:?}", first.get_at(column)?),
            format!("{:?}", second.get_at(column)?),
            "column {column} differs"
        );
    }

    Ok(())
}
