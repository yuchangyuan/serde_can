use serde::{ser, Serialize};

use crate::error::{Error, Result};
use heapless::Vec;

#[derive(Debug)]
pub struct Serializer {
    output: u64,
    len: usize,
}

pub fn to_bytes<T>(value: &T) -> Result<Vec<u8, 8>>
where
    T: Serialize,
{
    let mut serializer = Serializer {
        output: 0,
        len: 0,
    };

    value.serialize(&mut serializer)?;

    let mut out = serializer.output;
    if serializer.len > 0 {
        out = out << (64 - serializer.len);
    }

    let mut res: Vec<u8, 8> = Vec::new();

    for _ in 0..((serializer.len + 7) / 8) {
        // we're sure that len <= 64
        res.push((out >> 56) as u8).unwrap();
        out <<= 8;
    }

    Ok(res)
}

impl Serializer {
    fn check_len(&self) -> Result<()> {
        if self.len > 64 { return Err(Error::SerMsgTooLong); }
        Ok(())
    }

    // NOTE: all enc_* not check len
    fn enc_bool(&mut self, v: bool) -> Result<()> {
        self.output <<= 1;
        self.len += 1;
        if v { self.output |= 1 };

        Ok(())
    }

    fn enc_4bit(&mut self, n: usize, err: Error) -> Result<()> {
        if n >= 16 { return Err(err) }

        self.output <<= 4;
        self.output |= n as u64;
        self.len += 4;

        Ok(())
    }

    fn enc_u8(&mut self, v: u8) -> Result<()> {
        self.output <<= 8;
        self.output |= v as u64;
        self.len += 8;

        Ok(())
    }

    fn enc_bytes(&mut self, v: &[u8], err: Error) -> Result<()> {
        self.enc_4bit(v.len(), err)?;
        for b in v {
            self.enc_u8(*b)?
        }

        Ok(())
    }

    fn enc_tagged_union<T>(&mut self, idx: u32, value: &T, name: &'static str, variant: &'static str) -> Result<()> where
        T: ?Sized + Serialize
    {
        self.enc_4bit(idx as usize, Error::SerFieldIndexTooLarge(name, variant))?;
        value.serialize(self)
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;


    fn collect_str<T>(self, _value: &T) -> Result<()>
    where
        T: ?Sized + core::fmt::Display {
        Err(Error::Unsupport("Display"))
    }

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.enc_bool(v)?;
        self.check_len()
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.enc_u8(v)?;
        self.check_len()
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.output <<= 16;
        self.output |= v as u64;
        self.len += 16;
        self.check_len()
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.output <<= 32;
        self.output |= v as u64;
        self.len += 32;
        self.check_len()
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.output = v;
        self.len += 64;
        self.check_len()
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_u8(v as u8)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_u16(v as u16)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_u32(v as u32)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_u32(v.to_bits())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.serialize_u64(v.to_bits())
    }

    // -------------------- char/string/bytes as bytes
    fn serialize_char(self, v: char) -> Result<()> {
        let (len, buf) = {
            let mut buf = [0u8; 4];
            let res = v.encode_utf8(&mut buf);
            (res.len(), buf)
        };

        self.enc_bytes(&buf[0..len], Error::SerLengthTooLarge("char", len))?;
        self.check_len()
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        let bytes = v.as_bytes();
        self.enc_bytes(bytes, Error::SerLengthTooLarge("string", bytes.len()))?;
        self.check_len()
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.enc_bytes(v, Error::SerLengthTooLarge("bytes", v.len()))?;
        self.check_len()
    }

    // ---------------- option
    // none, 1bit 0
    fn serialize_none(self) -> Result<()> {
        self.enc_bool(false)?;
        self.check_len()
    }

    // some(v), 1bit 1 follow v
    fn serialize_some<T>(self, value: &T) -> Result<()> where
        T: ?Sized + Serialize
    {
        self.enc_bool(true)?;
        value.serialize(self)
    }

    // ---------------- unit
    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    // tagged union
    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.enc_tagged_union(variant_index, &(), name, variant)?;
        self.check_len()
    }

    // -------------- new type
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    // tagged union
    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.enc_tagged_union(variant_index, value, name, variant)?;
        self.check_len()
    }

    // seq, first 4bit len, then elements
    fn serialize_seq(self, len_opt: Option<usize>) -> Result<Self::SerializeSeq> {
        let Some(len) = len_opt else {
            return Err(Error::SerLengthUnknow);
        };

        self.enc_4bit(len, Error::SerLengthTooLarge("seq", len))?;
        Ok(self)
    }

    // all elements, left to right
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(self)
    }

    // same as tuple
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(self)
    }

    // tagged union
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.enc_tagged_union(variant_index, &(), name, variant)?;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::Unsupport("map"))
    }

    // same as tuple
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        Ok(self)
    }

    // tagged union
    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.enc_tagged_union(variant_index, &(), name, variant)?;
        Ok(self)
    }
}

// seq, element by element
impl<'a> ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where T: ?Sized + Serialize
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> { self.check_len() }
}

// same as seq
impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> { self.check_len() }
}

// same as tuple
impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> { self.check_len() }
}


impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> { self.check_len() }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, _key: &T) -> Result<()>
    where T: ?Sized + Serialize,
    {
        Err(Error::Unsupport("map"))
    }

    fn serialize_value<T>(&mut self, _value: &T) -> Result<()>
    where T: ?Sized + Serialize,
    {
        Err(Error::Unsupport("map"))
    }

    fn end(self) -> Result<()> {
        Err(Error::Unsupport("map"))
    }
}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> { self.check_len() }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> { self.check_len() }
}
