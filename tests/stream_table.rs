//! Streaming a binary table must agree with reading it whole, and must not need
//! the whole table in memory to do it.

mod common;

use common::{append_extension, fits_file, write_temp_fits};
use fits_io::Fits;
use fits_io::bin_table::Value;
use fits_io::fs::FsFits;
use fits_io::hdu::{BinTableHDU, ExtensionHDU};
use futures::StreamExt;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

/// A table of `rows` rows, each one big-endian `1J` counter holding its index.
fn counter_table(name: &str, rows: usize) -> Result<FsFits, Box<dyn Error + Send + Sync>> {
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
    for row in 0..rows {
        data.extend_from_slice(&(row as i32).to_be_bytes());
    }

    let rows = rows.to_string();
    append_extension(
        &mut file,
        &[
            ("XTENSION", "'BINTABLE'"),
            ("BITPIX", "8"),
            ("NAXIS", "2"),
            ("NAXIS1", "4"),
            ("NAXIS2", rows.as_str()),
            ("PCOUNT", "0"),
            ("GCOUNT", "1"),
            ("TFIELDS", "1"),
            ("TFORM1", "'1J'"),
            ("TTYPE1", "'INDEX'"),
        ],
        &data,
    );

    let path = write_temp_fits(name, &file)?;
    FsFits::open(&path)
}

fn table_hdu(fits: &FsFits) -> &impl BinTableHDU {
    let Some(ExtensionHDU::BinTable(hdu)) = fits.extension_hdu(0) else {
        panic!("the fixture has one binary table extension");
    };
    hdu
}

#[tokio::test]
async fn streams_every_row_in_order() -> TestResult {
    let fits = counter_table("stream-rows.fits", 5)?;
    let hdu = table_hdu(&fits);

    let rows: Vec<_> = hdu.stream_table_rows_raw()?.collect().await;

    assert_eq!(rows.len(), 5);
    for (index, row) in rows.iter().enumerate() {
        let value = row.get("INDEX")?.expect("the table has an INDEX column");
        assert!(
            matches!(value, Value::I32(ref v) if v == &[index as i32]),
            "row {index} got {value:?}"
        );
    }

    Ok(())
}

#[tokio::test]
async fn streaming_and_reading_agree() -> TestResult {
    // A table larger than one read block, so the stream has to carry a partial
    // row across a block boundary rather than getting each row whole.
    let fits = counter_table("stream-blocks.fits", 5000)?;
    let hdu = table_hdu(&fits);

    let read: Vec<_> = hdu
        .read_table()?
        .rows()
        .map(|row| row.get_at(0).unwrap().unwrap())
        .map(|value| match value {
            Value::I32(values) => values[0],
            other => panic!("expected an I32 column, got {other:?}"),
        })
        .collect();

    let streamed: Vec<_> = hdu
        .stream_table_rows_raw()?
        .map(|row| match row.get_at(0).unwrap().unwrap() {
            Value::I32(values) => values[0],
            other => panic!("expected an I32 column, got {other:?}"),
        })
        .collect()
        .await;

    assert_eq!(streamed.len(), 5000);
    assert_eq!(read, streamed);

    Ok(())
}

#[tokio::test]
async fn an_empty_table_streams_nothing() -> TestResult {
    let fits = counter_table("stream-empty.fits", 0)?;
    let hdu = table_hdu(&fits);

    let rows: Vec<_> = hdu.stream_table_rows_raw()?.collect().await;

    assert!(rows.is_empty());

    Ok(())
}

#[cfg(feature = "serde")]
#[tokio::test]
async fn streams_rows_deserialised_into_a_struct() -> TestResult {
    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(rename = "INDEX")]
        index: i32,
    }

    let fits = counter_table("stream-typed.fits", 3)?;
    let hdu = table_hdu(&fits);

    let rows: Vec<Row> = hdu
        .stream_table_rows::<Row>()?
        .map(|row| row.expect("every row fits the struct"))
        .collect()
        .await;

    assert_eq!(
        rows,
        vec![Row { index: 0 }, Row { index: 1 }, Row { index: 2 }]
    );

    Ok(())
}

#[tokio::test]
async fn streams_variable_length_array_columns_from_the_heap() -> TestResult {
    // Streaming keeps only a block of rows in memory, but a variable length
    // array column points into the heap, which sits after every row. The stream
    // has to have read it before the first row arrives.
    let mut rows = Vec::new();
    for row in 0..2_i32 {
        rows.extend_from_slice(&1_i32.to_be_bytes()); // one element each
        rows.extend_from_slice(&(row * 4).to_be_bytes()); // heap offset
    }

    let mut heap = Vec::new();
    for value in [42_i32, 43] {
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
            ("TFORM1", "'1PJ(1)'"),
            ("TTYPE1", "'SAMPLE'"),
        ],
        &data,
    );

    let path = write_temp_fits("stream-vla.fits", &file)?;
    let fits = FsFits::open(&path)?;
    let hdu = table_hdu(&fits);

    let streamed: Vec<_> = hdu.stream_table_rows_raw()?.collect().await;

    assert_eq!(streamed.len(), 2);
    for (index, expected) in [42, 43].into_iter().enumerate() {
        let value = streamed[index].get("SAMPLE")?.expect("the column exists");
        assert!(
            matches!(value, Value::I32(ref v) if v == &[expected]),
            "row {index} got {value:?}"
        );
    }

    Ok(())
}
