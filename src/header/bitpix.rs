use alloc::boxed::Box;
use alloc::format;
use core::error::Error;

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
