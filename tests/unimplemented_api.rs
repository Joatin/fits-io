//! The parts of the API that are not finished must report that, not panic.

mod common;

use common::{fits_file, write_temp_fits};
use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::ImageHDU;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

fn open_minimal(name: &str) -> Result<FsFits, Box<dyn Error + Send + Sync>> {
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
    let path = write_temp_fits(name, &file)?;

    FsFits::open(&path)
}

#[test]
fn writing_a_file_reports_that_it_is_unimplemented() -> TestResult {
    let fits = open_minimal("write.fits")?;

    let Err(error) = fits.save() else {
        panic!("save() must not claim success");
    };
    assert!(
        error.to_string().contains("not implemented"),
        "got: {error}"
    );

    let Err(error) = fits.to_vec() else {
        panic!("to_vec() must not claim success");
    };
    assert!(
        error.to_string().contains("not implemented"),
        "got: {error}"
    );

    Ok(())
}

#[test]
fn writing_image_data_reports_that_it_is_unimplemented() -> TestResult {
    let mut fits = open_minimal("write-image.fits")?;
    let hdu = fits.primary_hdu_mut();

    assert!(hdu.clear_images().is_err());
    assert!(hdu.set_raw_images_u8(1, 1, &[&[0]]).is_err());
    assert!(hdu.set_raw_images_i16(1, 1, &[&[0]]).is_err());
    assert!(hdu.set_raw_images_i32(1, 1, &[&[0]]).is_err());
    assert!(hdu.set_raw_images_f32(1, 1, &[&[0.0]]).is_err());
    assert!(hdu.set_raw_images_f64(1, 1, &[&[0.0]]).is_err());

    Ok(())
}

#[cfg(feature = "serde")]
#[test]
fn serializing_a_binary_table_reports_that_it_is_unimplemented() {
    #[derive(serde::Serialize)]
    struct Row {
        value: i32,
    }

    // This used to return an empty table, silently discarding the input.
    let Err(error) = fits_io::bin_table::to_bin_table(&Row { value: 1 }) else {
        panic!("to_bin_table() must not claim success");
    };

    assert!(
        error.to_string().contains("not implemented"),
        "got: {error}"
    );
}
