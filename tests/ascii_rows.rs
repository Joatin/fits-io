//! An ASCII table's rows deserialize into your own structs, and serialize back,
//! the same way a binary table's do.

mod common;

use fits_io::ascii_table::{AsciiColumnFormat, AsciiTable, from_ascii_table, to_ascii_table};
use fits_io::bin_table::Value;
use serde::{Deserialize, Serialize};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Star {
    #[serde(rename = "NAME")]
    name: String,
    #[serde(rename = "MAG")]
    magnitude: f64,
    #[serde(rename = "COUNT")]
    count: i64,
}

#[test]
fn rows_round_trip_through_an_ascii_table() -> TestResult {
    let stars = vec![
        Star {
            name: "Vega".into(),
            magnitude: 0.03,
            count: 1,
        },
        Star {
            name: "Betelgeuse".into(),
            magnitude: -0.42,
            count: 200,
        },
    ];

    let table = to_ascii_table(&stars)?;
    assert_eq!(table.len(), 2);

    let read: Vec<Star> = from_ascii_table(&table)?;
    assert_eq!(read, stars);

    Ok(())
}

#[test]
fn a_float_column_reads_back_the_same_f64() -> TestResult {
    // An ASCII table stores numbers as text, so a column that is too narrow or
    // carries too few digits loses precision silently.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "VALUE")]
        value: f64,
    }

    let rows = vec![
        Row {
            value: std::f64::consts::PI,
        },
        Row { value: 1e-300 },
        Row {
            value: -1.7976931348623157e308,
        },
        Row { value: 0.1 + 0.2 },
    ];

    let read: Vec<Row> = from_ascii_table(&to_ascii_table(&rows)?)?;

    assert_eq!(read, rows);

    Ok(())
}

#[test]
fn a_column_is_as_wide_as_its_widest_value() -> TestResult {
    let table = to_ascii_table(&vec![
        Star {
            name: "Vega".into(),
            magnitude: 0.0,
            count: 1,
        },
        Star {
            name: "Betelgeuse".into(),
            magnitude: 0.0,
            count: 1_000_000,
        },
    ])?;

    let formats: Vec<String> = table
        .field_definitions()
        .iter()
        .map(|field| String::from(field.format))
        .collect();

    // "Betelgeuse" is ten characters and 1000000 is seven digits.
    assert_eq!(formats[0], "A10");
    assert_eq!(formats[2], "I7");

    Ok(())
}

#[test]
fn a_negative_number_leaves_room_for_its_sign() -> TestResult {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "VALUE")]
        value: i64,
    }

    // A field sized for "1000" would write "-1000" as blanks.
    let rows = vec![Row { value: -1000 }, Row { value: 5 }];

    let read: Vec<Row> = from_ascii_table(&to_ascii_table(&rows)?)?;
    assert_eq!(read, rows);

    Ok(())
}

#[test]
fn a_table_written_through_an_hdu_reads_back() -> TestResult {
    use fits_io::hdu::{AsciiTableHDU, ExtensionHDU};
    use fits_io::{Fits, FitsSlice, SliceAsciiTableHDU};

    let stars = vec![Star {
        name: "Sirius".into(),
        magnitude: -1.46,
        count: 7,
    }];

    let mut fits = FitsSlice::new();
    let mut hdu = SliceAsciiTableHDU::from_table(&AsciiTable::from_columns(&[]))?;
    hdu.set_rows(&stars)?;
    fits.push_extension(ExtensionHDU::AsciiTable(hdu));

    let reopened = FitsSlice::from_slice(&fits.to_vec()?)?;

    let Some(ExtensionHDU::AsciiTable(hdu)) = reopened.extension_hdu(0) else {
        panic!("the extension is an ASCII table");
    };

    let read: Vec<Star> = hdu.read_rows()?;
    assert_eq!(read, stars);

    Ok(())
}

#[test]
fn a_column_can_declare_the_text_that_marks_an_undefined_entry() -> TestResult {
    let mut table = AsciiTable::from_columns(&[
        ("NAME".to_string(), AsciiColumnFormat::Character(6)),
        ("COUNT".to_string(), AsciiColumnFormat::Integer(6)),
    ]);

    table.field_definitions_mut()[1].null = Some("---".to_string());

    table.push_row(&[Value::String("M31".into()), Value::Null])?;

    let row = table.row(0).expect("one row");
    assert!(row.get("COUNT")?.expect("the column exists").is_null());

    // And the text itself is what landed in the field.
    let text = String::from_utf8_lossy(table.data()).to_string();
    assert!(text.contains("---"), "got {text:?}");

    Ok(())
}

#[test]
fn an_empty_table_has_no_rows() -> TestResult {
    let table = to_ascii_table(&Vec::<Star>::new())?;

    assert!(table.is_empty());

    Ok(())
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn rows_stream_deserialised_out_of_an_ascii_table() -> TestResult {
    use fits_io::hdu::{AsciiTableHDU, ExtensionHDU};
    use fits_io::{Fits, FitsSlice, SliceAsciiTableHDU};
    use futures::StreamExt;

    let stars = vec![
        Star {
            name: "Vega".into(),
            magnitude: 0.03,
            count: 1,
        },
        Star {
            name: "Rigel".into(),
            magnitude: 0.13,
            count: 2,
        },
    ];

    let mut fits = FitsSlice::new();
    let mut hdu = SliceAsciiTableHDU::from_table(&AsciiTable::from_columns(&[]))?;
    hdu.set_rows(&stars)?;
    fits.push_extension(ExtensionHDU::AsciiTable(hdu));

    let reopened = FitsSlice::from_slice(&fits.to_vec()?)?;
    let Some(ExtensionHDU::AsciiTable(hdu)) = reopened.extension_hdu(0) else {
        panic!("the extension is an ASCII table");
    };

    let streamed: Vec<Star> = hdu
        .stream_table_rows::<Star>()?
        .map(|row| row.expect("every row fits the struct"))
        .collect()
        .await;

    assert_eq!(streamed, stars);

    Ok(())
}
