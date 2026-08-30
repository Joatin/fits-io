/// One entry of one column of a table.
///
/// A FITS column holds a run of values rather than a single one, so every
/// variant here carries a vector even where the run is one long.
#[derive(Debug, Clone)]
pub enum Value {
    /// `rA`: one string of `r` characters.
    String(String),
    /// `rAw`: `r / w` strings of `w` characters each.
    StringArray(Vec<String>),
    /// `rL`: `r` logicals.
    Boolean(Vec<bool>),
    /// `rX`: `r` bits, packed into `ceil(r / 8)` bytes.
    ///
    /// The bits stay packed because that is how the column stores them; `len`
    /// says how many of them are real, since the last byte is padded. Use
    /// [`Value::bits`] to walk them.
    Bit {
        /// The bytes the bits are packed into.
        bytes: Vec<u8>,
        /// How many bits of them the column actually holds.
        len: usize,
    },
    /// `rB`: `r` unsigned bytes.
    U8(Vec<u8>),
    /// `rS`: `r` signed bytes.
    I8(Vec<i8>),
    /// `rU`: `r` unsigned 16-bit integers.
    U16(Vec<u16>),
    /// `rI`: `r` signed 16-bit integers.
    I16(Vec<i16>),
    /// `rV`: `r` unsigned 32-bit integers.
    U32(Vec<u32>),
    /// `rJ`: `r` signed 32-bit integers.
    I32(Vec<i32>),
    /// `rK`: `r` signed 64-bit integers.
    I64(Vec<i64>),
    /// `rK` with a TZEROn of 2^63: the unsigned reading of a signed column.
    U64(Vec<u64>),
    /// `rE`: `r` single precision floats.
    F32(Vec<f32>),
    /// `rD`: `r` double precision floats.
    F64(Vec<f64>),
    /// `rC`: single precision complex values, as `(real, imaginary)` pairs.
    C32(Vec<(f32, f32)>),
    /// `rM`: double precision complex values, as `(real, imaginary)` pairs.
    M64(Vec<(f64, f64)>),
    /// Every element of this entry is the TNULLn value, so the entry is
    /// undefined. Deserialising it into an `Option<T>` field yields `None`.
    Null,
}

impl Value {
    /// Returns some if the value is a string
    pub fn as_string(&self) -> Option<&String> {
        if let Value::String(s) = self {
            Some(s)
        } else {
            None
        }
    }
    /// The values if this is a double precision column, or `None` if it is not.
    pub fn as_f64(&self) -> Option<&Vec<f64>> {
        if let Value::F64(f) = self {
            Some(f)
        } else {
            None
        }
    }
    /// The values if this is a single precision column, or `None` if it is not.
    pub fn as_f32(&self) -> Option<&Vec<f32>> {
        if let Value::F32(f) = self {
            Some(f)
        } else {
            None
        }
    }
    /// The bits of an `rX` column, most significant first, or `None` for any
    /// other column.
    ///
    /// The padding in the last byte is left out, so this yields exactly the `r`
    /// bits the column declares.
    pub fn bits(&self) -> Option<impl Iterator<Item = bool> + '_> {
        let Value::Bit { bytes, len } = self else {
            return None;
        };

        Some((0..*len).map(move |bit| {
            let byte = bytes.get(bit / 8).copied().unwrap_or(0);

            // Bit 0 is the top of the first byte.
            byte & (1 << (7 - bit % 8)) != 0
        }))
    }

    /// Whether this entry is undefined, per its column's TNULLn card.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// The first value if this is a 64-bit integer column, or `None` if it is not.
    pub fn as_i64(&self) -> Option<i64> {
        if let Value::I64(f) = self {
            f.first().cloned()
        } else {
            None
        }
    }
}
