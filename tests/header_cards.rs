//! Setting header cards by keyword: what is set must read back, survive a write
//! and be readable again by the typed accessors.

mod common;

use common::{fits_file, write_temp_fits};
use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::HDU;
use fits_io::header::{Header, Value};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

fn minimal_image() -> Vec<u8> {
    fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "2"),
            ("NAXIS2", "2"),
        ],
        &[1, 2, 3, 4],
    )
}

/// Writes a file whose primary header `edit` has had its way with, and reads it
/// back, so that what a test asserts is what a reader would see.
fn round_trip(
    name: &str,
    edit: impl FnOnce(&mut Header) -> TestResult,
) -> Result<FsFits, Box<dyn Error + Send + Sync>> {
    let path = write_temp_fits(name, &minimal_image())?;
    let mut fits = FsFits::open(&path)?;

    edit(fits.primary_hdu_mut().header_mut())?;

    let written = write_temp_fits(&format!("out-{name}"), &fits.to_vec()?)?;
    FsFits::open(&written)
}

#[test]
fn a_card_set_by_keyword_reads_back() -> TestResult {
    let mut header = Header::default();

    header.set_card("OBJECT", "M31")?;
    header.set_card("GAIN", 120_i64)?;
    header.set_card("PIXSCALE", 1.25_f64)?;
    header.set_card("MOONLIT", true)?;

    assert_eq!(
        header
            .card("OBJECT")
            .map(|v| v.value_to_string())
            .as_deref(),
        Some("M31")
    );
    assert_eq!(
        header.card("GAIN").map(|v| v.value_to_string()).as_deref(),
        Some("120")
    );
    assert_eq!(
        header
            .card("PIXSCALE")
            .map(|v| v.value_to_string())
            .as_deref(),
        Some("1.25")
    );
    assert_eq!(
        header
            .card("MOONLIT")
            .map(|v| v.value_to_string())
            .as_deref(),
        Some("T")
    );

    Ok(())
}

#[test]
fn a_keyword_the_crate_knows_reaches_its_typed_accessor() -> TestResult {
    let mut header = Header::default();

    header.set_card("OBJECT", "NGC 7000")?;
    header.set_card("TELESCOP", "RASA 8")?;
    header.set_card("BSCALE", 2.5)?;
    header.set_card("EXTEND", true)?;

    assert_eq!(header.object(), Some("NGC 7000"));
    assert_eq!(header.telescope(), Some("RASA 8"));
    assert_eq!(header.bscale(), Some(2.5));
    assert_eq!(header.extend(), Some(true));

    Ok(())
}

#[test]
fn setting_a_keyword_twice_replaces_it_rather_than_repeating_it() -> TestResult {
    let mut header = Header::default();

    header.set_card("FILTER", "Ha")?;
    header.set_card("FILTER", "OIII")?;

    assert_eq!(header.raw_card("FILTER").len(), 1);
    assert_eq!(
        header
            .card("FILTER")
            .map(|v| v.value_to_string())
            .as_deref(),
        Some("OIII")
    );

    Ok(())
}

#[test]
fn a_lower_case_keyword_is_written_upper_case() -> TestResult {
    let mut header = Header::default();

    header.set_card("object", "M42")?;

    assert!(header.contains_card("OBJECT"));
    assert_eq!(header.object(), Some("M42"));

    Ok(())
}

#[test]
fn a_comment_rides_along_with_the_value() -> TestResult {
    let mut header = Header::default();

    header.set_card("EXPTIME", Value::from(300.0).with_comment("seconds"))?;

    assert_eq!(
        header
            .card("EXPTIME")
            .map(|v| v.comment_to_string())
            .as_deref(),
        Some("seconds")
    );

    Ok(())
}

#[test]
fn a_card_that_was_set_survives_being_written_and_read_again() -> TestResult {
    let fits = round_trip("header-set.fits", |header| {
        header.set_card("OBJECT", "Barnard 33")?;
        header.set_card("FILTER", Value::from("Ha").with_comment("3nm"))?;
        header.set_card("AIRMASS", 1.04)?;
        Ok(())
    })?;

    let header = fits.primary_hdu().header();

    assert_eq!(header.object(), Some("Barnard 33"));
    assert_eq!(
        header
            .card("FILTER")
            .map(|v| v.value_to_string())
            .as_deref(),
        Some("Ha")
    );
    assert_eq!(
        header
            .card("FILTER")
            .map(|v| v.comment_to_string())
            .as_deref(),
        Some("3nm")
    );
    assert_eq!(
        header
            .card("AIRMASS")
            .map(|v| v.value_to_string())
            .as_deref(),
        Some("1.04")
    );

    Ok(())
}

#[test]
fn a_long_keyword_becomes_a_hierarch_card_and_survives_a_round_trip() -> TestResult {
    let fits = round_trip("header-hierarch.fits", |header| {
        header.set_card("ESO INS FILT1 NAME", "Halpha")?;
        Ok(())
    })?;

    let header = fits.primary_hdu().header();

    assert_eq!(
        header
            .card("ESO INS FILT1 NAME")
            .map(|v| v.value_to_string())
            .as_deref(),
        Some("Halpha")
    );

    Ok(())
}

#[test]
fn a_long_string_is_written_across_continue_cards() -> TestResult {
    // Trailing blanks in a FITS string are not significant, so the value ends
    // on a character that survives the round trip.
    let long = "a very long value ".repeat(8) + "end";

    let fits = round_trip("header-continue.fits", |header| {
        header.set_card("LONGVAL", long.as_str())?;
        Ok(())
    })?;

    assert_eq!(
        fits.primary_hdu()
            .header()
            .card("LONGVAL")
            .map(|v| v.value_to_string())
            .as_deref(),
        Some(long.as_str())
    );

    Ok(())
}

#[test]
fn comment_and_history_cards_pile_up_rather_than_replacing_each_other() -> TestResult {
    let fits = round_trip("header-comments.fits", |header| {
        header.add_comment("stacked from 42 subs");
        header.add_comment("darks and flats applied");
        header.add_history("calibrated by fits-io");
        Ok(())
    })?;

    let header = fits.primary_hdu().header();

    let comments: Vec<&str> = header.comments().collect();
    assert!(
        comments.contains(&"stacked from 42 subs") && comments.contains(&"darks and flats applied"),
        "got {comments:?}"
    );
    assert_eq!(
        header.history().collect::<Vec<_>>(),
        vec!["calibrated by fits-io"]
    );

    Ok(())
}

#[test]
fn a_comment_longer_than_a_card_is_split_across_several() {
    let mut header = Header::default();
    let text = "word ".repeat(40);

    header.add_comment(text.trim());

    let lines: Vec<&str> = header.comments().collect();
    assert!(
        lines.len() > 1,
        "expected more than one card, got {lines:?}"
    );
    assert!(lines.iter().all(|line| line.len() <= 72), "got {lines:?}");
    assert_eq!(lines.join(" "), text.trim());
}

#[test]
fn removing_a_card_takes_every_card_with_that_keyword() -> TestResult {
    let mut header = Header::default();

    header.set_card("FILTER", "Ha")?;
    header.add_comment("one");
    header.add_comment("two");

    assert_eq!(header.remove_card("FILTER"), 1);
    assert_eq!(header.remove_card("COMMENT"), 2);
    assert!(!header.contains_card("FILTER"));
    assert_eq!(header.comments().count(), 0);

    Ok(())
}

#[test]
fn the_keywords_that_have_their_own_setters_are_refused() {
    let mut header = Header::default();

    assert!(header.set_card("COMMENT", "text").is_err());
    assert!(header.set_card("HISTORY", "text").is_err());
    assert!(header.set_card("END", "text").is_err());
    assert!(header.set_card("NAXIS2", 4_i64).is_err());
    assert!(header.set_card("", "text").is_err());
}

#[test]
fn a_number_too_long_for_a_card_is_refused_rather_than_truncated() {
    let mut header = Header::default();

    let error = header
        .set_card("HUGE", 1e300)
        .expect_err("a 300-digit number does not fit on an 80-byte card");

    assert!(error.to_string().contains("card holds"), "got: {error}");
}

#[test]
fn a_keyword_with_characters_a_card_cannot_carry_is_refused() {
    let mut header = Header::default();

    assert!(header.set_card("BAD\u{00e9}KEY", 1_i64).is_err());
    assert!(header.set_card("A LONG KEY WITH = IN IT", 1_i64).is_err());
}

#[test]
fn card_keys_lists_what_the_header_holds() -> TestResult {
    let mut header = Header::default();

    header.set_card("OBJECT", "M13")?;
    header.set_card("FILTER", "L")?;

    let keys: Vec<String> = header.card_keys().collect();
    assert_eq!(keys, vec!["OBJECT".to_string(), "FILTER".to_string()]);

    Ok(())
}
