//! ASCII tables store their values as fixed-width text, positioned by TBCOLn.

mod common;

use common::{append_extension, fits_file, write_temp_fits};
use fits_io::Fits;
use fits_io::bin_table::Value;
use fits_io::fs::FsFits;
use fits_io::hdu::{AsciiTableHDU, ExtensionHDU};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

/// A two-column ASCII table: an 8-character name and a 10-character integer.
fn catalogue(name: &str, rows: &[&str]) -> Result<FsFits, Box<dyn Error + Send + Sync>> {
    let mut file = fits_file(
        &[
            ("SIMPLE", "T"),
            ("BITPIX", "8"),
            ("NAXIS", "0"),
            ("EXTEND", "T"),
        ],
        &[],
    );

    let mut data = Vec::new();
    for row in rows {
        assert_eq!(row.len(), 18, "every row is NAXIS1 characters wide");
        data.extend_from_slice(row.as_bytes());
    }

    let count = rows.len().to_string();
    append_extension(
        &mut file,
        &[
            ("XTENSION", "'TABLE   '"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "18"),
            ("NAXIS2", count.as_str()),
            ("PCOUNT", "0"),
            ("GCOUNT", "1"),
            ("TFIELDS", "2"),
            ("TTYPE1", "'NAME'"),
            ("TBCOL1", "1"),
            ("TFORM1", "'A8'"),
            ("TTYPE2", "'COUNT'"),
            ("TBCOL2", "9"),
            ("TFORM2", "'I10'"),
        ],
        &data,
    );

    let path = write_temp_fits(name, &file)?;
    FsFits::open(&path)
}

fn table_hdu(fits: &FsFits) -> &impl AsciiTableHDU {
    let Some(ExtensionHDU::AsciiTable(hdu)) = fits.extension_hdu(0) else {
        panic!("the fixture has one ASCII table extension");
    };
    hdu
}

#[test]
fn an_ascii_table_extension_no_longer_fails_the_whole_file() -> TestResult {
    // `A8` and `I10` are not binary table format codes. Insisting on those used
    // to make every file with an ASCII table unopenable.
    catalogue("ascii-open.fits", &["M31             42"])?;

    Ok(())
}

#[test]
fn columns_are_read_from_the_positions_tbcol_gives() -> TestResult {
    let fits = catalogue(
        "ascii-read.fits",
        &["M31             42", "NGC 4565      1234"],
    )?;
    let table = table_hdu(&fits).read_table()?;

    assert_eq!(table.len(), 2);

    let first = table.row(0).expect("the table has two rows");
    assert!(
        matches!(first.get("NAME")?, Some(Value::String(ref text)) if text == "M31"),
        "got {:?}",
        first.get("NAME")?
    );
    assert!(
        matches!(first.get("COUNT")?, Some(Value::I64(ref v)) if v == &[42]),
        "got {:?}",
        first.get("COUNT")?
    );

    let second = table.row(1).expect("the table has two rows");
    assert!(
        matches!(second.get("NAME")?, Some(Value::String(ref text)) if text == "NGC 4565"),
        "got {:?}",
        second.get("NAME")?
    );
    assert!(
        matches!(second.get("COUNT")?, Some(Value::I64(ref v)) if v == &[1234]),
        "got {:?}",
        second.get("COUNT")?
    );

    Ok(())
}

#[test]
fn a_blank_field_is_undefined() -> TestResult {
    let fits = catalogue("ascii-blank.fits", &["M31               "])?;
    let table = table_hdu(&fits).read_table()?;

    let row = table.row(0).expect("the table has one row");

    assert!(
        row.get("COUNT")?.expect("the column exists").is_null(),
        "an all-blank numeric field carries no value"
    );

    Ok(())
}

#[test]
fn a_table_can_be_written_and_read_back() -> TestResult {
    use fits_io::FitsSlice;
    use fits_io::ascii_table::{AsciiColumnFormat, AsciiTable};

    let mut table = AsciiTable::from_columns(&[
        ("NAME".to_string(), AsciiColumnFormat::Character(10)),
        ("COUNT".to_string(), AsciiColumnFormat::Integer(6)),
        ("MAG".to_string(), AsciiColumnFormat::Fixed(8, 2)),
    ]);

    table.push_row(&[
        Value::String("M31".into()),
        Value::I64(vec![42]),
        Value::F64(vec![3.44]),
    ])?;
    table.push_row(&[
        Value::String("Betelgeuse".into()),
        Value::Null,
        Value::F64(vec![-1.5]),
    ])?;

    let mut fits = FitsSlice::new();
    fits.push_extension(fits_io::hdu::ExtensionHDU::AsciiTable(
        fits_io::SliceAsciiTableHDU::from_table(&table)?,
    ));

    let reopened = FitsSlice::from_slice(&fits.to_vec()?)?;

    let Some(ExtensionHDU::AsciiTable(hdu)) = reopened.extension_hdu(0) else {
        panic!("the extension is an ASCII table");
    };

    let read = hdu.read_table()?;
    assert_eq!(read.len(), 2);

    let first = read.row(0).expect("two rows");
    assert!(
        matches!(first.get("NAME")?, Some(Value::String(ref t)) if t == "M31"),
        "got {:?}",
        first.get("NAME")?
    );
    assert!(
        matches!(first.get("COUNT")?, Some(Value::I64(ref v)) if v == &[42]),
        "got {:?}",
        first.get("COUNT")?
    );
    assert!(
        matches!(first.get("MAG")?, Some(Value::F64(ref v)) if (v[0] - 3.44).abs() < 1e-9),
        "got {:?}",
        first.get("MAG")?
    );

    let second = read.row(1).expect("two rows");
    assert!(
        matches!(second.get("NAME")?, Some(Value::String(ref t)) if t == "Betelgeuse"),
        "got {:?}",
        second.get("NAME")?
    );
    assert!(
        second.get("COUNT")?.expect("the column exists").is_null(),
        "an undefined entry must come back undefined"
    );

    Ok(())
}

#[test]
fn a_row_with_the_wrong_number_of_values_is_rejected() -> TestResult {
    use fits_io::ascii_table::{AsciiColumnFormat, AsciiTable};

    let mut table = AsciiTable::from_columns(&[
        ("A".to_string(), AsciiColumnFormat::Integer(4)),
        ("B".to_string(), AsciiColumnFormat::Integer(4)),
    ]);

    // A short row would leave the last field holding whatever the padding was.
    let error = table
        .push_row(&[Value::I64(vec![1])])
        .expect_err("a row must have one value per column");

    assert!(error.to_string().contains("2 columns"), "got: {error}");

    Ok(())
}
