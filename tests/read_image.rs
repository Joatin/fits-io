mod common;

use common::{fits_file, write_temp_fits};
use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::ImageHDU;
use fits_io::image::Image;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

fn raw_bytes(image: &Image) -> Vec<u8> {
    match image {
        Image::U8(data) => data.raw().to_vec(),
        other => panic!("expected an 8-bit image, got {other:?}"),
    }
}

#[test]
fn reads_a_two_axis_image() -> TestResult {
    let data: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "3"),
            ("NAXIS2", "2"),
        ],
        &data,
    );
    let path = write_temp_fits("two-axis.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let hdu = fits.primary_hdu();

    assert_eq!(hdu.image_count(), 1);
    assert_eq!((hdu.images_width(), hdu.images_height()), (3, 2));

    let image = hdu.read_image(0)?.expect("image 0 exists");
    assert_eq!((image.width(), image.height()), (3, 2));
    assert_eq!(raw_bytes(&image), data);

    // There is no second image in a 2-axis HDU.
    assert!(hdu.read_image(1)?.is_none());

    Ok(())
}

#[test]
fn reads_every_plane_of_a_three_axis_cube() -> TestResult {
    // Three 2x2 planes. Reading plane n used to panic outright, because the
    // bounds check looked at NAXIS4 rather than NAXIS3.
    let planes: [[u8; 4]; 3] = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];
    let data: Vec<u8> = planes.iter().flatten().copied().collect();

    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "3"),
            ("NAXIS1", "2"),
            ("NAXIS2", "2"),
            ("NAXIS3", "3"),
        ],
        &data,
    );
    let path = write_temp_fits("cube.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let hdu = fits.primary_hdu();

    assert_eq!(hdu.image_count(), 3);

    for (index, expected) in planes.iter().enumerate() {
        let image = hdu
            .read_image(index)?
            .unwrap_or_else(|| panic!("plane {index} exists"));

        assert_eq!((image.width(), image.height()), (2, 2), "plane {index}");
        assert_eq!(raw_bytes(&image), expected, "plane {index}");
    }

    // One past the end is absent, not an error and not a panic.
    assert!(hdu.read_image(3)?.is_none());
    assert!(hdu.read_image(usize::MAX)?.is_none());

    Ok(())
}

#[test]
fn a_header_only_hdu_has_no_images() -> TestResult {
    // A primary HDU with NAXIS = 0 is the normal shape for a file whose data
    // lives in extensions. Reading image 0 used to panic on a missing NAXIS1.
    let file = fits_file(&[("SIMPLE", "T"), ("BITPIX", "8"), ("NAXIS", "0")], &[]);
    let path = write_temp_fits("header-only.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let hdu = fits.primary_hdu();

    assert_eq!(hdu.image_count(), 0);
    assert_eq!(hdu.image_data_size(), 0);
    assert!(hdu.read_image(0)?.is_none());

    Ok(())
}

#[test]
fn reads_an_image_without_bzero_or_bscale_cards() -> TestResult {
    // BZERO and BSCALE are optional, with standard defaults of 0.0 and 1.0.
    // Reading a file that omits them used to panic.
    let data: Vec<u8> = vec![10, 20, 30, 40];
    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "2"),
            ("NAXIS2", "2"),
        ],
        &data,
    );
    let path = write_temp_fits("no-scaling.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let image = fits.primary_hdu().read_image(0)?.expect("image 0 exists");

    assert_eq!(raw_bytes(&image), data);

    Ok(())
}

#[test]
fn a_truncated_data_section_is_an_error() -> TestResult {
    // The header promises 4x4 pixels but the file stops after the header block.
    let mut file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "4"),
            ("NAXIS2", "4"),
        ],
        &[],
    );
    file.truncate(2880);
    let path = write_temp_fits("truncated.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let error = fits
        .primary_hdu()
        .read_image(0)
        .expect_err("a truncated image must not read as a short image");

    assert!(
        error.to_string().contains("ended"),
        "error should mention the short file, got: {error}"
    );

    Ok(())
}
