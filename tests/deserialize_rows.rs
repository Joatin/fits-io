//! Deserializing binary table rows into user structs. Every field type here
//! used to hit a `todo!()` except `f32`, `f64` and `i64`.

mod common;

use common::{append_extension, fits_file, fixture, write_temp_fits};
use fits_io::Fits;
use fits_io::fs::FsFits;
use fits_io::hdu::{BinTableHDU, ExtensionHDU};
use serde::Deserialize;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

/// One row: a name, a flag, and one column of every numeric width.
fn row_bytes() -> Vec<u8> {
    let mut row = Vec::new();
    row.extend_from_slice(b"Vega      "); // 10A
    row.push(b'T'); // 1L
    row.push(200); // 1B  (u8)
    row.extend_from_slice(&(-1234_i16).to_be_bytes()); // 1I
    row.extend_from_slice(&70_000_i32.to_be_bytes()); // 1J
    row.extend_from_slice(&5_000_000_000_i64.to_be_bytes()); // 1K
    row.extend_from_slice(&1.5_f32.to_be_bytes()); // 1E
    row.extend_from_slice(&2.25_f64.to_be_bytes()); // 1D
    row
}

const ROW_LEN: usize = 10 + 1 + 1 + 2 + 4 + 8 + 4 + 8;

fn table_file(rows: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for _ in 0..rows {
        data.extend_from_slice(&row_bytes());
    }

    let mut file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "0"),
            ("EXTEND", "T"),
        ],
        &[],
    );
    let row_len = ROW_LEN.to_string();
    let row_count = rows.to_string();

    append_extension(
        &mut file,
        &[
            ("XTENSION", "'BINTABLE'"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", &row_len),
            ("NAXIS2", &row_count),
            ("PCOUNT", "0"),
            ("GCOUNT", "1"),
            ("TFIELDS", "8"),
            ("TFORM1", "'10A     '"),
            ("TTYPE1", "'name    '"),
            ("TFORM2", "'1L      '"),
            ("TTYPE2", "'flag    '"),
            ("TFORM3", "'1B      '"),
            ("TTYPE3", "'tiny    '"),
            ("TFORM4", "'1I      '"),
            ("TTYPE4", "'small   '"),
            ("TFORM5", "'1J      '"),
            ("TTYPE5", "'medium  '"),
            ("TFORM6", "'1K      '"),
            ("TTYPE6", "'large   '"),
            ("TFORM7", "'1E      '"),
            ("TTYPE7", "'single  '"),
            ("TFORM8", "'1D      '"),
            ("TTYPE8", "'double  '"),
        ],
        &data,
    );

    file
}

fn rows<T: serde::de::DeserializeOwned + Send + Sync>(
    name: &str,
    count: usize,
) -> Result<Vec<T>, Box<dyn Error + Send + Sync>> {
    let path = write_temp_fits(name, &table_file(count))?;
    let fits = FsFits::open(&path)?;

    let Some(ExtensionHDU::BinTable(table)) = fits.extension_hdu(0) else {
        panic!("expected a binary table extension");
    };

    table.read_rows()
}

#[derive(Debug, Deserialize, PartialEq)]
struct EveryType {
    name: String,
    flag: bool,
    tiny: u8,
    small: i16,
    medium: i32,
    large: i64,
    single: f32,
    double: f64,
}

#[test]
fn deserializes_every_scalar_column_type() -> TestResult {
    let rows: Vec<EveryType> = rows("every-type.fits", 3)?;

    assert_eq!(rows.len(), 3);
    for row in &rows {
        assert_eq!(
            row,
            &EveryType {
                name: "Vega".to_string(),
                flag: true,
                tiny: 200,
                small: -1234,
                medium: 70_000,
                large: 5_000_000_000,
                single: 1.5,
                double: 2.25,
            }
        );
    }

    Ok(())
}

#[derive(Debug, Deserialize, PartialEq)]
struct JustTwo {
    name: String,
    double: f64,
}

#[test]
fn a_struct_may_name_a_subset_of_the_columns() -> TestResult {
    // The columns in between are skipped through deserialize_ignored_any, which
    // used to be a todo!().
    let rows: Vec<JustTwo> = rows("subset.fits", 1)?;

    assert_eq!(
        rows,
        vec![JustTwo {
            name: "Vega".to_string(),
            double: 2.25,
        }]
    );

    Ok(())
}

#[derive(Debug, Deserialize, PartialEq)]
struct Optional {
    name: Option<String>,
    medium: Option<i32>,
}

#[test]
fn option_fields_read_the_column_value() -> TestResult {
    let rows: Vec<Optional> = rows("optional.fits", 1)?;

    assert_eq!(
        rows,
        vec![Optional {
            name: Some("Vega".to_string()),
            medium: Some(70_000),
        }]
    );

    Ok(())
}

#[derive(Debug, Deserialize)]
struct Widened {
    tiny: i64,
    small: f64,
}

#[test]
fn narrow_columns_widen_into_larger_fields() -> TestResult {
    let rows: Vec<Widened> = rows("widened.fits", 1)?;

    assert_eq!(rows[0].tiny, 200);
    assert_eq!(rows[0].small, -1234.0);

    Ok(())
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Only used to prove that deserializing it fails.
struct TooNarrow {
    medium: u8,
}

#[test]
fn a_value_that_does_not_fit_the_field_is_an_error() -> TestResult {
    // 70000 does not fit in a u8. This must report, not wrap or panic.
    let Err(error) = rows::<TooNarrow>("too-narrow.fits", 1) else {
        panic!("70000 must not deserialize into a u8");
    };

    assert!(error.to_string().contains("does not fit"), "got: {error}");

    Ok(())
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Only used to prove that deserializing it fails.
struct WrongType {
    name: i32,
}

#[test]
fn a_mismatched_column_type_is_an_error() -> TestResult {
    let Err(error) = rows::<WrongType>("wrong-type.fits", 1) else {
        panic!("a string column must not deserialize into an i32");
    };

    assert!(error.to_string().contains("invalid type"), "got: {error}");

    Ok(())
}

#[test]
fn the_real_gaia_catalogue_still_deserializes() -> TestResult {
    #[derive(Debug, Deserialize)]
    struct GaiaCatalogEntry {
        pub source_id: i64,
        pub ra: f64,
        pub dec: f64,
        pub phot_g_mean_mag: f32,
    }

    let Some(path) = fixture("gaia-dr3-mag-gt-12.fits") else {
        return Ok(());
    };

    let fits = FsFits::open(&path)?;

    let Some(ExtensionHDU::BinTable(table)) = fits.extension_hdu(0) else {
        panic!("expected a binary table extension");
    };

    let rows: Vec<GaiaCatalogEntry> = table.read_rows()?;
    assert_eq!(rows.len(), 2_482_633);

    let first = &rows[0];
    assert!(first.source_id != 0);
    assert!((-90.0..=90.0).contains(&first.dec), "dec {}", first.dec);
    assert!((0.0..=360.0).contains(&first.ra), "ra {}", first.ra);
    assert!(first.phot_g_mean_mag > 0.0);

    Ok(())
}
