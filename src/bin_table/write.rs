use crate::bin_table::{BinTable, FieldDefinition};
use crate::header::card::Card;
use crate::header::{ExtensionType, Header, TableNullValue};

/// Rewrites `header` to describe `table`.
///
/// A binary table's header does not merely accompany its data, it is the only
/// thing that says how to read it: how wide a row is, how many there are, and
/// what each column holds. Writing the data without bringing these cards along
/// produces bytes nobody can decode.
pub(crate) fn apply_to_header(table: &BinTable, header: &mut Header) {
    // Columns that the new table does not have would otherwise be left behind,
    // describing fields that are no longer there.
    clear_column_cards(header);

    header.set(Card::Xtension {
        value: ExtensionType::BinTable,
        comment: None,
    });
    header.set(Card::Bitpix {
        value: crate::header::Bitpix::U8,
        comment: None,
    });
    header.set(Card::NAxis {
        value: 2,
        comment: None,
    });
    header.set(Card::NAxisN {
        index: 0,
        value: table.bytes_per_row() as i64,
        comment: Some("width of a row in bytes".into()),
    });
    header.set(Card::NAxisN {
        index: 1,
        value: table.len() as i64,
        comment: Some("number of rows".into()),
    });
    // PCOUNT is the size of the heap, which is where variable length array
    // columns keep their values.
    header.set(Card::ParameterCount {
        value: table.heap_len() as i64,
        comment: None,
    });
    header.set(Card::GroupCount {
        value: 1,
        comment: None,
    });
    header.set(Card::TableFields {
        value: table.field_definitions().len() as i64,
        comment: None,
    });

    for (index, field) in table.field_definitions().iter().enumerate() {
        apply_column(header, index, field);
    }
}

fn apply_column(header: &mut Header, index: usize, field: &FieldDefinition) {
    header.set(Card::TableFormatN {
        index,
        value: String::from(field.format),
        comment: None,
    });

    // TTYPEn is optional, and a column that carries no name should not gain an
    // empty one.
    if !field.name.is_empty() {
        header.set(Card::TableTypeN {
            index,
            value: field.name.clone(),
            comment: None,
        });
    }

    if let Some(scale) = field.scale {
        header.set(Card::TableScalingFactorN {
            index,
            value: scale,
            comment: None,
        });
    }
    if let Some(zero) = field.zero {
        header.set(Card::TableScalingZeroPointN {
            index,
            value: zero,
            comment: None,
        });
    }
    if let Some(null) = field.null {
        header.set(Card::TableNullValueN {
            index,
            value: TableNullValue::Integer(null),
            comment: Some("value marking an undefined entry".into()),
        });
    }
    if !field.dimensions.is_empty() {
        let shape: Vec<String> = field.dimensions.iter().map(usize::to_string).collect();
        header.set(Card::TableDimensionsN {
            index,
            value: format!("({})", shape.join(",")),
            comment: None,
        });
    }
}

/// Removes every per-column card, so that the ones written next are the only
/// ones the header carries.
pub(crate) fn clear_column_cards(header: &mut Header) {
    use crate::header::card_keys;

    for prefix in [
        card_keys::PREFIX_TFORM_N,
        card_keys::PREFIX_TTYPE_N,
        card_keys::PREFIX_TSCAL_N,
        card_keys::PREFIX_TZERO_N,
        card_keys::PREFIX_TNULL_N,
        card_keys::PREFIX_TDIM_N,
        card_keys::PREFIX_TUNIT_N,
        card_keys::PREFIX_TDISP_N,
        card_keys::PREFIX_TBCOL_N,
    ] {
        header.remove_prefixed(prefix);
    }
}
