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

#[test]
fn blank_pixels_read_as_undefined_rather_than_black() -> TestResult {
    // BLANK names the raw value that stands for "no data here". Without it, the
    // sentinel is just a very small number and clamps to 0.0, which is
    // indistinguishable from a genuinely black pixel.
    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "16"),
            ("NAXIS", "2"),
            ("NAXIS1", "2"),
            ("NAXIS2", "1"),
            ("BLANK", "-32768"),
            ("DATAMIN", "0"),
            ("DATAMAX", "100"),
        ],
        &[0x80, 0x00, 0x00, 0x32],
    );
    let path = write_temp_fits("blank.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let image = fits
        .primary_hdu()
        .read_image(0)?
        .expect("a two axis HDU holds one image");

    let normalized = image.normalized();

    assert!(
        normalized.get_pixel(0, 0)[0].is_nan(),
        "the BLANK pixel carries no value, got {}",
        normalized.get_pixel(0, 0)[0]
    );
    assert_eq!(normalized.get_pixel(1, 0)[0], 0.5);

    Ok(())
}

#[test]
fn every_plane_of_a_four_axis_array_is_reachable() -> TestResult {
    // The first two axes are the image; every axis beyond them multiplies the
    // number of planes. Counting only NAXIS3 leaves most of this file
    // unreachable: NAXIS3 x NAXIS4 is six planes, not two.
    let data: Vec<u8> = (1..=24).collect();
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
        &data,
    );
    let path = write_temp_fits("four-axes.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let hdu = fits.primary_hdu();

    assert_eq!(hdu.image_count(), 6);

    // Each plane is the next four bytes of the file, in order.
    for plane in 0..6 {
        let image = hdu
            .read_image(plane)?
            .unwrap_or_else(|| panic!("plane {plane} is missing"));

        let expected: Vec<u8> = (1..=4).map(|byte| byte + 4 * plane as u8).collect();
        assert_eq!(raw_bytes(&image), expected, "plane {plane}");
    }

    assert!(hdu.read_image(6)?.is_none());

    Ok(())
}

#[test]
fn a_random_groups_hdu_reads_its_groups_rather_than_an_image() -> TestResult {
    // The random-groups convention puts GCOUNT groups in the primary HDU, each
    // one PCOUNT parameters followed by an array. NAXIS1 = 0 is the placeholder
    // that marks it.
    let mut data = Vec::new();
    for group in 0..3_i16 {
        // Two parameters, then a three-element array.
        for value in [group * 10, group * 10 + 1] {
            data.extend_from_slice(&value.to_be_bytes());
        }
        for value in [group * 100, group * 100 + 1, group * 100 + 2] {
            data.extend_from_slice(&value.to_be_bytes());
        }
    }

    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "16"),
            ("NAXIS", "2"),
            ("NAXIS1", "0"),
            ("NAXIS2", "3"),
            ("GROUPS", "T"),
            ("PCOUNT", "2"),
            ("GCOUNT", "3"),
            ("PTYPE1", "'TIME'"),
            ("PTYPE2", "'BASELINE'"),
        ],
        &data,
    );
    let path = write_temp_fits("groups.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let hdu = fits.primary_hdu();

    // A groups HDU holds no image; reporting one would hand back a zero-width
    // array that means nothing.
    assert_eq!(hdu.image_count(), 0);
    assert!(hdu.read_image(0)?.is_none());

    assert_eq!(hdu.group_count(), 3);

    let group = hdu.read_group(1)?.expect("a second group");
    assert_eq!(group.parameters(), &[10.0, 11.0]);
    assert_eq!(group.parameter("TIME"), Some(10.0));
    assert_eq!(group.parameter("BASELINE"), Some(11.0));
    assert_eq!(group.data(), &[100.0, 101.0, 102.0]);

    assert!(hdu.read_group(3)?.is_none());

    Ok(())
}

#[test]
fn group_parameters_carry_their_own_scaling() -> TestResult {
    // PSCALn and PZEROn scale a parameter independently of BSCALE and BZERO,
    // which scale the array.
    let mut data = Vec::new();
    data.extend_from_slice(&10_i16.to_be_bytes());
    data.extend_from_slice(&5_i16.to_be_bytes());

    let file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "16"),
            ("NAXIS", "2"),
            ("NAXIS1", "0"),
            ("NAXIS2", "1"),
            ("GROUPS", "T"),
            ("PCOUNT", "1"),
            ("GCOUNT", "1"),
            ("PTYPE1", "'TIME'"),
            ("PSCAL1", "0.5"),
            ("PZERO1", "100"),
            ("BSCALE", "2"),
            ("BZERO", "1"),
        ],
        &data,
    );
    let path = write_temp_fits("groups-scaled.fits", &file)?;

    let fits = FsFits::open(&path)?;
    let group = fits.primary_hdu().read_group(0)?.expect("one group");

    // 100 + 0.5 * 10
    assert_eq!(group.parameters(), &[105.0]);
    // 1 + 2 * 5
    assert_eq!(group.data(), &[11.0]);

    Ok(())
}

#[test]
fn an_ordinary_image_hdu_has_no_groups() -> TestResult {
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
    let path = write_temp_fits("no-groups.fits", &file)?;

    let fits = FsFits::open(&path)?;

    assert_eq!(fits.primary_hdu().group_count(), 0);
    assert!(fits.primary_hdu().read_group(0)?.is_none());

    Ok(())
}
