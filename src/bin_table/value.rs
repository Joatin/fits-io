use alloc::string::String;
use alloc::vec::Vec;
#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    StringArray(Vec<String>),
    Boolean(Vec<bool>),
    Bit(Vec<u8>),
    U8(Vec<u8>),
    I8(Vec<i8>),
    U16(Vec<u16>),
    I16(Vec<i16>),
    U32(Vec<u32>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    C32(Vec<i32>),
    M64(Vec<i64>),
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
    pub fn as_f64(&self) -> Option<&Vec<f64>> {
        if let Value::F64(f) = self {
            Some(f)
        } else {
            None
        }
    }
    pub fn as_f32(&self) -> Option<&Vec<f32>> {
        if let Value::F32(f) = self {
            Some(f)
        } else {
            None
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        if let Value::I64(f) = self {
            f.first().cloned()
        } else {
            None
        }
    }
}
