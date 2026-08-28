//! Round-trips a minimal FITS file through both the plain and the gzipped open
//! paths. Guards against gzip detection silently failing and handing the parser
//! raw deflate bytes.

mod common;

use common::{fits_file, write_temp_fits};
use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::HDU;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::error::Error;
use std::io::Write;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

fn minimal_fits() -> Vec<u8> {
    fits_file(&[("SIMPLE", "T"), ("BITPIX", "8"), ("NAXIS", "0")], &[])
}

fn assert_opens_as_minimal_fits(path: &Path) -> TestResult {
    let fits = FsFits::open(path)?;
    let header = fits.primary_hdu().header();

    assert_eq!(header.simple(), Some(true), "SIMPLE for {path:?}");
    assert_eq!(header.naxis(), Some(0), "NAXIS for {path:?}");
    assert_eq!(fits.extension_hdus().count(), 0, "extensions for {path:?}");

    Ok(())
}

#[test]
fn opens_a_plain_fits_file() -> TestResult {
    let path = write_temp_fits("minimal.fits", &minimal_fits())?;

    assert_opens_as_minimal_fits(&path)
}

#[test]
fn opens_a_gzipped_fits_file() -> TestResult {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&minimal_fits())?;
    let path = write_temp_fits("minimal.fits.gz", &encoder.finish()?)?;

    assert_opens_as_minimal_fits(&path)
}
