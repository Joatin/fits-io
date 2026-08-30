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

#[test]
fn a_cd_matrix_header_is_not_read_as_one_degree_per_pixel() -> TestResult {
    // Modern pipelines write CDi_j rather than CDELTn: the matrix carries the
    // scale and the rotation together. A reader that knows only CDELTn finds
    // none, falls back to the standard's default of 1, and puts every pixel a
    // whole degree from where it belongs -- roughly 3600 times too far.
    let fits = open(
        "wcs-cd.fits",
        &[
            ("CTYPE1", "'RA---TAN'"),
            ("CTYPE2", "'DEC--TAN'"),
            ("CRPIX1", "2.5"),
            ("CRPIX2", "2.5"),
            ("CRVAL1", "180.0"),
            ("CRVAL2", "0.0"),
            ("CD1_1", "-0.001"),
            ("CD1_2", "0.0"),
            ("CD2_1", "0.0"),
            ("CD2_2", "0.001"),
        ],
    )?;

    let wcs = Wcs::from_header(fits.primary_hdu().header())?;

    // At the equator one pixel right is 0.001 degrees of right ascension, and
    // CD1_1 is negative, so the coordinate decreases.
    let (ra, dec) = wcs.pixel_to_world((3.5, 2.5));
    assert!(
        (ra - 179.999).abs() < 1e-6,
        "one pixel should move 0.001 degrees, not {} of them",
        (ra - 180.0).abs()
    );
    assert!(dec.abs() < 1e-6, "got {dec}");

    Ok(())
}

#[test]
fn a_cd_matrix_takes_precedence_over_cdelt() -> TestResult {
    // The standard says a header carrying CDi_j means it; CDELTn and CROTAn are
    // then ignored rather than combined with it.
    let fits = open(
        "wcs-cd-wins.fits",
        &[
            ("CTYPE1", "'RA---TAN'"),
            ("CTYPE2", "'DEC--TAN'"),
            ("CRPIX1", "2.5"),
            ("CRPIX2", "2.5"),
            ("CRVAL1", "180.0"),
            ("CRVAL2", "0.0"),
            ("CD1_1", "-0.001"),
            ("CD2_2", "0.001"),
            // Nonsense that must not be used.
            ("CDELT1", "10.0"),
            ("CDELT2", "10.0"),
            ("CROTA2", "45.0"),
        ],
    )?;

    let wcs = Wcs::from_header(fits.primary_hdu().header())?;

    assert_eq!(wcs.transform(), [[-0.001, 0.0], [0.0, 0.001]]);

    Ok(())
}

#[test]
fn a_pc_matrix_is_scaled_by_cdelt() -> TestResult {
    // PCi_j is dimensionless: CDELTn supplies the scale. A quarter turn here
    // swaps the axes.
    let fits = open(
        "wcs-pc.fits",
        &[
            ("CTYPE1", "'RA---TAN'"),
            ("CTYPE2", "'DEC--TAN'"),
            ("CRPIX1", "2.5"),
            ("CRPIX2", "2.5"),
            ("CRVAL1", "180.0"),
            ("CRVAL2", "0.0"),
            ("CDELT1", "-0.001"),
            ("CDELT2", "0.001"),
            ("PC1_1", "0.0"),
            ("PC1_2", "1.0"),
            ("PC2_1", "1.0"),
            ("PC2_2", "0.0"),
        ],
    )?;

    let wcs = Wcs::from_header(fits.primary_hdu().header())?;

    assert_eq!(wcs.transform(), [[0.0, -0.001], [0.001, 0.0]]);

    Ok(())
}

#[test]
fn an_absent_pc_element_comes_from_the_identity_matrix() -> TestResult {
    // A header that writes only the elements it needs leaves the rest to the
    // identity, not to zero -- zeroes would make the matrix singular.
    let fits = open(
        "wcs-pc-partial.fits",
        &[
            ("CTYPE1", "'RA---TAN'"),
            ("CTYPE2", "'DEC--TAN'"),
            ("CRPIX1", "2.5"),
            ("CRPIX2", "2.5"),
            ("CRVAL1", "180.0"),
            ("CRVAL2", "0.0"),
            ("CDELT1", "-0.001"),
            ("CDELT2", "0.001"),
            ("PC1_2", "0.0"),
        ],
    )?;

    let wcs = Wcs::from_header(fits.primary_hdu().header())?;

    assert_eq!(wcs.transform(), [[-0.001, 0.0], [0.0, 0.001]]);

    Ok(())
}

#[test]
fn a_crota_rotation_pairs_each_scale_with_the_right_axis() -> TestResult {
    // The standard's relation is
    //   CD1_1 =  CDELT1 cos   CD1_2 = -CDELT2 sin
    //   CD2_1 =  CDELT1 sin   CD2_2 =  CDELT2 cos
    // The off-diagonal terms take the scale of the axis they draw from. Pairing
    // them the other way mirrors the rotation whenever the two scales differ in
    // sign, which for a sky image they almost always do.
    let fits = open(
        "wcs-crota.fits",
        &[
            ("CTYPE1", "'RA---TAN'"),
            ("CTYPE2", "'DEC--TAN'"),
            ("CRPIX1", "2.5"),
            ("CRPIX2", "2.5"),
            ("CRVAL1", "180.0"),
            ("CRVAL2", "0.0"),
            ("CDELT1", "-0.001"),
            ("CDELT2", "0.002"),
            ("CROTA2", "90.0"),
        ],
    )?;

    let wcs = Wcs::from_header(fits.primary_hdu().header())?;
    let transform = wcs.transform();

    // At 90 degrees the diagonal vanishes and the off-diagonal terms are
    // -CDELT2 and CDELT1.
    assert!(transform[0][0].abs() < 1e-12, "got {:?}", transform);
    assert!(
        (transform[0][1] - -0.002).abs() < 1e-12,
        "CD1_2 should carry CDELT2, got {:?}",
        transform
    );
    assert!(
        (transform[1][0] - -0.001).abs() < 1e-12,
        "CD2_1 should carry CDELT1, got {:?}",
        transform
    );
    assert!(transform[1][1].abs() < 1e-12, "got {:?}", transform);

    Ok(())
}

#[test]
fn every_convention_round_trips_between_pixel_and_sky() -> TestResult {
    let conventions: Vec<(&str, Vec<(&str, &str)>)> = vec![
        (
            "cd",
            vec![
                ("CD1_1", "-0.001"),
                ("CD1_2", "0.0003"),
                ("CD2_1", "0.0002"),
                ("CD2_2", "0.001"),
            ],
        ),
        (
            "pc",
            vec![
                ("CDELT1", "-0.001"),
                ("CDELT2", "0.001"),
                ("PC1_1", "0.98"),
                ("PC1_2", "-0.17"),
                ("PC2_1", "0.17"),
                ("PC2_2", "0.98"),
            ],
        ),
        (
            "crota",
            vec![
                ("CDELT1", "-0.001"),
                ("CDELT2", "0.001"),
                ("CROTA2", "23.5"),
            ],
        ),
    ];

    for (name, extra) in conventions {
        let mut cards = vec![
            ("CTYPE1", "'RA---TAN'"),
            ("CTYPE2", "'DEC--TAN'"),
            ("CRPIX1", "2.5"),
            ("CRPIX2", "2.5"),
            ("CRVAL1", "83.633"),
            ("CRVAL2", "22.0145"),
        ];
        cards.extend(extra.iter().copied());

        let fits = open(&format!("wcs-roundtrip-{name}.fits"), &cards)?;
        let wcs = Wcs::from_header(fits.primary_hdu().header())?;

        for pixel in [(1.0, 1.0), (2.5, 2.5), (4.0, 3.0)] {
            let world = wcs.pixel_to_world(pixel);
            let back = wcs.world_to_pixel(world);

            assert!(
                (back.0 - pixel.0).abs() < 1e-9 && (back.1 - pixel.1).abs() < 1e-9,
                "{name}: {pixel:?} came back as {back:?}"
            );
        }
    }

    Ok(())
}

#[test]
fn a_degenerate_matrix_is_rejected() -> TestResult {
    // Two axes that map onto the same line describe no coordinate system, and
    // inverting one would divide by zero.
    let fits = open(
        "wcs-singular.fits",
        &[
            ("CTYPE1", "'RA---TAN'"),
            ("CTYPE2", "'DEC--TAN'"),
            ("CRPIX1", "2.5"),
            ("CRPIX2", "2.5"),
            ("CRVAL1", "180.0"),
            ("CRVAL2", "0.0"),
            ("CD1_1", "0.001"),
            ("CD1_2", "0.002"),
            ("CD2_1", "0.002"),
            ("CD2_2", "0.004"),
        ],
    )?;

    let error = Wcs::from_header(fits.primary_hdu().header())
        .expect_err("a singular matrix describes nothing");

    assert!(error.to_string().contains("inverted"), "got: {error}");

    Ok(())
}
