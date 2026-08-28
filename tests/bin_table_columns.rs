//! Column offsets within a binary table row. A column whose declared width is
//! wrong shifts every column after it, so these read a trailing sentinel column
//! to prove the offsets line up.

mod common;

use common::{append_extension, fits_file, write_temp_fits};
use fits_io::Fits;
use fits_io::bin_table::{BinTable, Value};
use fits_io::fs::FsFits;
use fits_io::hdu::{BinTableHDU, ExtensionHDU};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

/// Builds a one-row table from `(TTYPEn, TFORMn)` columns and the raw row bytes.
fn read_table(
    name: &str,
    columns: &[(&str, &str)],
    row: &[u8],
) -> Result<BinTable, Box<dyn Error + Send + Sync>> {
    let mut file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "0"),
            ("EXTEND", "T"),
        ],
        &[],
    );

    let row_len = row.len().to_string();
    let field_count = columns.len().to_string();

    let mut cards: Vec<(String, String)> = vec![
        ("XTENSION".into(), "'BINTABLE'".into()),
        ("BITPIX".into(), "8".into()),
        ("NAXIS".into(), "2".into()),
        ("NAXIS1".into(), row_len),
        ("NAXIS2".into(), "1".into()),
        ("PCOUNT".into(), "0".into()),
        ("GCOUNT".into(), "1".into()),
        ("TFIELDS".into(), field_count),
    ];
    for (index, (ttype, tform)) in columns.iter().enumerate() {
        cards.push((format!("TFORM{}", index + 1), format!("'{:<8}'", tform)));
        cards.push((format!("TTYPE{}", index + 1), format!("'{:<8}'", ttype)));
    }

    let borrowed: Vec<(&str, &str)> = cards
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    append_extension(&mut file, &borrowed, row);

    let path = write_temp_fits(name, &file)?;
    let fits = FsFits::open(&path)?;

    let Some(ExtensionHDU::BinTable(table)) = fits.extension_hdu(0) else {
        panic!("expected a binary table extension");
    };

    table.read_table()
}

#[test]
fn a_column_after_a_single_precision_complex_reads_at_the_right_offset() -> TestResult {
    // 1C occupies 8 bytes, not 4. Getting it wrong reads `after` from the middle
    // of the complex value.
    let mut row = Vec::new();
    row.extend_from_slice(&1.5_f32.to_be_bytes());
    row.extend_from_slice(&(-2.5_f32).to_be_bytes());
    row.extend_from_slice(&0x0BADF00D_u32.to_be_bytes());

    let table = read_table(
        "complex-offset.fits",
        &[("value", "1C"), ("after", "1J")],
        &row,
    )?;
    let row = table.row(0).expect("the table has one row");

    let Some(Value::C32(complex)) = row.get("value")? else {
        panic!("expected a complex column");
    };
    assert_eq!(complex, vec![(1.5, -2.5)]);

    let Some(Value::I32(after)) = row.get("after")? else {
        panic!("expected an i32 column");
    };
    assert_eq!(after, vec![0x0BADF00D]);

    Ok(())
}

#[test]
fn a_column_after_a_double_precision_complex_reads_at_the_right_offset() -> TestResult {
    // 1M occupies 16 bytes, not 8.
    let mut row = Vec::new();
    row.extend_from_slice(&1.5_f64.to_be_bytes());
    row.extend_from_slice(&(-2.5_f64).to_be_bytes());
    row.extend_from_slice(&0x0BADF00D_u32.to_be_bytes());

    let table = read_table(
        "complex64-offset.fits",
        &[("value", "1M"), ("after", "1J")],
        &row,
    )?;
    let row = table.row(0).expect("the table has one row");

    let Some(Value::M64(complex)) = row.get("value")? else {
        panic!("expected a complex column");
    };
    assert_eq!(complex, vec![(1.5, -2.5)]);

    let Some(Value::I32(after)) = row.get("after")? else {
        panic!("expected an i32 column");
    };
    assert_eq!(after, vec![0x0BADF00D]);

    Ok(())
}

#[test]
fn a_column_after_a_bit_column_reads_at_the_right_offset() -> TestResult {
    // 16X is 16 bits, which is 2 bytes, not 16.
    let mut row = Vec::new();
    row.extend_from_slice(&[0b1010_1010, 0b0101_0101]);
    row.extend_from_slice(&0x0BADF00D_u32.to_be_bytes());

    let table = read_table("bit-offset.fits", &[("bits", "16X"), ("after", "1J")], &row)?;
    let row = table.row(0).expect("the table has one row");

    let Some(Value::Bit(bits)) = row.get("bits")? else {
        panic!("expected a bit column");
    };
    assert_eq!(bits, vec![0b1010_1010, 0b0101_0101]);

    let Some(Value::I32(after)) = row.get("after")? else {
        panic!("expected an i32 column");
    };
    assert_eq!(after, vec![0x0BADF00D]);

    Ok(())
}

#[test]
fn a_column_after_a_string_array_reads_at_the_right_offset() -> TestResult {
    // 15A5 is 15 bytes holding three 5-character substrings, not 75 bytes.
    let mut row = Vec::new();
    row.extend_from_slice(b"alphabeta gamma");
    row.extend_from_slice(&0x0BADF00D_u32.to_be_bytes());

    let table = read_table(
        "stringarray-offset.fits",
        &[("names", "15A5"), ("after", "1J")],
        &row,
    )?;
    let row = table.row(0).expect("the table has one row");

    let Some(Value::StringArray(names)) = row.get("names")? else {
        panic!("expected a string array column");
    };
    assert_eq!(names, vec!["alpha", "beta", "gamma"]);

    let Some(Value::I32(after)) = row.get("after")? else {
        panic!("expected an i32 column");
    };
    assert_eq!(after, vec![0x0BADF00D]);

    Ok(())
}

#[test]
fn logical_columns_round_trip_true_and_false() -> TestResult {
    let mut row = Vec::new();
    row.extend_from_slice(b"TF");
    row.extend_from_slice(&0x0BADF00D_u32.to_be_bytes());

    let table = read_table("logical.fits", &[("flags", "2L"), ("after", "1J")], &row)?;
    let row = table.row(0).expect("the table has one row");

    let Some(Value::Boolean(flags)) = row.get("flags")? else {
        panic!("expected a logical column");
    };
    assert_eq!(flags, vec![true, false], "'F' must not read as true");

    Ok(())
}

#[test]
fn a_table_declaring_a_variable_length_column_is_rejected() -> TestResult {
    // The heap that holds the actual data is not read yet, so accepting the
    // descriptor would hand back meaningless offsets.
    let error = read_table(
        "varlen.fits",
        &[("data", "1PJ(4)"), ("after", "1J")],
        &[0; 12],
    )
    .expect_err("variable length array columns are not supported");

    assert!(
        error.to_string().contains("not supported yet"),
        "got: {error}"
    );

    Ok(())
}
