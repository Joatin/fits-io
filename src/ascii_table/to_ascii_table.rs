use crate::ascii_table::{AsciiColumnFormat, AsciiTable, write};
use crate::bin_table::Value;
use crate::bin_table::to_bin_table::{Error, collect_rows, column_names};
use serde::Serialize;

/// How many decimal places a floating point column is written with.
///
/// Seventeen significant digits is what it takes to write an `f64` and read the
/// same number back, and `Ew.d` puts one digit before the point.
const DECIMALS: usize = 16;

/// Serialises rows into an ASCII table.
///
/// `data` is a sequence of structs — a `Vec<Row>` or a slice of them — where
/// each struct's fields are the table's columns, exactly as for
/// [`to_bin_table`](crate::bin_table::to_bin_table).
///
/// An ASCII table stores everything as text, so each column's field is made wide
/// enough for the widest value that goes in it. Floating point columns are
/// written in exponential notation with enough digits to read back the same
/// `f64`, which is verbose but lossless.
pub fn to_ascii_table<T: Serialize>(data: &T) -> Result<AsciiTable, Error> {
    let rows = collect_rows(data)?;
    let names = column_names(&rows)?;

    if names.is_empty() {
        return Ok(AsciiTable::from_columns(&[]));
    }

    let columns: Vec<(String, AsciiColumnFormat)> = names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            (
                (*name).to_string(),
                column_format(rows.iter().map(|row| &row[index].1.value)),
            )
        })
        .collect();

    let mut table = AsciiTable::from_columns(&columns);
    for row in &rows {
        let values: Vec<Value> = row.iter().map(|(_, column)| column.value.clone()).collect();
        table
            .push_row(&values)
            .map_err(|error| Error::Custom(error.to_string()))?;
    }

    Ok(table)
}

/// Picks a TFORMn wide enough for every value in a column.
fn column_format<'a>(values: impl Iterator<Item = &'a Value> + Clone) -> AsciiColumnFormat {
    let is = |kind: fn(&Value) -> bool| values.clone().any(kind);

    // Text wins over everything: a column holding one string is a text column.
    if is(|value| matches!(value, Value::String(_) | Value::StringArray(_))) {
        return AsciiColumnFormat::Character(width(values, AsciiColumnFormat::Character(0)));
    }

    if is(|value| {
        matches!(
            value,
            Value::F32(_) | Value::F64(_) | Value::C32(_) | Value::M64(_)
        )
    }) {
        // Fixed-point is easier to read, and an ASCII table exists to be read,
        // so it is used wherever it can hold the values exactly.
        if let Some(decimals) = fixed_decimals(values.clone()) {
            let candidate = AsciiColumnFormat::Fixed(0, decimals);
            return AsciiColumnFormat::Fixed(width(values, candidate), decimals);
        }

        let candidate = AsciiColumnFormat::Exponential(0, DECIMALS);
        return AsciiColumnFormat::Exponential(width(values, candidate), DECIMALS);
    }

    AsciiColumnFormat::Integer(width(values, AsciiColumnFormat::Integer(0)))
}

/// How many decimal places would write every value in a column exactly, if any
/// reasonable number does.
///
/// Exponential notation always round-trips, but `1.5` reading back as
/// `1.5000000000000000E0` is a poor thing to hand someone looking at a table
/// meant for human eyes. Fixed-point is used when it loses nothing.
fn fixed_decimals<'a>(values: impl Iterator<Item = &'a Value> + Clone) -> Option<usize> {
    /// Beyond this a fixed-point field is both unwieldy and unlikely to help.
    const MOST: usize = 12;

    let numbers: Vec<f64> = values
        .flat_map(|value| match value {
            Value::F32(values) => values.iter().map(|value| *value as f64).collect(),
            Value::F64(values) => values.clone(),
            _ => Vec::new(),
        })
        .collect();

    // A value too large for fixed-point to state compactly, or one that is not a
    // number at all, belongs in exponential notation.
    if numbers
        .iter()
        .any(|number| !number.is_finite() || number.abs() >= 1e15)
    {
        return None;
    }

    (0..=MOST).find(|decimals| {
        numbers.iter().all(|number| {
            format!("{:.*}", decimals, number)
                .parse::<f64>()
                .is_ok_and(|read| read == *number)
        })
    })
}

/// The width of the widest value in a column, once written.
///
/// Rendering each value is the only reliable way to size a text field: how many
/// characters a number takes depends on its sign, its magnitude and the notation
/// it is written in.
fn width<'a>(values: impl Iterator<Item = &'a Value>, format: AsciiColumnFormat) -> usize {
    values
        .map(|value| write::render(value, format, None).len())
        .max()
        .unwrap_or(1)
        .max(1)
}
