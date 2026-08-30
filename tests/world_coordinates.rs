//! Reading a world coordinate system out of a header.

mod common;

use common::{fits_file, write_temp_fits};
use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::HDU;
use fits_io::wcs::{Projection, Wcs};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

fn open(name: &str, extra: &[(&str, &str)]) -> Result<FsFits, Box<dyn Error + Send + Sync>> {
    let mut cards = vec![
        ("SIMPLE", "T"),
        ("BITPIX", "8"),
        ("NAXIS", "2"),
        ("NAXIS1", "4"),
        ("NAXIS2", "4"),
    ];
    cards.extend_from_slice(extra);

    let file = fits_file(&cards, &[0; 16]);
    let path = write_temp_fits(name, &file)?;

    FsFits::open(&path)
}

#[test]
fn a_header_with_wcs_cards_maps_pixels_onto_the_sky() -> TestResult {
    let fits = open(
        "wcs.fits",
        &[
            ("CTYPE1", "'RA---TAN'"),
            ("CTYPE2", "'DEC--TAN'"),
            ("CRPIX1", "2.5"),
            ("CRPIX2", "2.5"),
            ("CRVAL1", "83.633"),
            ("CRVAL2", "22.0145"),
            ("CDELT1", "-0.001"),
            ("CDELT2", "0.001"),
        ],
    )?;

    let wcs = Wcs::from_header(fits.primary_hdu().header())?;

    assert_eq!(wcs.projection(), Projection::Gnomonic);

    // The reference pixel sits at the reference value.
    let (ra, dec) = wcs.pixel_to_world((2.5, 2.5));
    assert!((ra - 83.633).abs() < 1e-9, "got {ra}");
    assert!((dec - 22.0145).abs() < 1e-9, "got {dec}");

    // And every pixel round-trips.
    let world = wcs.pixel_to_world_indexed((3, 1));
    assert_eq!(wcs.world_to_pixel_indexed(world, 4, 4), Some((3, 1)));

    Ok(())
}

#[test]
fn a_header_without_wcs_cards_says_so() -> TestResult {
    let fits = open("no-wcs.fits", &[])?;

    let error = Wcs::from_header(fits.primary_hdu().header())
        .expect_err("a header with no CRPIX or CRVAL carries no WCS");

    assert!(error.to_string().contains("CRPIX1"), "got: {error}");

    Ok(())
}

#[test]
fn an_integer_valued_wcs_card_is_accepted() -> TestResult {
    // `CRPIX1 = 2` rather than `2.0` is a perfectly ordinary way to write a
    // whole number, and used to fail the whole header.
    let fits = open(
        "wcs-integers.fits",
        &[
            ("CTYPE1", "'RA---TAN'"),
            ("CTYPE2", "'DEC--TAN'"),
            ("CRPIX1", "2"),
            ("CRPIX2", "2"),
            ("CRVAL1", "180"),
            ("CRVAL2", "0"),
        ],
    )?;

    let wcs = Wcs::from_header(fits.primary_hdu().header())?;
    let (ra, dec) = wcs.pixel_to_world((2.0, 2.0));

    assert!((ra - 180.0).abs() < 1e-9, "got {ra}");
    assert!(dec.abs() < 1e-9, "got {dec}");

    Ok(())
}
