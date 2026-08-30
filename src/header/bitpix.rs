use std::error::Error;

/// The type of the values in an array, from its BITPIX card.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bitpix {
    /// Float 64
    F64 = -64,

    /// Float 32
    F32 = -32,

    /// Unsigned 8 bit
    U8 = 8,

    /// Signed 16 bit
    I16 = 16,

    /// Signed 32 bit
    I32 = 32,
}

impl Bitpix {
    /// How many bytes one value of this type occupies.
    pub fn byte_size(&self) -> usize {
        match self {
            Bitpix::F64 => 8,
            Bitpix::F32 => 4,
            Bitpix::U8 => 1,
            Bitpix::I16 => 2,
            Bitpix::I32 => 4,
        }
    }
}

impl Bitpix {
    /// Decodes one big-endian array value from the front of `bytes`, widening it
    /// to `f64`.
    ///
    /// Returns `None` when `bytes` is shorter than [`Bitpix::byte_size`], so a
    /// truncated data section produces a short read rather than a panic.
    pub fn read_be(&self, bytes: &[u8]) -> Option<f64> {
        fn take<const N: usize>(bytes: &[u8]) -> Option<[u8; N]> {
            bytes.get(..N)?.try_into().ok()
        }

        match self {
            Bitpix::F64 => Some(f64::from_be_bytes(take(bytes)?)),
            Bitpix::F32 => Some(f32::from_be_bytes(take(bytes)?) as f64),
            Bitpix::U8 => Some(*bytes.first()? as f64),
            Bitpix::I16 => Some(i16::from_be_bytes(take(bytes)?) as f64),
            Bitpix::I32 => Some(i32::from_be_bytes(take(bytes)?) as f64),
        }
    }

    /// The inclusive range of values this type can represent, as `f64`.
    ///
    /// `None` for the floating point types, which have no meaningful full-scale
    /// range to normalise against.
    pub fn value_range(&self) -> Option<(f64, f64)> {
        match self {
            Bitpix::F64 | Bitpix::F32 => None,
            Bitpix::U8 => Some((u8::MIN as f64, u8::MAX as f64)),
            Bitpix::I16 => Some((i16::MIN as f64, i16::MAX as f64)),
            Bitpix::I32 => Some((i32::MIN as f64, i32::MAX as f64)),
        }
    }
}

impl From<Bitpix> for i64 {
    fn from(value: Bitpix) -> Self {
        value as i64
    }
}

impl TryFrom<i64> for Bitpix {
    type Error = Box<dyn Error + Send + Sync>;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            -64 => Ok(Bitpix::F64),
            -32 => Ok(Bitpix::F32),
            8 => Ok(Bitpix::U8),
            16 => Ok(Bitpix::I16),
            32 => Ok(Bitpix::I32),
            _ => Err(From::from(format!("Invalid bitpix value: {}", value))),
        }
    }
}
