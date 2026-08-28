//! Malformed input must produce errors, never panics and never hangs.

mod common;

use common::{BLOCK, card, fits_file, write_temp_fits};
use fits_io::fs::FsFits;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

fn open_error(name: &str, contents: &[u8]) -> Result<String, Box<dyn Error + Send + Sync>> {
    let path = write_temp_fits(name, contents)?;

    match FsFits::open(&path) {
        Ok(_) => panic!("{name} should not have opened successfully"),
        Err(error) => Ok(error.to_string()),
    }
}

#[test]
fn a_header_without_bitpix_is_rejected() -> TestResult {
    let file = fits_file(&[("SIMPLE", "T"), ("NAXIS", "0")], &[]);
    let error = open_error("no-bitpix.fits", &file)?;

    assert!(error.contains("BITPIX"), "got: {error}");

    Ok(())
}

#[test]
fn a_header_without_naxis_is_rejected() -> TestResult {
    let file = fits_file(&[("SIMPLE", "T"), ("BITPIX", "8")], &[]);
    let error = open_error("no-naxis.fits", &file)?;

    assert!(error.contains("NAXIS"), "got: {error}");

    Ok(())
}

#[test]
fn a_header_missing_one_of_its_axis_lengths_is_rejected() -> TestResult {
    // NAXIS says two axes but only NAXIS1 is present, so the data section length
    // cannot be computed.
    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "4"),
        ],
        &[],
    );
    let error = open_error("missing-naxis2.fits", &file)?;

    assert!(error.contains("NAXIS2"), "got: {error}");

    Ok(())
}

#[test]
fn a_negative_axis_count_is_rejected() -> TestResult {
    let file = fits_file(&[("SIMPLE", "T"), ("BITPIX", "8"), ("NAXIS", "-1")], &[]);
    let error = open_error("negative-naxis.fits", &file)?;

    assert!(error.contains("negative"), "got: {error}");

    Ok(())
}

#[test]
fn a_negative_axis_length_is_rejected() -> TestResult {
    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "1"),
            ("NAXIS1", "-4"),
        ],
        &[],
    );
    let error = open_error("negative-naxis1.fits", &file)?;

    assert!(error.contains("negative"), "got: {error}");

    Ok(())
}

#[test]
fn a_zero_indexed_axis_keyword_is_rejected() -> TestResult {
    // NAXIS0 has no 0-based equivalent. Converting the index used to underflow.
    let mut header = String::new();
    header.push_str(&card("SIMPLE", "T"));
    header.push_str(&card("BITPIX", "8"));
    header.push_str(&card("NAXIS", "1"));
    header.push_str(&card("NAXIS0", "4"));
    header.push_str(&format!("{:<80}", "END"));

    let mut file = header.into_bytes();
    file.resize(BLOCK, b' ');

    let error = open_error("naxis0.fits", &file)?;
    assert!(
        error.contains("NAXIS0") || error.contains("index"),
        "got: {error}"
    );

    Ok(())
}

#[test]
fn a_non_numeric_index_keyword_is_rejected() -> TestResult {
    let mut header = String::new();
    header.push_str(&card("SIMPLE", "T"));
    header.push_str(&card("BITPIX", "8"));
    header.push_str(&card("NAXIS", "1"));
    header.push_str(&card("NAXISX", "4"));
    header.push_str(&format!("{:<80}", "END"));

    let mut file = header.into_bytes();
    file.resize(BLOCK, b' ');

    let error = open_error("naxisx.fits", &file)?;
    assert!(
        error.contains("NAXISX") || error.contains("Invalid index"),
        "got: {error}"
    );

    Ok(())
}

#[test]
fn a_file_that_is_not_fits_is_rejected() -> TestResult {
    let error = open_error("garbage.fits", &vec![0xFF_u8; BLOCK])?;

    assert!(!error.is_empty());

    Ok(())
}

#[test]
fn a_header_without_simple_is_rejected() -> TestResult {
    let file = fits_file(&[("BITPIX", "8"), ("NAXIS", "0")], &[]);
    let error = open_error("no-simple.fits", &file)?;

    assert!(error.contains("SIMPLE"), "got: {error}");

    Ok(())
}
