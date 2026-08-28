//! Extension HDU discovery, including the cards that are and are not mandatory.

mod common;

use common::write_temp_fits;
use common::{append_extension, fits_file};
use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::{ExtensionHDU, HDU};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

fn primary() -> Vec<u8> {
    fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "0"),
            ("EXTEND", "T"),
        ],
        &[],
    )
}

#[test]
fn an_extension_without_extname_is_still_valid() -> TestResult {
    // EXTNAME is optional in the FITS standard. Validation used to require it,
    // which rejected conforming files.
    let mut file = primary();
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
        &[1, 2, 3, 4],
    );
    let path = write_temp_fits("no-extname.fits", &file)?;

    let fits = FsFits::open(&path)?;

    assert_eq!(fits.extension_hdus().count(), 1);
    assert!(matches!(
        fits.extension_hdu(0),
        Some(ExtensionHDU::Image(_))
    ));
    assert_eq!(
        fits.extension_hdu(0).and_then(|hdu| match hdu {
            ExtensionHDU::Image(image) => image.header().extension_name(),
            _ => None,
        }),
        None,
        "the extension really does carry no EXTNAME"
    );

    Ok(())
}

#[test]
fn an_extension_without_xtension_is_rejected() -> TestResult {
    // XTENSION is the card that is actually mandatory.
    let mut file = primary();
    append_extension(
        &mut file,
        &[("EXTNAME", "'NAMED   '"), ("BITPIX", "8"), ("NAXIS", "0")],
        &[],
    );
    let path = write_temp_fits("no-xtension.fits", &file)?;

    let Err(error) = FsFits::open(&path) else {
        panic!("an extension without XTENSION must be rejected");
    };

    assert!(error.to_string().contains("XTENSION"), "got: {error}");

    Ok(())
}

#[test]
fn an_extension_keeps_its_extname_when_present() -> TestResult {
    let mut file = primary();
    append_extension(
        &mut file,
        &[
            ("XTENSION", "'IMAGE   '"),
            ("BITPIX", "8"),
            ("NAXIS", "0"),
            ("EXTNAME", "'SCI     '"),
        ],
        &[],
    );
    let path = write_temp_fits("named.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let Some(ExtensionHDU::Image(image)) = fits.extension_hdu(0) else {
        panic!("expected one image extension");
    };

    assert_eq!(image.header().extension_name(), Some("SCI"));

    Ok(())
}
