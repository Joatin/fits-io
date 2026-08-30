//! Serialising rows into a binary table, and reading them back.

mod common;

use fits_io::bin_table::{Value, from_bin_table, to_bin_table};
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
    count: i32,
}

#[test]
fn rows_round_trip_through_a_binary_table() -> TestResult {
    let stars = vec![
        Star {
            name: "Vega".into(),
            magnitude: 0.03,
            count: 1,
        },
        Star {
            name: "Betelgeuse".into(),
            magnitude: 0.42,
            count: 2,
        },
    ];

    let table = to_bin_table(&stars)?;

    assert_eq!(table.len(), 2);

    let read: Vec<Star> = from_bin_table(&table)?;
    assert_eq!(read, stars);

    Ok(())
}

#[test]
fn a_bare_struct_is_a_table_of_one_row() -> TestResult {
    let table = to_bin_table(&Star {
        name: "Sirius".into(),
        magnitude: -1.46,
        count: 7,
    })?;

    assert_eq!(table.len(), 1);

    let read: Vec<Star> = from_bin_table(&table)?;
    assert_eq!(read[0].name, "Sirius");
    assert_eq!(read[0].count, 7);

    Ok(())
}

#[test]
fn a_character_column_is_as_wide_as_its_longest_string() -> TestResult {
    let table = to_bin_table(&vec![
        Star {
            name: "Vega".into(),
            magnitude: 0.0,
            count: 0,
        },
        Star {
            name: "Betelgeuse".into(),
            magnitude: 0.0,
            count: 0,
        },
    ])?;

    // "Betelgeuse" is ten characters, so a narrower field would truncate it.
    assert_eq!(
        String::from(table.field_definitions()[0].format),
        "10A".to_string()
    );

    let read: Vec<Star> = from_bin_table(&table)?;
    assert_eq!(read[1].name, "Betelgeuse");

    Ok(())
}

#[test]
fn an_integer_column_keeps_the_width_its_rust_type_asked_for() -> TestResult {
    #[derive(Serialize)]
    struct Row {
        byte: u8,
        short: i16,
        int: i32,
        long: i64,
    }

    // Every value here would fit in a byte. Choosing the column type from the
    // values rather than the type would make an `i64` field come out as `B`
    // because this particular batch of rows happened to be small.
    let table = to_bin_table(&vec![Row {
        byte: 1,
        short: 2,
        int: 3,
        long: 4,
    }])?;

    let formats: Vec<String> = table
        .field_definitions()
        .iter()
        .map(|field| String::from(field.format))
        .collect();

    assert_eq!(
        formats,
        vec![
            "1B".to_string(),
            "1I".to_string(),
            "1J".to_string(),
            "1K".to_string()
        ]
    );

    Ok(())
}

#[test]
fn a_column_is_wide_enough_for_every_row_in_it() -> TestResult {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "VALUE")]
        value: i32,
    }

    // A value that needs all 32 bits must survive; writing it into a narrower
    // column would keep the wrong end of the number.
    let rows = vec![Row { value: 100_000 }, Row { value: -7 }];
    let table = to_bin_table(&rows)?;

    let read: Vec<Row> = from_bin_table(&table)?;
    assert_eq!(read, rows);

    Ok(())
}

#[test]
fn a_none_field_becomes_an_undefined_entry() -> TestResult {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "COUNT")]
        count: Option<i32>,
    }

    let rows = vec![Row { count: Some(5) }, Row { count: None }];

    let table = to_bin_table(&rows)?;

    // The column has to declare a TNULLn, or the sentinel it writes is just a
    // number like any other.
    assert!(table.field_definitions()[0].null.is_some());

    let first = table.row(0).expect("two rows");
    assert!(
        matches!(first.get("COUNT")?, Some(Value::I32(ref v)) if v == &[5]),
        "got {:?}",
        first.get("COUNT")?
    );

    let second = table.row(1).expect("two rows");
    assert!(
        second.get("COUNT")?.expect("the column exists").is_null(),
        "a None field must read back as undefined"
    );

    let read: Vec<Row> = from_bin_table(&table)?;
    assert_eq!(read, rows);

    Ok(())
}

#[test]
fn an_array_field_becomes_a_repeated_column() -> TestResult {
    #[derive(Serialize)]
    struct Row {
        samples: Vec<i16>,
    }

    let table = to_bin_table(&vec![
        Row {
            samples: vec![1, 2, 3],
        },
        Row {
            samples: vec![4, 5, 6],
        },
    ])?;

    assert_eq!(
        String::from(table.field_definitions()[0].format),
        "3I".to_string()
    );

    let row = table.row(1).expect("two rows");
    assert!(
        matches!(row.get("samples")?, Some(Value::I16(ref v)) if v == &[4, 5, 6]),
        "got {:?}",
        row.get("samples")?
    );

    Ok(())
}

#[test]
fn rows_that_disagree_about_their_columns_are_rejected() {
    // Serialising these into one table would silently drop or misplace a column.
    #[derive(Serialize)]
    #[serde(untagged)]
    enum Row {
        One { a: i32 },
        Other { b: i32 },
    }

    let error = to_bin_table(&vec![Row::One { a: 1 }, Row::Other { b: 2 }])
        .expect_err("rows must agree on their columns");

    assert!(error.to_string().contains("same columns"), "got: {error}");
}

#[test]
fn something_that_is_not_a_table_is_rejected() {
    // A bare number is not a row, and pretending it is would produce a table
    // with no columns rather than an error.
    let error = to_bin_table(&42_i32).expect_err("a number is not a table");

    assert!(
        error.to_string().contains("sequence of structs"),
        "got: {error}"
    );
}

#[test]
fn an_empty_table_has_no_rows() -> TestResult {
    let table = to_bin_table(&Vec::<Star>::new())?;

    assert!(table.is_empty());
    assert_eq!(table.len(), 0);

    Ok(())
}

#[test]
fn a_table_can_be_written_into_a_file_and_read_back() -> TestResult {
    use fits_io::hdu::{BinTableHDU, ExtensionHDU};
    use fits_io::{Fits, FitsSlice, SliceBinTableHDU};

    let stars = vec![
        Star {
            name: "Vega".into(),
            magnitude: 0.03,
            count: 1,
        },
        Star {
            name: "Betelgeuse".into(),
            magnitude: 0.42,
            count: 2,
        },
    ];

    // Building the table is only half of it: the header cards describing the
    // columns have to travel with the bytes, or nothing can decode them.
    let mut fits = FitsSlice::new();
    let mut hdu = SliceBinTableHDU::from_table(&to_bin_table(&stars)?)?;
    hdu.set_rows(&stars)?;
    fits.push_extension(ExtensionHDU::BinTable(hdu));

    let bytes = fits.to_vec()?;
    let reopened = FitsSlice::from_slice(&bytes)?;

    let Some(ExtensionHDU::BinTable(hdu)) = reopened.extension_hdu(0) else {
        panic!("the extension is a binary table");
    };

    let read: Vec<Star> = hdu.read_rows()?;
    assert_eq!(read, stars);

    Ok(())
}

#[test]
fn a_written_table_carries_its_column_names_and_formats() -> TestResult {
    use fits_io::hdu::{ExtensionHDU, HDU};
    use fits_io::{Fits, FitsSlice, SliceBinTableHDU};

    let mut fits = FitsSlice::new();
    fits.push_extension(ExtensionHDU::BinTable(SliceBinTableHDU::from_table(
        &to_bin_table(&vec![Star {
            name: "Sirius".into(),
            magnitude: -1.46,
            count: 7,
        }])?,
    )?));

    let bytes = fits.to_vec()?;
    let reopened = FitsSlice::from_slice(&bytes)?;

    let Some(ExtensionHDU::BinTable(hdu)) = reopened.extension_hdu(0) else {
        panic!("the extension is a binary table");
    };

    let header = hdu.header();
    assert_eq!(header.table_fields(), Some(3));
    assert_eq!(header.table_column_type(0), Some("NAME"));
    assert_eq!(header.table_format(1), Some("1D"));

    Ok(())
}

#[test]
fn a_column_with_undefined_entries_writes_its_tnull_card() -> TestResult {
    use fits_io::hdu::{BinTableHDU, ExtensionHDU, HDU};
    use fits_io::{Fits, FitsSlice, SliceBinTableHDU};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "COUNT")]
        count: Option<i32>,
    }

    let rows = vec![Row { count: Some(5) }, Row { count: None }];

    let mut fits = FitsSlice::new();
    fits.push_extension(ExtensionHDU::BinTable(SliceBinTableHDU::from_table(
        &to_bin_table(&rows)?,
    )?));

    let reopened = FitsSlice::from_slice(&fits.to_vec()?)?;

    let Some(ExtensionHDU::BinTable(hdu)) = reopened.extension_hdu(0) else {
        panic!("the extension is a binary table");
    };

    // Without the TNULLn card the sentinel is just a number like any other, and
    // the `None` comes back as `Some(i32::MIN)`.
    assert!(hdu.header().table_null_value(0).is_some());

    let read: Vec<Row> = hdu.read_rows()?;
    assert_eq!(read, rows);

    Ok(())
}

#[test]
fn an_extension_can_be_removed() -> TestResult {
    use fits_io::hdu::ExtensionHDU;
    use fits_io::{Fits, FitsSlice, SliceBinTableHDU};

    let mut fits = FitsSlice::new();
    fits.push_extension(ExtensionHDU::BinTable(SliceBinTableHDU::from_table(
        &to_bin_table(&Vec::<Star>::new())?,
    )?));

    assert_eq!(fits.extension_count(), 1);
    assert!(fits.remove_extension(0).is_some());
    assert_eq!(fits.extension_count(), 0);
    assert!(fits.remove_extension(0).is_none());

    Ok(())
}

#[test]
fn a_large_u64_survives_the_round_trip() -> TestResult {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "V")]
        value: u64,
    }

    // FITS has no unsigned 64-bit column, so this has to go out as a `K` with a
    // TZERO of 2^63. Casting straight to `i64` would write `u64::MAX` as -1.
    let rows = vec![
        Row { value: u64::MAX },
        Row { value: 0 },
        Row {
            value: i64::MAX as u64 + 1,
        },
    ];

    let table = to_bin_table(&rows)?;

    assert_eq!(
        String::from(table.field_definitions()[0].format),
        "1K".to_string()
    );
    assert!(
        table.field_definitions()[0].zero.is_some(),
        "an unsigned column must record its TZERO, or it reads back signed"
    );

    let read: Vec<Row> = from_bin_table(&table)?;
    assert_eq!(read, rows);

    Ok(())
}

#[test]
fn a_ragged_column_is_written_as_a_variable_length_array() -> TestResult {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "SAMPLES")]
        samples: Vec<i32>,
    }

    // Rows of different lengths do not fit a fixed-width field. Padding the
    // short ones out would invent values that read back as real ones.
    let rows = vec![
        Row {
            samples: vec![1, 2, 3],
        },
        Row { samples: vec![4] },
        Row { samples: vec![] },
    ];

    let table = to_bin_table(&rows)?;

    assert!(
        String::from(table.field_definitions()[0].format).contains('P'),
        "got {}",
        String::from(table.field_definitions()[0].format)
    );
    assert!(table.heap_len() > 0, "the values live in the heap");

    let read: Vec<Row> = from_bin_table(&table)?;
    assert_eq!(read, rows);

    Ok(())
}

#[test]
fn a_column_of_equal_length_arrays_stays_fixed_width() -> TestResult {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "SAMPLES")]
        samples: Vec<i32>,
    }

    // Nothing is gained by putting these in the heap.
    let rows = vec![
        Row {
            samples: vec![1, 2, 3],
        },
        Row {
            samples: vec![4, 5, 6],
        },
    ];

    let table = to_bin_table(&rows)?;

    assert_eq!(
        String::from(table.field_definitions()[0].format),
        "3J".to_string()
    );
    assert_eq!(table.heap_len(), 0);

    let read: Vec<Row> = from_bin_table(&table)?;
    assert_eq!(read, rows);

    Ok(())
}

#[test]
fn a_variable_length_column_survives_a_whole_file() -> TestResult {
    use fits_io::hdu::{BinTableHDU, ExtensionHDU};
    use fits_io::{Fits, FitsSlice, SliceBinTableHDU};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "SAMPLES")]
        samples: Vec<f64>,
    }

    let rows = vec![
        Row {
            samples: vec![1.5, 2.5],
        },
        Row { samples: vec![3.5] },
    ];

    // The heap sits after the rows, and PCOUNT is what tells a reader it is
    // there at all.
    let mut fits = FitsSlice::new();
    fits.push_extension(ExtensionHDU::BinTable(SliceBinTableHDU::from_table(
        &to_bin_table(&rows)?,
    )?));

    let reopened = FitsSlice::from_slice(&fits.to_vec()?)?;
    let Some(ExtensionHDU::BinTable(hdu)) = reopened.extension_hdu(0) else {
        panic!("the extension is a binary table");
    };

    let read: Vec<Row> = hdu.read_rows()?;
    assert_eq!(read, rows);

    Ok(())
}

#[test]
fn a_nested_array_keeps_its_shape() -> TestResult {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "GRID")]
        grid: Vec<Vec<i32>>,
    }

    // FITS stores a multidimensional column flat, with a TDIMn card giving the
    // shape. Without that card the six values read back as six arrays of one.
    let rows = vec![Row {
        grid: vec![vec![1, 2, 3], vec![4, 5, 6]],
    }];

    let table = to_bin_table(&rows)?;

    assert_eq!(
        String::from(table.field_definitions()[0].format),
        "6J".to_string()
    );
    // TDIMn lists the fastest-varying axis first, so 2 groups of 3 is (3,2).
    assert_eq!(table.field_definitions()[0].dimensions, vec![3, 2]);

    let read: Vec<Row> = from_bin_table(&table)?;
    assert_eq!(read, rows);

    Ok(())
}

#[test]
fn a_nested_array_column_survives_a_whole_file() -> TestResult {
    use fits_io::hdu::{BinTableHDU, ExtensionHDU, HDU};
    use fits_io::{Fits, FitsSlice, SliceBinTableHDU};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "GRID")]
        grid: Vec<Vec<u8>>,
    }

    let rows = vec![Row {
        grid: vec![vec![1, 2], vec![3, 4]],
    }];

    let mut fits = FitsSlice::new();
    fits.push_extension(ExtensionHDU::BinTable(SliceBinTableHDU::from_table(
        &to_bin_table(&rows)?,
    )?));

    let reopened = FitsSlice::from_slice(&fits.to_vec()?)?;
    let Some(ExtensionHDU::BinTable(hdu)) = reopened.extension_hdu(0) else {
        panic!("the extension is a binary table");
    };

    // The shape only survives if the TDIMn card was actually written out.
    assert_eq!(hdu.header().table_dimensions(0), Some("(2,2)"));

    let read: Vec<Row> = hdu.read_rows()?;
    assert_eq!(read, rows);

    Ok(())
}

#[test]
fn a_ragged_nested_array_is_written_without_a_shape() -> TestResult {
    #[derive(Serialize, Debug)]
    struct Row {
        #[serde(rename = "GRID")]
        grid: Vec<Vec<i32>>,
    }

    // There is no rectangular shape for TDIMn to describe here, so the values
    // go in flat rather than under a shape that would be a lie.
    let table = to_bin_table(&vec![Row {
        grid: vec![vec![1, 2], vec![3]],
    }])?;

    assert!(table.field_definitions()[0].dimensions.is_empty());

    Ok(())
}
