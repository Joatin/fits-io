use std::fmt;

/// The value a table column uses to mark an undefined entry, from its TNULLn card.
///
/// The two table kinds spell this differently: a binary table gives an integer
/// that matching raw entries are compared against, while an ASCII table gives
/// the literal character string that fills the field. Both are accepted here so
/// that a header carrying either still parses.
#[derive(Debug, Clone, PartialEq)]
pub enum TableNullValue {
    /// `TNULLn = -32768`, as binary tables write it.
    Integer(i64),
    /// `TNULLn = '   '`, as ASCII tables write it.
    Text(String),
}

impl TableNullValue {
    /// The integer form, or `None` for an ASCII table's string form.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            TableNullValue::Integer(value) => Some(*value),
            TableNullValue::Text(_) => None,
        }
    }

    /// The string form, or `None` for a binary table's integer form.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            TableNullValue::Text(value) => Some(value.as_str()),
            TableNullValue::Integer(_) => None,
        }
    }
}

impl fmt::Display for TableNullValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TableNullValue::Integer(value) => write!(f, "{}", value),
            TableNullValue::Text(value) => write!(f, "{}", value),
        }
    }
}
