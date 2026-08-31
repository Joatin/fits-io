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

//
// Reference values
//
// Everything below checks this crate's world coordinates against wcslib, the
// implementation the WCS papers are written around, reached through astropy.
// Agreement here is agreement with the standard rather than with this crate's
// own reading of it.

/// Where wcslib, through astropy, puts the sixteen test pixels under TAN.
#[allow(clippy::excessive_precision)]
const TAN_REFERENCE: [(f64, f64); 16] = [
    (83.7723800896406, 21.86053143459768),
    (83.7428344700254, 21.86291971990563),
    (83.6198659060849, 21.87280231842533),
    (83.47419197914745, 21.88438985086372),
    (83.77497826654452, 21.887952004520894),
    (83.7454269835917, 21.890340784700893),
    (83.62243483198252, 21.900225364414958),
    (83.4767329272202, 21.911815080137455),
    (83.78580167446422, 22.002065875436806),
    (83.75622680084446, 22.004456717062656),
    (83.63313639452386, 22.014349549943585),
    (83.48731794752335, 22.025948361286495),
    (83.798644192122, 22.137228885002216),
    (83.76904133012354, 22.139622172472098),
    (83.64583435241616, 22.149524798071646),
    (83.49987763675011, 22.161134403000688),
];

/// Where wcslib, through astropy, puts the sixteen test pixels under SIP.
#[allow(clippy::excessive_precision)]
const SIP_REFERENCE: [(f64, f64); 16] = [
    (83.77226614551562, 21.8604396469687),
    (83.74274615294779, 21.862840049907213),
    (83.61981603509086, 21.872731624360842),
    (83.47401932350888, 21.88424512773013),
    (83.77487643656954, 21.887878437654333),
    (83.7453518021667, 21.890280458459657),
    (83.62240232909956, 21.900178692551624),
    (83.47658260881404, 21.91169993102411),
    (83.78571040452893, 22.002014875519762),
    (83.75616644558608, 22.00442362368816),
    (83.63313639440682, 22.014349549851087),
    (83.4872208861706, 22.02590295274636),
    (83.79848037164267, 22.137075289818867),
    (83.76891351257117, 22.139491994050662),
    (83.64578797413672, 22.149450670411316),
    (83.49975895477054, 22.16104211356746),
];

/// Where wcslib, through astropy, puts the sixteen test pixels under TPV.
#[allow(clippy::excessive_precision)]
const TPV_REFERENCE: [(f64, f64); 16] = [
    (83.77242816402705, 21.860522508767712),
    (83.7428718740314, 21.862905821724762),
    (83.61988050027719, 21.8727849308871),
    (83.47422454123222, 21.884404300656684),
    (83.77501996454299, 21.887951662637928),
    (83.74545825718207, 21.89033498469772),
    (83.62244432618292, 21.900214054045232),
    (83.47676162392771, 21.91183321462114),
    (83.78582854601649, 22.00208726595443),
    (83.75624427559185, 22.00447062985654),
    (83.63313639456985, 22.0143495499351),
    (83.48734230751772, 22.0259678592644),
    (83.79867797252604, 22.1372468254978),
    (83.76906693918438, 22.139630247339607),
    (83.64584762002917, 22.149509025374062),
    (83.49992141388425, 22.161126367371658),
];

/// Where wcslib, through astropy, puts the sixteen test pixels under AIT.
#[allow(clippy::excessive_precision)]
const AIT_REFERENCE: [(f64, f64); 16] = [
    (83.77238071025187, 21.860530649852535),
    (83.74283487718944, 21.862919097230165),
    (83.61986587578701, 21.872801990426524),
    (83.47419131201315, 21.884389195841745),
    (83.77497878351602, 21.887951459433374),
    (83.74542730924091, 21.890340373164232),
    (83.62243481612882, 21.900225192382024),
    (83.47673239581252, 21.911814648256417),
    (83.78580199823904, 22.002065839487905),
    (83.75622697066436, 22.004456698188225),
    (83.63313639452383, 22.01434954994358),
    (83.48731766706624, 22.025948391194948),
    (83.79864488658546, 22.1372295087422),
    (83.76904180031968, 22.139622679969953),
    (83.64583437936142, 22.14952508200517),
    (83.49987709701497, 22.16113508149551),
];

/// The header every reference case is built on: a thousand pixels square, with
/// a CD matrix that both scales and rotates.
fn reference_cards(ctype1: &str, ctype2: &str) -> Vec<(&'static str, String)> {
    vec![
        ("CTYPE1", format!("'{ctype1}'")),
        ("CTYPE2", format!("'{ctype2}'")),
        ("CRPIX1", "512.5".into()),
        ("CRPIX2", "512.5".into()),
        ("CRVAL1", "83.633".into()),
        ("CRVAL2", "22.0145".into()),
        ("CD1_1", "-0.000277".into()),
        ("CD1_2", "0.0000241".into()),
        ("CD2_1", "0.0000239".into()),
        ("CD2_2", "0.000277".into()),
    ]
}

/// The pixels the reference values were taken at, in FITS order.
const REFERENCE_PIXELS: [(f64, f64); 16] = [
    (1.0, 1.0),
    (100.0, 1.0),
    (512.0, 1.0),
    (1000.0, 1.0),
    (1.0, 100.0),
    (100.0, 100.0),
    (512.0, 100.0),
    (1000.0, 100.0),
    (1.0, 512.0),
    (100.0, 512.0),
    (512.0, 512.0),
    (1000.0, 512.0),
    (1.0, 1000.0),
    (100.0, 1000.0),
    (512.0, 1000.0),
    (1000.0, 1000.0),
];

/// Checks a header's world coordinates against wcslib's, and its inverse
/// against itself.
fn assert_matches_reference(
    name: &str,
    cards: &[(&str, String)],
    reference: &[(f64, f64); 16],
) -> TestResult {
    let borrowed: Vec<(&str, &str)> = cards
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect();

    let fits = open(&format!("wcs-reference-{name}.fits"), &borrowed)?;
    let wcs = Wcs::from_header(fits.primary_hdu().header())?;

    for (pixel, expected) in REFERENCE_PIXELS.iter().zip(reference) {
        let actual = wcs.pixel_to_world(*pixel);

        // A ten-thousandth of an arcsecond. Nothing on the sky is measured
        // anywhere near this well; a projection with a term wrong in it would
        // be out by seconds of arc at the corners of the frame, not by this.
        let tolerance = 1e-4 / 3600.0;

        assert!(
            (actual.0 - expected.0).abs() < tolerance && (actual.1 - expected.1).abs() < tolerance,
            "{name} at pixel {pixel:?}: wcslib says {expected:?}, this crate {actual:?}"
        );

        let back = wcs.world_to_pixel(actual);
        assert!(
            (back.0 - pixel.0).abs() < 1e-6 && (back.1 - pixel.1).abs() < 1e-6,
            "{name}: pixel {pixel:?} came back as {back:?}"
        );
    }

    Ok(())
}

#[test]
fn the_gnomonic_projection_matches_wcslib() -> TestResult {
    assert_matches_reference(
        "tan",
        &reference_cards("RA---TAN", "DEC--TAN"),
        &TAN_REFERENCE,
    )
}

#[test]
fn the_hammer_aitoff_projection_matches_wcslib() -> TestResult {
    // A whole-sky projection, whose sphere is turned a quarter turn from the
    // reference point rather than centred on it.
    assert_matches_reference(
        "ait",
        &reference_cards("RA---AIT", "DEC--AIT"),
        &AIT_REFERENCE,
    )
}

#[test]
fn a_sip_distortion_matches_wcslib() -> TestResult {
    let mut cards = reference_cards("RA---TAN-SIP", "DEC--TAN-SIP");
    cards.extend([
        ("A_ORDER", "3".to_string()),
        ("A_2_0", "1.2E-6".to_string()),
        ("A_1_1", "-3.1E-7".to_string()),
        ("A_0_2", "5.5E-7".to_string()),
        ("A_3_0", "2.0E-10".to_string()),
        ("B_ORDER", "3".to_string()),
        ("B_2_0", "-8.0E-7".to_string()),
        ("B_1_1", "4.4E-7".to_string()),
        ("B_0_2", "-1.1E-6".to_string()),
        ("B_0_3", "-1.5E-10".to_string()),
    ]);

    assert_matches_reference("sip", &cards, &SIP_REFERENCE)
}

#[test]
fn a_tpv_distortion_matches_wcslib() -> TestResult {
    let mut cards = reference_cards("RA---TPV", "DEC--TPV");
    cards.extend([
        ("PV1_0", "0.0".to_string()),
        ("PV1_1", "1.0".to_string()),
        ("PV1_2", "0.0".to_string()),
        ("PV1_4", "0.0012".to_string()),
        ("PV1_5", "-0.0004".to_string()),
        ("PV1_6", "0.0007".to_string()),
        ("PV2_0", "0.0".to_string()),
        ("PV2_1", "1.0".to_string()),
        ("PV2_2", "0.0".to_string()),
        ("PV2_4", "-0.0009".to_string()),
        ("PV2_5", "0.0003".to_string()),
        ("PV2_6", "0.0011".to_string()),
    ]);

    assert_matches_reference("tpv", &cards, &TPV_REFERENCE)
}

#[test]
fn an_undistorted_reading_of_a_distorted_header_is_visibly_different() -> TestResult {
    // The point of the two tests above: dropping the distortion leaves
    // coordinates that are right in the middle of the frame and wrong at its
    // corners, by far more than any measurement tolerance.
    let build =
        |name: &str, cards: &[(&str, String)]| -> Result<Wcs, Box<dyn Error + Send + Sync>> {
            let borrowed: Vec<(&str, &str)> = cards
                .iter()
                .map(|(key, value)| (*key, value.as_str()))
                .collect();

            Wcs::from_header(open(name, &borrowed)?.primary_hdu().header())
        };

    let mut cards = reference_cards("RA---TAN-SIP", "DEC--TAN-SIP");
    cards.extend([
        ("A_ORDER", "3".to_string()),
        ("A_2_0", "1.2E-6".to_string()),
        ("B_ORDER", "3".to_string()),
        ("B_0_2", "-1.1E-6".to_string()),
    ]);

    let distorted = build("wcs-distorted.fits", &cards)?;
    let ideal = build("wcs-ideal.fits", &reference_cards("RA---TAN", "DEC--TAN"))?;

    let corner = (1.0, 1.0);
    let with = distorted.pixel_to_world(corner);
    let without = ideal.pixel_to_world(corner);

    let arcseconds = ((with.0 - without.0).abs() + (with.1 - without.1).abs()) * 3600.0;
    assert!(
        arcseconds > 0.1,
        "the correction should move the corner by more than a tenth of an arcsecond, got \
         {arcseconds}"
    );

    Ok(())
}
