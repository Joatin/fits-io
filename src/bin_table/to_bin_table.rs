use crate::bin_table::{BinTable, Value};
use alloc::string::String;
use alloc::string::ToString;
use core::fmt::{Display, Formatter};
use serde::ser::Impossible;
use serde::{Serialize, ser};
use std::prelude::v1::Vec;
use std::{dbg, vec};

#[derive(Debug, Clone)]
struct Serializer {
    field_names: Vec<String>,
    rows: Vec<Vec<Value>>,
    current_row: Vec<Value>,
}

#[derive(Debug, Clone)]
pub enum Error {
    Unknown,
    NotSupported(&'static str),
}

impl Display for Error {
    fn fmt(&self, _f: &mut Formatter<'_>) -> core::fmt::Result {
        todo!()
    }
}

impl core::error::Error for Error {}
impl ser::Error for Error {
    fn custom<T>(_msg: T) -> Self
    where
        T: Display,
    {
        todo!()
    }
}

pub fn to_bin_table<T: Serialize>(data: &T) -> Result<BinTable, Error> {
    let mut serializer = Serializer {
        field_names: vec![],
        rows: vec![],
        current_row: vec![],
    };

    data.serialize(&mut serializer)?;

    std::dbg!(serializer);
    Ok(BinTable::new(vec![]))
}

impl<'a> ser::Serializer for &'a mut Serializer {
    // The output type produced by this `Serializer` during successful
    // serialization. Most serializers that produce text or binary output should
    // set `Ok = ()` and serialize into an `io::Write` or buffer contained
    // within the `Serializer` instance, as happens here. Serializers that build
    // in-memory data structures may be simplified by using `Ok` to propagate
    // the data structure around.
    type Ok = Value;

    // The error type when some error occurs during serialization.
    type Error = Error;

    // Associated types for keeping track of additional state while serializing
    // compound data structures like sequences and maps. In this case no
    // additional state is required beyond what is already stored in the
    // Serializer struct.
    type SerializeSeq = Self;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = Self;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    // Here we go with the simple methods. The following 12 methods receive one
    // of the primitive types of the data model and map it to JSON by appending
    // into the output string.
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Boolean(vec![v]))
    }

    // JSON does not distinguish between different sizes of integers, so all
    // signed integers will be serialized the same and all unsigned integers
    // will be serialized the same. Other formats, especially compact binary
    // formats, may need independent logic for the different sizes.
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::I8(vec![v]))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::I16(vec![v]))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::I32(vec![v]))
    }

    // Not particularly efficient but this is example code anyway. A more
    // performant approach would be to use the `itoa` crate.
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::I64(vec![v]))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::U8(vec![v]))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::U16(vec![v]))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::U32(vec![v]))
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        Err(Error::NotSupported("u64"))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::F32(vec![v]))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::F64(vec![v]))
    }

    // Serialize a char as a single-character string. Other formats may
    // represent this differently.
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&v.to_string())
    }

    // This only works for strings that don't require escape sequences but you
    // get the idea. For example it would emit invalid JSON if the input string
    // contains a '"' character.
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::String(v.to_string()))
    }

    // Serialize a byte array as an array of bytes. Could also use a base64
    // string here. Binary formats will typically represent byte arrays more
    // compactly.
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Value::U8(v.to_vec()))
    }

    // An absent optional is represented as the JSON `null`.
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(Error::NotSupported("None"))
    }

    // A present optional is represented as just the contained value. Note that
    // this is a lossy representation. For example the values `Some(())` and
    // `None` both serialize as just `null`. Unfortunately this is typically
    // what people expect when working with JSON. Other formats are encouraged
    // to behave more intelligently if possible.
    fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::NotSupported("Some"))
    }

    // In Serde, unit means an anonymous value containing no data. Map this to
    // JSON as `null`.
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(Error::NotSupported("Unit"))
    }

    // Unit struct means a named value containing no data. Again, since there is
    // no data, map this to JSON as `null`. There is no need to serialize the
    // name in most formats.
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    // When serializing a unit variant (or any other kind of variant), formats
    // can choose whether to keep track of it by index or by name. Binary
    // formats typically use the index of the variant and human-readable formats
    // typically use the name.
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain.
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    // Note that newtype variant (and all of the other variant serialization
    // methods) refer exclusively to the "externally tagged" enum
    // representation.
    //
    // Serialize this to JSON in externally tagged form as `{ NAME: VALUE }`.
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        variant.serialize(&mut *self)?;
        value.serialize(&mut *self)?;
        Err(Error::NotSupported("Enum"))
    }

    // Now we get to the serialization of compound types.
    //
    // The start of the sequence, each value, and the end are three separate
    // method calls. This one is responsible only for serializing the start,
    // which in JSON is `[`.
    //
    // The length of the sequence may or may not be known ahead of time. This
    // doesn't make a difference in JSON because the length is not represented
    // explicitly in the serialized form. Some serializers may only be able to
    // support sequences for which the length is known up front.
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(self)
    }

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently by omitting the length, since tuple
    // means that the corresponding `Deserialize implementation will know the
    // length without needing to look at the serialized data.
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(Error::Unknown)
    }

    // Tuple structs look just like sequences in JSON.
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(Error::Unknown)
    }

    // Tuple variants are represented in JSON as `{ NAME: [DATA...] }`. Again
    // this method is only responsible for the externally tagged representation.
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::Unknown)
    }

    // Maps are represented in JSON as `{ K: V, K: V, ... }`.
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(Error::Unknown)
    }

    // Structs look just like maps in JSON. In particular, JSON requires that we
    // serialize the field names of the struct. Other formats may be able to
    // omit the field names when serializing structs because the corresponding
    // Deserialize implementation is required to know what the keys are without
    // looking at the serialized data.
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(self)
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }`.
    // This is the externally tagged representation.
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::Unknown)
    }
}

// The following 7 impls deal with the serialization of compound types like
// sequences and maps. Serialization of such types is begun by a Serializer
// method and followed by zero or more calls to serialize individual elements of
// the compound type and one call to end the compound type.
//
// This impl is SerializeSeq so these methods are called after `serialize_seq`
// is called on the Serializer.
impl<'a> ser::SerializeSeq for &'a mut Serializer {
    // Must match the `Ok` type of the serializer.
    type Ok = Value;
    // Must match the `Error` type of the serializer.
    type Error = Error;

    // Serialize a single element of the sequence.
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)?;
        Ok(())
    }

    // Close the sequence.
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Boolean(vec![false]))
    }
}

// Same thing but for tuples.
// impl<'a> ser::SerializeTuple for &'a mut Serializer {
//     type Ok = ();
//     type Error = Error;
//
//     fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
//     where
//         T: ?Sized + Serialize,
//     {
//         if !self.output.ends_with('[') {
//             self.output += ",";
//         }
//         value.serialize(&mut **self)
//     }
//
//     fn end(self) -> Result<()> {
//         self.output += "]";
//         Ok(())
//     }
// }

// Same thing but for tuple structs.
// impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
//     type Ok = ();
//     type Error = Error;
//
//     fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
//     where
//         T: ?Sized + Serialize,
//     {
//         if !self.output.ends_with('[') {
//             self.output += ",";
//         }
//         value.serialize(&mut **self)
//     }
//
//     fn end(self) -> Result<()> {
//         self.output += "]";
//         Ok(())
//     }
// }

// Tuple variants are a little different. Refer back to the
// `serialize_tuple_variant` method above:
//
//    self.output += "{";
//    variant.serialize(&mut *self)?;
//    self.output += ":[";
//
// So the `end` method in this impl is responsible for closing both the `]` and
// the `}`.
// impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
//     type Ok = ();
//     type Error = Error;
//
//     fn serialize_field<T>(&mut self, value: &T) -> Result<()>
//     where
//         T: ?Sized + Serialize,
//     {
//         if !self.output.ends_with('[') {
//             self.output += ",";
//         }
//         value.serialize(&mut **self)
//     }
//
//     fn end(self) -> Result<()> {
//         self.output += "]}";
//         Ok(())
//     }
// }

// Some `Serialize` types are not able to hold a key and value in memory at the
// same time so `SerializeMap` implementations are required to support
// `serialize_key` and `serialize_value` individually.
//
// There is a third optional method on the `SerializeMap` trait. The
// `serialize_entry` method allows serializers to optimize for the case where
// key and value are both available simultaneously. In JSON it doesn't make a
// difference so the default behavior for `serialize_entry` is fine.
// impl<'a> ser::SerializeMap for &'a mut Serializer {
//     type Ok = ();
//     type Error = Error;
//
//     // The Serde data model allows map keys to be any serializable type. JSON
//     // only allows string keys so the implementation below will produce invalid
//     // JSON if the key serializes as something other than a string.
//     //
//     // A real JSON serializer would need to validate that map keys are strings.
//     // This can be done by using a different Serializer to serialize the key
//     // (instead of `&mut **self`) and having that other serializer only
//     // implement `serialize_str` and return an error on any other data type.
//     fn serialize_key<T>(&mut self, key: &T) -> Result<()>
//     where
//         T: ?Sized + Serialize,
//     {
//         if !self.output.ends_with('{') {
//             self.output += ",";
//         }
//         key.serialize(&mut **self)
//     }
//
//     // It doesn't make a difference whether the colon is printed at the end of
//     // `serialize_key` or at the beginning of `serialize_value`. In this case
//     // the code is a bit simpler having it here.
//     fn serialize_value<T>(&mut self, value: &T) -> Result<()>
//     where
//         T: ?Sized + Serialize,
//     {
//         self.output += ":";
//         value.serialize(&mut **self)
//     }
//
//     fn end(self) -> Result<()> {
//         self.output += "}";
//         Ok(())
//     }
// }

// Structs are like maps in which the keys are constrained to be compile-time
// constant strings.
impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let value = value.serialize(&mut **self)?;
        dbg!(value);
        self.field_names.push(key.to_string());
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::U8(vec![0]))
    }
}

// Similar to `SerializeTupleVariant`, here the `end` method is responsible for
// closing both of the curly braces opened by `serialize_struct_variant`.
// impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
//     type Ok = ();
//     type Error = Error;
//
//     fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
//     where
//         T: ?Sized + Serialize,
//     {
//         if !self.output.ends_with('{') {
//             self.output += ",";
//         }
//         key.serialize(&mut **self)?;
//         self.output += ":";
//         value.serialize(&mut **self)
//     }
//
//     fn end(self) -> Result<()> {
//         self.output += "}}";
//         Ok(())
//     }
// }
