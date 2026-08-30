//! The two conventions for headers that do not fit the fixed 80-column card: a
//! value continued across several cards, and a keyword too long for eight
//! columns.
//!
//! Both were previously dropped on the floor, which lost the data on reading and
//! left blank cards in its place on writing.

mod common;

use common::{BLOCK, card, write_temp_fits};
use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::{HDU, ImageHDU};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

/// A header-only file whose cards are `extra`, verbatim.
fn file(extra: &[String]) -> Vec<u8> {
    let mut header = String::new();
    for (keyword, value) in [("SIMPLE", "T"), ("BITPIX", "8"), ("NAXIS", "0")] {
        header.push_str(&card(keyword, value));
    }
    for line in extra {
        header.push_str(&format!("{:<80}", line));
    }
    header.push_str(&format!("{:<80}", "END"));

    let mut bytes = header.into_bytes();
    let padding = (BLOCK - bytes.len() % BLOCK) % BLOCK;
    bytes.resize(bytes.len() + padding, b' ');

    bytes
}

fn open(name: &str, cards: &[String]) -> Result<FsFits, Box<dyn Error + Send + Sync>> {
    let path = write_temp_fits(name, &file(cards))?;
    FsFits::open(&path)
}

/// The cards of a written header, as text.
fn written(fits: &FsFits) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    Ok(fits.to_vec()?[..BLOCK]
        .chunks(80)
        .map(|card| String::from_utf8_lossy(card).trim_end().to_string())
        .collect())
}

#[test]
fn a_value_continued_across_cards_is_joined() -> TestResult {
    // The `&` at the end of the value says the rest is on the next card. Read
    // without joining, the value stops there and keeps the marker.
    let fits = open(
        "continue-read.fits",
        &[
            "OBJECT  = 'a name too long for one card and therefore split in two&'".to_string(),
            "CONTINUE  ' and this is the rest of it'".to_string(),
        ],
    )?;

    assert_eq!(
        fits.primary_hdu().header().object(),
        Some("a name too long for one card and therefore split in two and this is the rest of it")
    );

    Ok(())
}

#[test]
fn a_value_continued_across_three_cards_is_joined() -> TestResult {
    let fits = open(
        "continue-three.fits",
        &[
            "OBJECT  = 'one&'".to_string(),
            "CONTINUE  'two&'".to_string(),
            "CONTINUE  'three'".to_string(),
        ],
    )?;

    assert_eq!(fits.primary_hdu().header().object(), Some("onetwothree"));

    Ok(())
}

#[test]
fn a_comment_on_the_last_continuation_belongs_to_the_whole_value() -> TestResult {
    let fits = open(
        "continue-comment.fits",
        &[
            "OBJECT  = 'first&'".to_string(),
            "CONTINUE  'second' / what was observed".to_string(),
        ],
    )?;

    let header = fits.primary_hdu().header();
    assert_eq!(header.object(), Some("firstsecond"));

    // And it survives being written back out.
    let cards = written(&fits)?;
    assert!(
        cards.iter().any(|card| card.contains("what was observed")),
        "the comment was lost: {cards:?}"
    );

    Ok(())
}

#[test]
fn an_ampersand_without_a_continuation_is_left_alone() -> TestResult {
    // A value may legitimately end in `&` with nothing following it.
    let fits = open(
        "continue-none.fits",
        &["OBJECT  = 'ends with an ampersand &'".to_string()],
    )?;

    assert_eq!(
        fits.primary_hdu().header().object(),
        Some("ends with an ampersand &")
    );

    Ok(())
}

#[test]
fn a_long_value_survives_being_written_and_read_again() -> TestResult {
    // The round trip is what the convention is for: a value longer than a card
    // has to be split again on the way out and come back whole.
    let fits = open(
        "continue-roundtrip.fits",
        &[
            "OBJECT  = 'M31 observed through a filter whose name is far too long to fi&'"
                .to_string(),
            "CONTINUE  't inside a single eighty column FITS card, several times over,&'"
                .to_string(),
            "CONTINUE  ' as happens with real instrument configurations'".to_string(),
        ],
    )?;

    let before = fits
        .primary_hdu()
        .header()
        .object()
        .expect("an OBJECT card")
        .to_string();
    assert!(before.len() > 80, "the value should not fit one card");

    let path = write_temp_fits("continue-written.fits", &fits.to_vec()?)?;
    let reopened = FsFits::open(&path)?;

    assert_eq!(
        reopened.primary_hdu().header().object(),
        Some(before.as_str())
    );

    Ok(())
}

#[test]
fn a_long_value_is_written_as_a_first_card_and_continuations() -> TestResult {
    let fits = open(
        "continue-form.fits",
        &[
            "OBJECT  = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa&'"
                .to_string(),
            "CONTINUE  'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb&'"
                .to_string(),
            "CONTINUE  'ccccc'".to_string(),
        ],
    )?;

    let cards = written(&fits)?;

    // The first card must say that more follows, or a reader stops there.
    let object = cards
        .iter()
        .find(|card| card.starts_with("OBJECT"))
        .expect("an OBJECT card");
    assert!(object.ends_with("&'"), "got {object:?}");

    assert!(
        cards.iter().filter(|c| c.starts_with("CONTINUE")).count() >= 2,
        "expected continuations, got {cards:?}"
    );

    Ok(())
}

#[test]
fn a_hierarch_keyword_is_read() -> TestResult {
    // HIERARCH carries keywords that eight columns cannot hold, which is how
    // most instruments record their settings.
    let fits = open(
        "hierarch-read.fits",
        &["HIERARCH ESO DET CHIP NAME = 'CCD-44' / the detector".to_string()],
    )?;

    let header = fits.primary_hdu().header();
    let values = header.raw_card("ESO DET CHIP NAME");

    assert_eq!(values.len(), 1, "the keyword was lost");
    assert_eq!(values[0].value_to_string(), "CCD-44");
    assert_eq!(values[0].comment_to_string(), "the detector");

    Ok(())
}

#[test]
fn a_hierarch_keyword_survives_being_written_and_read_again() -> TestResult {
    let fits = open(
        "hierarch-write.fits",
        &[
            "HIERARCH ESO DET CHIP NAME = 'CCD-44'".to_string(),
            "HIERARCH ESO INS FILT1 NO = 42".to_string(),
        ],
    )?;

    let path = write_temp_fits("hierarch-again.fits", &fits.to_vec()?)?;
    let reopened = FsFits::open(&path)?;
    let header = reopened.primary_hdu().header();

    assert_eq!(
        header.raw_card("ESO DET CHIP NAME")[0].value_to_string(),
        "CCD-44"
    );
    assert_eq!(
        header.raw_card("ESO INS FILT1 NO")[0].value_to_string(),
        "42"
    );

    Ok(())
}

#[test]
fn a_header_with_neither_convention_is_unchanged() -> TestResult {
    // The joining must not disturb an ordinary header.
    let fits = open(
        "plain.fits",
        &[
            "OBJECT  = 'M31'".to_string(),
            "TELESCOP= 'JWST'".to_string(),
        ],
    )?;

    let header = fits.primary_hdu().header();
    assert_eq!(header.object(), Some("M31"));
    assert_eq!(header.telescope(), Some("JWST"));

    Ok(())
}

#[test]
fn joining_continuations_does_not_move_the_data_section() -> TestResult {
    // Joining takes several cards down to one, so a header's card count no
    // longer says how long it was in the file. If the data section were located
    // from the joined count, this header — which spans two blocks before joining
    // and would fit in one after — would be read from the wrong offset.
    let mut cards = Vec::new();
    for index in 0..16 {
        cards.push(format!("{:<8}= 'first&'", format!("MYKEY{index:02}")));
        cards.push(format!("CONTINUE  'second{index:02}'"));
    }

    let mut header = String::new();
    for (keyword, value) in [
        ("SIMPLE", "T"),
        ("BITPIX", "8"),
        ("NAXIS", "2"),
        ("NAXIS1", "2"),
        ("NAXIS2", "2"),
    ] {
        header.push_str(&card(keyword, value));
    }
    for line in &cards {
        header.push_str(&format!("{line:<80}"));
    }
    header.push_str(&format!("{:<80}", "END"));

    // 5 + 32 + 1 cards is more than the 36 a block holds, so this is two blocks.
    let mut bytes = header.into_bytes();
    assert!(bytes.len() > BLOCK, "the header should span two blocks");
    let padding = (BLOCK - bytes.len() % BLOCK) % BLOCK;
    bytes.resize(bytes.len() + padding, b' ');
    bytes.extend_from_slice(&[11, 22, 33, 44]);
    bytes.resize(bytes.len() + BLOCK - 4, 0);

    let path = write_temp_fits("continue-blocks.fits", &bytes)?;
    let fits = FsFits::open(&path)?;

    // The values joined...
    assert_eq!(
        fits.primary_hdu().header().raw_card("MYKEY00")[0].value_to_string(),
        "firstsecond00"
    );

    // ...and the pixels are still where the file put them.
    let image = fits
        .primary_hdu()
        .read_image(0)?
        .expect("a two axis HDU holds one image");
    match &image {
        fits_io::image::Image::U8(data) => assert_eq!(data.raw(), &[11, 22, 33, 44]),
        other => panic!("expected an 8-bit image, got {other:?}"),
    }

    Ok(())
}
