//! TSCALn, TZEROn and TNULLn describe how a stored table entry maps to the
//! physical value it stands for. Reading a column without applying them gives a
//! wrong number rather than an error, so each is checked here against a file
//! whose expected values are known by construction.

mod common;

use common::{append_extension, fits_file, write_temp_fits};
use fits_io::Fits;
use fits_io::bin_table::Value;
use fits_io::fs::FsFits;
use fits_io::hdu::{BinTableHDU, ExtensionHDU};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

/// Builds a one-column binary table with the given extra header cards and data.
fn table(
    name: &str,
    cards: &[(&str, &str)],
    rows: usize,
    data: &[u8],
) -> Result<FsFits, Box<dyn Error + Send + Sync>> {
    let mut file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "0"),
            ("EXTEND", "T"),
        ],
        &[],
    );

    let bytes_per_row = (data.len() / rows).to_string();
    let rows = rows.to_string();
    let mut header = vec![
        ("XTENSION", "'BINTABLE'"),
        ("BITPIX", "8"),
        ("NAXIS", "2"),
        ("NAXIS1", bytes_per_row.as_str()),
        ("NAXIS2", rows.as_str()),
        ("PCOUNT", "0"),
        ("GCOUNT", "1"),
        ("TFIELDS", "1"),
    ];
    header.extend_from_slice(cards);

    append_extension(&mut file, &header, data);

    let path = write_temp_fits(name, &file)?;
    FsFits::open(&path)
}

/// Reads column 0 of every row of the file's single binary table extension.
fn column(fits: &FsFits) -> Result<Vec<Value>, Box<dyn Error + Send + Sync>> {
    let Some(ExtensionHDU::BinTable(hdu)) = fits.extension_hdu(0) else {
        panic!("the fixture has one binary table extension");
    };

    let table = hdu.read_table()?;
    table
        .rows()
        .map(|row| Ok(row.get_at(0)?.expect("the table has exactly one column")))
        .collect()
}

#[test]
fn an_integer_tzero_card_no_longer_fails_the_whole_header() -> TestResult {
    // TZEROn and TSCALn are typed as floating point by the standard, but a whole
    // value is almost always written without a decimal point. Rejecting that
    // spelling used to make the entire file unopenable.
    table(
        "integer-tzero.fits",
        &[("TFORM1", "'1I'"), ("TZERO1", "32768"), ("TSCAL1", "1")],
        1,
        &[0x80, 0x01],
    )?;

    Ok(())
}

#[test]
fn tzero_reads_a_signed_column_as_the_unsigned_one_it_stands_for() -> TestResult {
    // FITS has no unsigned 16-bit TFORMn code. An unsigned column is written as
    // `I` with TZERO1 = 32768, and the value it stands for is TZERO plus the
    // stored number:
    //
    //   bytes 0x8001 are i16 -32767, so the value is 32768 - 32767 = 1
    //   bytes 0x0000 are i16 0,      so the value is 32768 + 0     = 32768
    //
    // Reinterpreting the stored bit pattern instead of adding TZERO would give
    // 32769 and 0 -- every value out by half the range.
    let fits = table(
        "unsigned.fits",
        &[("TFORM1", "'1I'"), ("TZERO1", "32768"), ("TSCAL1", "1")],
        2,
        &[0x80, 0x01, 0x00, 0x00],
    )?;

    let values = column(&fits)?;

    assert!(
        matches!(values[0], Value::U16(ref v) if v == &[1]),
        "got {:?}",
        values[0]
    );
    assert!(
        matches!(values[1], Value::U16(ref v) if v == &[32768]),
        "got {:?}",
        values[1]
    );

    Ok(())
}

#[test]
fn tscal_and_tzero_convert_a_stored_entry_to_its_physical_value() -> TestResult {
    // physical = TZEROn + TSCALn * stored, so a stored 10 means 0.5 + 0.25 * 10.
    let fits = table(
        "scaled.fits",
        &[("TFORM1", "'1I'"), ("TSCAL1", "0.25"), ("TZERO1", "0.5")],
        1,
        &[0x00, 0x0A],
    )?;

    let values = column(&fits)?;

    let Value::F64(ref scaled) = values[0] else {
        panic!(
            "a scaled column reads as floating point, got {:?}",
            values[0]
        );
    };
    assert_eq!(scaled, &[3.0]);

    Ok(())
}

#[test]
fn a_column_without_scaling_cards_is_left_alone() -> TestResult {
    let fits = table("unscaled.fits", &[("TFORM1", "'1I'")], 1, &[0xFF, 0xFF])?;

    let values = column(&fits)?;

    assert!(
        matches!(values[0], Value::I16(ref v) if v == &[-1]),
        "got {:?}",
        values[0]
    );

    Ok(())
}

#[test]
fn an_identity_scaling_is_left_alone() -> TestResult {
    // TSCAL1 = 1 with TZERO1 = 0 is the default written out explicitly. It must
    // not push an integer column through a float.
    let fits = table(
        "identity.fits",
        &[("TFORM1", "'1I'"), ("TSCAL1", "1"), ("TZERO1", "0")],
        1,
        &[0x00, 0x07],
    )?;

    let values = column(&fits)?;

    assert!(
        matches!(values[0], Value::I16(ref v) if v == &[7]),
        "got {:?}",
        values[0]
    );

    Ok(())
}

#[test]
fn tnull_marks_an_entry_as_undefined() -> TestResult {
    let fits = table(
        "null.fits",
        &[("TFORM1", "'1I'"), ("TNULL1", "-32768")],
        2,
        &[0x80, 0x00, 0x00, 0x05],
    )?;

    let values = column(&fits)?;

    assert!(values[0].is_null(), "got {:?}", values[0]);
    assert!(
        matches!(values[1], Value::I16(ref v) if v == &[5]),
        "got {:?}",
        values[1]
    );

    Ok(())
}

#[cfg(feature = "serde")]
#[test]
fn a_tnull_entry_deserialises_into_none() -> TestResult {
    #[derive(serde::Deserialize, Debug)]
    struct Row {
        #[serde(rename = "COUNT")]
        count: Option<i16>,
    }

    let fits = table(
        "null-rows.fits",
        &[
            ("TFORM1", "'1I'"),
            ("TTYPE1", "'COUNT'"),
            ("TNULL1", "-32768"),
        ],
        2,
        &[0x80, 0x00, 0x00, 0x05],
    )?;

    let Some(ExtensionHDU::BinTable(hdu)) = fits.extension_hdu(0) else {
        panic!("the fixture has one binary table extension");
    };

    let rows: Vec<Row> = hdu.read_rows()?;

    assert_eq!(rows[0].count, None);
    assert_eq!(rows[1].count, Some(5));

    Ok(())
}

#[test]
fn a_variable_length_array_column_reads_its_values_from_the_heap() -> TestResult {
    // Two rows of `1PJ`: the first points at three integers in the heap, the
    // second at none. The heap follows the rows, and PCOUNT gives its size.
    let mut rows = Vec::new();
    rows.extend_from_slice(&3_i32.to_be_bytes()); // count
    rows.extend_from_slice(&0_i32.to_be_bytes()); // heap offset
    rows.extend_from_slice(&0_i32.to_be_bytes());
    rows.extend_from_slice(&0_i32.to_be_bytes());

    let mut heap = Vec::new();
    for value in [11_i32, 22, 33] {
        heap.extend_from_slice(&value.to_be_bytes());
    }

    let pcount = heap.len().to_string();
    let mut data = rows;
    data.extend_from_slice(&heap);

    let mut file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "0"),
            ("EXTEND", "T"),
        ],
        &[],
    );
    append_extension(
        &mut file,
        &[
            ("XTENSION", "'BINTABLE'"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "8"),
            ("NAXIS2", "2"),
            ("PCOUNT", pcount.as_str()),
            ("GCOUNT", "1"),
            ("TFIELDS", "1"),
            ("TFORM1", "'1PJ(3)'"),
            ("TTYPE1", "'SAMPLES'"),
        ],
        &data,
    );

    let path = write_temp_fits("vla.fits", &file)?;
    let fits = FsFits::open(&path)?;

    let values = column(&fits)?;

    assert!(
        matches!(values[0], Value::I32(ref v) if v == &[11, 22, 33]),
        "got {:?}",
        values[0]
    );
    assert!(
        matches!(values[1], Value::I32(ref v) if v.is_empty()),
        "got {:?}",
        values[1]
    );

    Ok(())
}

#[test]
fn every_unsigned_column_width_reads_the_value_it_stands_for() -> TestResult {
    // The same convention applies at all four widths, and each has its own
    // conversion to get wrong.
    //
    // `B` with TZERO -128 is the mirror image: it stores unsigned bytes to mean
    // signed ones, so stored 0 means -128.
    let fits = table(
        "unsigned-byte.fits",
        &[("TFORM1", "'1B'"), ("TZERO1", "-128"), ("TSCAL1", "1")],
        2,
        &[0x00, 0xFF],
    )?;
    let values = column(&fits)?;
    assert!(
        matches!(values[0], Value::I8(ref v) if v == &[-128]),
        "got {:?}",
        values[0]
    );
    assert!(
        matches!(values[1], Value::I8(ref v) if v == &[127]),
        "got {:?}",
        values[1]
    );

    // `J` with TZERO 2^31: stored i32::MIN means 0, stored 0 means 2^31.
    let fits = table(
        "unsigned-int.fits",
        &[
            ("TFORM1", "'1J'"),
            ("TZERO1", "2147483648"),
            ("TSCAL1", "1"),
        ],
        2,
        &[0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    )?;
    let values = column(&fits)?;
    assert!(
        matches!(values[0], Value::U32(ref v) if v == &[0]),
        "got {:?}",
        values[0]
    );
    assert!(
        matches!(values[1], Value::U32(ref v) if v == &[2147483648]),
        "got {:?}",
        values[1]
    );

    // `K` with TZERO 2^63: stored i64::MAX means u64::MAX.
    let fits = table(
        "unsigned-long.fits",
        &[
            ("TFORM1", "'1K'"),
            ("TZERO1", "9223372036854775808"),
            ("TSCAL1", "1"),
        ],
        1,
        &[0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    )?;
    let values = column(&fits)?;
    assert!(
        matches!(values[0], Value::U64(ref v) if v == &[u64::MAX]),
        "got {:?}",
        values[0]
    );

    Ok(())
}

#[cfg(feature = "serde")]
#[test]
fn a_tdim_column_deserialises_into_the_shape_it_declares() -> TestResult {
    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "GRID")]
        grid: Vec<Vec<i16>>,
    }

    // TDIM1 = '(2,3)' means the six values are 3 groups of 2, because TDIMn
    // lists the fastest-varying axis first.
    let mut data = Vec::new();
    for value in 1..=6_i16 {
        data.extend_from_slice(&value.to_be_bytes());
    }

    let fits = table(
        "tdim.fits",
        &[
            ("TFORM1", "'6I'"),
            ("TTYPE1", "'GRID'"),
            ("TDIM1", "'(2,3)'"),
        ],
        1,
        &data,
    )?;

    let Some(ExtensionHDU::BinTable(hdu)) = fits.extension_hdu(0) else {
        panic!("the fixture has one binary table extension");
    };

    let rows: Vec<Row> = hdu.read_rows()?;

    assert_eq!(rows[0].grid, vec![vec![1, 2], vec![3, 4], vec![5, 6]]);

    Ok(())
}

#[cfg(feature = "serde")]
#[test]
fn a_column_without_tdim_stays_flat() -> TestResult {
    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "GRID")]
        grid: Vec<i16>,
    }

    let mut data = Vec::new();
    for value in 1..=6_i16 {
        data.extend_from_slice(&value.to_be_bytes());
    }

    let fits = table(
        "no-tdim.fits",
        &[("TFORM1", "'6I'"), ("TTYPE1", "'GRID'")],
        1,
        &data,
    )?;

    let Some(ExtensionHDU::BinTable(hdu)) = fits.extension_hdu(0) else {
        panic!("the fixture has one binary table extension");
    };

    let rows: Vec<Row> = hdu.read_rows()?;

    assert_eq!(rows[0].grid, vec![1, 2, 3, 4, 5, 6]);

    Ok(())
}

#[cfg(feature = "serde")]
#[test]
fn a_three_axis_tdim_column_nests_three_deep() -> TestResult {
    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "CUBE")]
        cube: Vec<Vec<Vec<u8>>>,
    }

    // TDIM1 = '(2,2,2)': eight values as 2 groups of 2 groups of 2.
    let fits = table(
        "tdim-cube.fits",
        &[
            ("TFORM1", "'8B'"),
            ("TTYPE1", "'CUBE'"),
            ("TDIM1", "'(2,2,2)'"),
        ],
        1,
        &[1, 2, 3, 4, 5, 6, 7, 8],
    )?;

    let Some(ExtensionHDU::BinTable(hdu)) = fits.extension_hdu(0) else {
        panic!("the fixture has one binary table extension");
    };

    let rows: Vec<Row> = hdu.read_rows()?;

    assert_eq!(
        rows[0].cube,
        vec![vec![vec![1, 2], vec![3, 4]], vec![vec![5, 6], vec![7, 8]],]
    );

    Ok(())
}
