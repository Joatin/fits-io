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

    let Some(Value::Bit { bytes, len }) = row.get("bits")? else {
        panic!("expected a bit column");
    };
    assert_eq!(bytes, vec![0b1010_1010, 0b0101_0101]);
    assert_eq!(len, 16);

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
fn a_variable_length_column_takes_only_its_descriptor_from_the_row() -> TestResult {
    // Only the 8-byte descriptor sits in the row; the values it points at live
    // in the heap. Getting that width wrong misaligns every following column,
    // so the check is that `after` still reads correctly.
    let mut data = Vec::new();
    data.extend_from_slice(&0_i32.to_be_bytes()); // empty array: count
    data.extend_from_slice(&0_i32.to_be_bytes()); // and offset
    data.extend_from_slice(&7_i32.to_be_bytes()); // after

    let table = read_table("varlen.fits", &[("data", "1PJ(4)"), ("after", "1J")], &data)?;

    let row = table.row(0).expect("the table has one row");

    assert!(
        matches!(row.get("after")?, Some(Value::I32(ref v)) if v == &[7]),
        "the column after a variable length array must still line up"
    );

    Ok(())
}

#[test]
fn a_table_can_be_built_a_row_at_a_time() -> TestResult {
    use fits_io::bin_table::FieldDefinition;
    use fits_io::header::TableColumnFormat;

    // Not every column has a Rust type that maps onto it, so there has to be a
    // way to build a table without going through serde. A complex column is the
    // clearest example.
    let mut table = BinTable::new(vec![
        FieldDefinition {
            format: TableColumnFormat::C32(1),
            offset: 0,
            name: "AMPLITUDE".to_string(),
            scale: None,
            zero: None,
            null: None,
            dimensions: Vec::new(),
        },
        FieldDefinition {
            format: TableColumnFormat::I32(1),
            offset: 8,
            name: "COUNT".to_string(),
            scale: None,
            zero: None,
            null: None,
            dimensions: Vec::new(),
        },
    ]);

    table.push_row(&[Value::C32(vec![(1.5, -2.5)]), Value::I32(vec![7])])?;
    table.push_row(&[Value::C32(vec![(0.0, 1.0)]), Value::I32(vec![8])])?;

    assert_eq!(table.len(), 2);

    let row = table.row(0).expect("two rows");
    assert!(
        matches!(row.get("AMPLITUDE")?, Some(Value::C32(ref v)) if v == &[(1.5, -2.5)]),
        "got {:?}",
        row.get("AMPLITUDE")?
    );
    assert!(
        matches!(row.get("COUNT")?, Some(Value::I32(ref v)) if v == &[7]),
        "got {:?}",
        row.get("COUNT")?
    );

    let row = table.row(1).expect("two rows");
    assert!(
        matches!(row.get("AMPLITUDE")?, Some(Value::C32(ref v)) if v == &[(0.0, 1.0)]),
        "got {:?}",
        row.get("AMPLITUDE")?
    );

    Ok(())
}

#[test]
fn a_row_with_the_wrong_number_of_values_is_rejected() -> TestResult {
    use fits_io::bin_table::FieldDefinition;
    use fits_io::header::TableColumnFormat;

    let mut table = BinTable::new(vec![
        FieldDefinition {
            format: TableColumnFormat::I32(1),
            offset: 0,
            name: "A".to_string(),
            scale: None,
            zero: None,
            null: None,
            dimensions: Vec::new(),
        },
        FieldDefinition {
            format: TableColumnFormat::I32(1),
            offset: 4,
            name: "B".to_string(),
            scale: None,
            zero: None,
            null: None,
            dimensions: Vec::new(),
        },
    ]);

    let error = table
        .push_row(&[Value::I32(vec![1])])
        .expect_err("a row must have one value per column");

    assert!(error.to_string().contains("2 columns"), "got: {error}");

    Ok(())
}

#[test]
fn a_bit_column_reads_out_its_individual_bits() -> TestResult {
    // `rX` keeps its bits packed, and the padding in the last byte is not part
    // of the column: a 12-bit column has 12 bits, not 16.
    let table = read_table(
        "bits.fits",
        &[("flags", "12X"), ("after", "1J")],
        &[0b1010_0000, 0b1100_0000, 0, 0, 0, 7],
    )?;

    let row = table.row(0).expect("one row");

    let Some(value) = row.get("flags")? else {
        panic!("the column exists");
    };
    let bits: Vec<bool> = value.bits().expect("a bit column").collect();

    assert_eq!(bits.len(), 12);
    assert_eq!(
        bits,
        vec![
            true, false, true, false, false, false, false, false, true, true, false, false
        ]
    );

    // And the column after it still lines up.
    assert!(
        matches!(row.get("after")?, Some(Value::I32(ref v)) if v == &[7]),
        "got {:?}",
        row.get("after")?
    );

    Ok(())
}
