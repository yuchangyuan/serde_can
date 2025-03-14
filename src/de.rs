use super::error::{Error, Result};
use serde::{Deserialize, de::{self, Visitor, IntoDeserializer}};

#[derive(Default)]
pub struct Deserializer<'de> {
    _phantom: core::marker::PhantomData<&'de ()>,
    input: u64,
    len: isize,

    // buffer for decode bytes
    buf: [u8; 15],
    buf_idx: usize,
}

impl<'de> Deserializer<'de> {
    pub fn from_bytes(bytes: &'de [u8]) -> Self {
        let mut input = 0;
        let mut len = 0;

        for i in bytes {
            input = (input << 8) | *i as u64;
            len += 8;

            if len == 64 { break; }
        }

        if len > 0 { input <<= 64 - len; }

        Deserializer { input, len, ..Default::default() }
    }

    pub fn check_len(&self) -> Result<()> {
        if self.len < 0 { return Err(Error::DeMsgTooLong) }
        Ok(())
    }

    pub fn dec_bool(&mut self) -> Result<bool> {
        let mut res = false;
        if (self.input >> 63) != 0 { res = true; }

        self.len -= 1;
        self.input <<= 1;

        Ok(res)
    }

    pub fn dec_4bit(&mut self) -> Result<usize> {
        let res = (self.input >> 60) as usize;
        self.len -= 4;
        self.input <<= 4;
        Ok(res)
    }

    pub fn dec_u8(&mut self) -> Result<u8> {
        let res = (self.input >> 56) as u8;
        self.len -= 8;
        self.input <<= 8;
        Ok(res)
    }

    pub fn dec_u16(&mut self) -> Result<u16> {
        let res = (self.input >> 48) as u16;
        self.len -= 16;
        self.input <<= 16;
        Ok(res)
    }

    pub fn dec_u32(&mut self) -> Result<u32> {
        let res = (self.input >> 32) as u32;
        self.len -= 32;
        self.input <<= 32;
        Ok(res)
    }

    pub fn dec_u64(&mut self) -> Result<u64> {
        let res = self.input;
        self.len -= 64;
        self.input = 0;
        Ok(res)
    }

    pub fn dec_bytes<'a>(&'a mut self) -> Result<&'a [u8]> {
        let len = self.dec_4bit()?;
        let mut idx = self.buf_idx;

        for _ in 0..len {
            self.buf[idx] = self.dec_u8()?;
            idx += 1;
        }

        let res = &self.buf[self.buf_idx..idx];
        self.buf_idx = idx;

        Ok(res)
    }

    pub fn dec_str<'a>(&'a mut self) -> Result<&'a str> {
        let bytes = self.dec_bytes()?;
        match core::str::from_utf8(bytes) {
            Ok(s) => Ok(s),
            Err(_) => Err(Error::DeUtf8DecodeFail),
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _v: V) -> Result<V::Value> {
        Err(Error::Unsupport("any"))
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value>
    {
        self.check_len()?;
        visitor.visit_bool(self.dec_bool()?)
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value>
    {
        self.check_len()?;
        visitor.visit_i8(self.dec_u8()? as i8)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value>
    {
        self.check_len()?;
        visitor.visit_u8(self.dec_u8()?)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value>
    {
        self.check_len()?;
        visitor.visit_i16(self.dec_u16()? as i16)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value>
    {
        self.check_len()?;
        visitor.visit_u16(self.dec_u16()?)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value>
    {
        self.check_len()?;
        visitor.visit_i32(self.dec_u32()? as i32)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value>
    {
        self.check_len()?;
        visitor.visit_u32(self.dec_u32()?)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value>
    {
        self.check_len()?;
        visitor.visit_i64(self.dec_u64()? as i64)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value>
    {
        self.check_len()?;
        visitor.visit_u64(self.dec_u64()?)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value>
    {
        self.check_len()?;
        visitor.visit_f32(f32::from_bits(self.dec_u32()?))
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value>
    {
        self.check_len()?;
        visitor.visit_f64(f64::from_bits(self.dec_u64()?))
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.check_len()?;

        let s = self.dec_str()?;
        match s.chars().next() {
            Some(c) => visitor.visit_char(c),
            None => Err(Error::DeCharFail),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.check_len()?;
        let s = self.dec_str()?;
        visitor.visit_str(s)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.check_len()?;
        let bytes = self.dec_bytes()?;
        visitor.visit_bytes(bytes)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.check_len()?;
        if self.dec_bool()? { visitor.visit_some(self) }
        else { visitor.visit_none() }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, _: &'static str, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _: &'static str, visitor: V) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.check_len()?;

        let len = self.dec_4bit()?;
        visitor.visit_seq(SeqAccess {de: self, len})
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(SeqAccess {de: self, len})
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, _name: &'static str, len: usize, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(SeqAccess {de: self, len})
    }

    fn deserialize_map<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::Unsupport("map"))
    }

    fn deserialize_struct<V: Visitor<'de>>(self, _name: &'static str,
                                           fields: &'static [&'static str],
                                           visitor: V) -> Result<V::Value> {
        let len = fields.len();
        visitor.visit_seq(SeqAccess {de: self, len})
    }

    fn deserialize_enum<V: Visitor<'de>>(self, _name: &'static str,
                                         _variants: &'static [&'static str],
                                         visitor: V) -> Result<V::Value> {
        let tag = self.dec_4bit()?;
        visitor.visit_enum(Enum { de: self, tag})
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_any(visitor)
    }
}


// ------------------ seq access
struct SeqAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    len: usize,
}

impl<'de, 'a> de::SeqAccess<'de> for SeqAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.len == 0 { return Ok(None) }
        self.len -= 1;
        seed.deserialize(&mut *self.de).map(Some)
    }
}

// ------------------ enum access
struct Enum<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    tag: usize,
}

impl<'de, 'a> de::EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where V: de::DeserializeSeed<'de> {
        let v = seed.deserialize(self.tag.into_deserializer())?;
        Ok((v, self))
    }
}

impl <'de, 'a> de::VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self.de, len, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self.de, fields.len(), visitor)
    }
}

// ------------------ pub api

pub fn from_bytes<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T> {
    let mut de = Deserializer::from_bytes(bytes);
    let res = T::deserialize(&mut de);
    de.check_len()?;
    res
}
