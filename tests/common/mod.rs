//! Builders for small, standard-conforming FITS files, so tests can cover header
//! and pixel handling without depending on the large fixtures in `tests/`.

// Every test binary compiles this module and uses only the part it needs.
#![allow(dead_code)]

use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

pub const BLOCK: usize = 2880;
pub const CARD: usize = 80;

/// Formats one fixed-format header card: keyword in columns 1-8, `= ` in 9-10,
/// value right-justified in 11-30.
pub fn card(keyword: &str, value: &str) -> String {
    // The value is right-justified against column 30 where it fits there, and
    // simply runs on from column 11 where it does not — a long string value,
    // say. Either way the card is padded out to its eighty columns.
    let card = format!("{:<8}= {:>20}", keyword, value);
    let card = format!("{:<width$}", card, width = CARD);

    debug_assert_eq!(card.len(), CARD, "card {:?} does not fit", keyword);
    card
}

/// Assembles a single-HDU FITS file from `cards` plus a raw data section, padding
/// both to the 2880-byte block size.
pub fn fits_file(cards: &[(&str, &str)], data: &[u8]) -> Vec<u8> {
    let mut header = String::new();
    for (keyword, value) in cards {
        header.push_str(&card(keyword, value));
    }
    header.push_str(&format!("{:<80}", "END"));

    let mut bytes = header.into_bytes();
    pad_to_block(&mut bytes, b' ');

    bytes.extend_from_slice(data);
    pad_to_block(&mut bytes, 0);

    bytes
}

/// Appends a second HDU: an extension header plus its data section.
pub fn append_extension(file: &mut Vec<u8>, cards: &[(&str, &str)], data: &[u8]) {
    file.extend_from_slice(&fits_file(cards, data));
}

fn pad_to_block(bytes: &mut Vec<u8>, filler: u8) {
    let padding = (BLOCK - bytes.len() % BLOCK) % BLOCK;
    bytes.resize(bytes.len() + padding, filler);
}

/// Locates one of the large Git LFS fixtures under `tests/`.
///
/// Returns `None` when the fixture is absent, which is the normal state in a
/// packaged crate: the fixtures are excluded from the crates.io tarball because
/// of their size. Tests that need real-world data should skip in that case
/// rather than fail.
pub fn fixture(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name);

    // A Git LFS pointer is a few hundred bytes of text standing in for the real
    // file, which is what a checkout without `git lfs pull` leaves behind.
    match fs::metadata(&path) {
        Ok(metadata) if metadata.len() > 4096 => Some(path),
        _ => {
            eprintln!("skipping: fixture {name} is not present (run `git lfs pull`)");
            None
        }
    }
}

/// Writes `contents` to a uniquely named file under the system temp directory.
///
/// The caller owns the returned path; tests that want cleanup should remove the
/// parent directory themselves.
pub fn write_temp_fits(
    name: &str,
    contents: &[u8],
) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    static COUNTER: AtomicU32 = AtomicU32::new(0);

    let directory = std::env::temp_dir().join(format!(
        "fits-io-test-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory)?;

    let path = directory.join(name);
    fs::write(&path, contents)?;

    Ok(path)
}
