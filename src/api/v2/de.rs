// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the
// "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS
// BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
// ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! A module providing a visitor and deserializer for tagged data. Code
//! is taken from `serde`.

use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::marker::PhantomData;

use serde::de::value::MapDeserializer;
use serde::de::value::SeqAccessDeserializer;
use serde::de::value::SeqDeserializer;
use serde::de::DeserializeSeed;
use serde::de::EnumAccess;
use serde::de::Error;
use serde::de::Expected;
use serde::de::IntoDeserializer;
use serde::de::MapAccess;
use serde::de::SeqAccess;
use serde::de::Unexpected;
use serde::de::VariantAccess;
use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;

#[derive(Debug)]
pub enum Content<'de> {
  Bool(bool),

  U8(u8),
  U16(u16),
  U32(u32),
  U64(u64),

  I8(i8),
  I16(i16),
  I32(i32),
  I64(i64),

  F32(f32),
  F64(f64),

  Char(char),
  String(String),
  Str(&'de str),
  ByteBuf(Vec<u8>),
  Bytes(&'de [u8]),

  None,
  Some(Box<Content<'de>>),

  Unit,
  Newtype(Box<Content<'de>>),
  Seq(Vec<Content<'de>>),
  Map(Vec<(Content<'de>, Content<'de>)>),
}

impl<'de> Content<'de> {
  #[cold]
  fn unexpected(&self) -> Unexpected<'_> {
    match *self {
      Content::Bool(b) => Unexpected::Bool(b),
      Content::U8(n) => Unexpected::Unsigned(n as u64),
      Content::U16(n) => Unexpected::Unsigned(n as u64),
      Content::U32(n) => Unexpected::Unsigned(n as u64),
      Content::U64(n) => Unexpected::Unsigned(n),
      Content::I8(n) => Unexpected::Signed(n as i64),
      Content::I16(n) => Unexpected::Signed(n as i64),
      Content::I32(n) => Unexpected::Signed(n as i64),
      Content::I64(n) => Unexpected::Signed(n),
      Content::F32(f) => Unexpected::Float(f as f64),
      Content::F64(f) => Unexpected::Float(f),
      Content::Char(c) => Unexpected::Char(c),
      Content::String(ref s) => Unexpected::Str(s),
      Content::Str(s) => Unexpected::Str(s),
      Content::ByteBuf(ref b) => Unexpected::Bytes(b),
      Content::Bytes(b) => Unexpected::Bytes(b),
      Content::None | Content::Some(_) => Unexpected::Option,
      Content::Unit => Unexpected::Unit,
      Content::Newtype(_) => Unexpected::NewtypeStruct,
      Content::Seq(_) => Unexpected::Seq,
      Content::Map(_) => Unexpected::Map,
    }
  }
}

impl<'de> Deserialize<'de> for Content<'de> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let visitor = ContentVisitor { value: PhantomData };
    deserializer.deserialize_any(visitor)
  }
}

struct ContentVisitor<'de> {
  value: PhantomData<Content<'de>>,
}

impl<'de> ContentVisitor<'de> {
  fn new() -> Self {
    ContentVisitor { value: PhantomData }
  }
}

impl<'de> Visitor<'de> for ContentVisitor<'de> {
  type Value = Content<'de>;

  fn expecting(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    fmt.write_str("any value")
  }

  fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::Bool(value))
  }

  fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::I8(value))
  }

  fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::I16(value))
  }

  fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::I32(value))
  }

  fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::I64(value))
  }

  fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::U8(value))
  }

  fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::U16(value))
  }

  fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::U32(value))
  }

  fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::U64(value))
  }

  fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::F32(value))
  }

  fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::F64(value))
  }

  fn visit_char<E>(self, value: char) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::Char(value))
  }

  fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::String(value.into()))
  }

  fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::Str(value))
  }

  fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::String(value))
  }

  fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::ByteBuf(value.into()))
  }

  fn visit_borrowed_bytes<E>(self, value: &'de [u8]) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::Bytes(value))
  }

  fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::ByteBuf(value))
  }

  fn visit_unit<E>(self) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::Unit)
  }

  fn visit_none<E>(self) -> Result<Self::Value, E>
  where
    E: Error,
  {
    Ok(Content::None)
  }

  fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
  where
    D: Deserializer<'de>,
  {
    Deserialize::deserialize(deserializer).map(|v| Content::Some(Box::new(v)))
  }

  fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
  where
    D: Deserializer<'de>,
  {
    Deserialize::deserialize(deserializer).map(|v| Content::Newtype(Box::new(v)))
  }

  fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
  where
    V: SeqAccess<'de>,
  {
    let mut vec = Vec::with_capacity(visitor.size_hint().unwrap_or(1024));
    while let Some(e) = visitor.next_element()? {
      vec.push(e);
    }
    Ok(Content::Seq(vec))
  }

  fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
  where
    V: MapAccess<'de>,
  {
    let mut vec = Vec::with_capacity(visitor.size_hint().unwrap_or(1024));
    while let Some(kv) = visitor.next_entry()? {
      vec.push(kv);
    }
    Ok(Content::Map(vec))
  }

  fn visit_enum<V>(self, _visitor: V) -> Result<Self::Value, V::Error>
  where
    V: EnumAccess<'de>,
  {
    Err(Error::custom(
      "untagged and internally tagged enums do not support enum input",
    ))
  }
}

struct VariantDeserializer<'de, E>
where
  E: Error,
{
  value: Option<Content<'de>>,
  err: PhantomData<E>,
}

impl<'de, E> VariantAccess<'de> for VariantDeserializer<'de, E>
where
  E: Error,
{
  type Error = E;

  fn unit_variant(self) -> Result<(), E> {
    match self.value {
      Some(value) => Deserialize::deserialize(ContentDeserializer::new(value)),
      None => Ok(()),
    }
  }

  fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, E>
  where
    T: DeserializeSeed<'de>,
  {
    match self.value {
      Some(value) => seed.deserialize(ContentDeserializer::new(value)),
      None => Err(Error::invalid_type(
        Unexpected::UnitVariant,
        &"newtype variant",
      )),
    }
  }

  fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }

  fn struct_variant<V>(
    self,
    _fields: &'static [&'static str],
    _visitor: V,
  ) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    unimplemented!()
  }
}


struct EnumDeserializer<'de, E>
where
  E: Error,
{
  variant: Content<'de>,
  value: Option<Content<'de>>,
  err: PhantomData<E>,
}

impl<'de, E> EnumDeserializer<'de, E>
where
  E: Error,
{
  fn new(variant: Content<'de>, value: Option<Content<'de>>) -> EnumDeserializer<'de, E> {
    EnumDeserializer {
      variant,
      value,
      err: PhantomData,
    }
  }
}

impl<'de, E> EnumAccess<'de> for EnumDeserializer<'de, E>
where
  E: Error,
{
  type Error = E;
  type Variant = VariantDeserializer<'de, Self::Error>;

  fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), E>
  where
    V: DeserializeSeed<'de>,
  {
    let visitor = VariantDeserializer {
      value: self.value,
      err: PhantomData,
    };
    seed
      .deserialize(ContentDeserializer::new(self.variant))
      .map(|v| (v, visitor))
  }
}

fn visit_content_seq<'de, V, E>(content: Vec<Content<'de>>, visitor: V) -> Result<V::Value, E>
where
  V: Visitor<'de>,
  E: Error,
{
  let seq = content.into_iter().map(ContentDeserializer::new);
  let mut seq_visitor = SeqDeserializer::new(seq);
  let value = visitor.visit_seq(&mut seq_visitor)?;
  seq_visitor.end()?;
  Ok(value)
}

fn visit_content_map<'de, V, E>(
  content: Vec<(Content<'de>, Content<'de>)>,
  visitor: V,
) -> Result<V::Value, E>
where
  V: Visitor<'de>,
  E: Error,
{
  let map = content
    .into_iter()
    .map(|(k, v)| (ContentDeserializer::new(k), ContentDeserializer::new(v)));
  let mut map_visitor = MapDeserializer::new(map);
  let value = visitor.visit_map(&mut map_visitor)?;
  map_visitor.end()?;
  Ok(value)
}

pub struct ContentDeserializer<'de, E> {
  content: Content<'de>,
  err: PhantomData<E>,
}

impl<'de, E> ContentDeserializer<'de, E>
where
  E: Error,
{
  pub fn new(content: Content<'de>) -> Self {
    ContentDeserializer {
      content,
      err: PhantomData,
    }
  }

  #[cold]
  fn invalid_type(self, exp: &dyn Expected) -> E {
    Error::invalid_type(self.content.unexpected(), exp)
  }

  fn deserialize_integer<V>(self, visitor: V) -> Result<V::Value, E>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::U8(v) => visitor.visit_u8(v),
      Content::U16(v) => visitor.visit_u16(v),
      Content::U32(v) => visitor.visit_u32(v),
      Content::U64(v) => visitor.visit_u64(v),
      Content::I8(v) => visitor.visit_i8(v),
      Content::I16(v) => visitor.visit_i16(v),
      Content::I32(v) => visitor.visit_i32(v),
      Content::I64(v) => visitor.visit_i64(v),
      _ => Err(self.invalid_type(&visitor)),
    }
  }
}

impl<'de, E> Deserializer<'de> for ContentDeserializer<'de, E>
where
  E: Error,
{
  type Error = E;

  fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::Bool(v) => visitor.visit_bool(v),
      Content::U8(v) => visitor.visit_u8(v),
      Content::U16(v) => visitor.visit_u16(v),
      Content::U32(v) => visitor.visit_u32(v),
      Content::U64(v) => visitor.visit_u64(v),
      Content::I8(v) => visitor.visit_i8(v),
      Content::I16(v) => visitor.visit_i16(v),
      Content::I32(v) => visitor.visit_i32(v),
      Content::I64(v) => visitor.visit_i64(v),
      Content::F32(v) => visitor.visit_f32(v),
      Content::F64(v) => visitor.visit_f64(v),
      Content::Char(v) => visitor.visit_char(v),
      Content::String(v) => visitor.visit_string(v),
      Content::Str(v) => visitor.visit_borrowed_str(v),
      Content::ByteBuf(v) => visitor.visit_byte_buf(v),
      Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
      Content::Unit => visitor.visit_unit(),
      Content::None => visitor.visit_none(),
      Content::Some(v) => visitor.visit_some(ContentDeserializer::new(*v)),
      Content::Newtype(v) => visitor.visit_newtype_struct(ContentDeserializer::new(*v)),
      Content::Seq(v) => visit_content_seq(v, visitor),
      Content::Map(v) => visit_content_map(v, visitor),
    }
  }

  fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::Bool(v) => visitor.visit_bool(v),
      _ => Err(self.invalid_type(&visitor)),
    }
  }

  fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    self.deserialize_integer(visitor)
  }

  fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    self.deserialize_integer(visitor)
  }

  fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    self.deserialize_integer(visitor)
  }

  fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    self.deserialize_integer(visitor)
  }

  fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    self.deserialize_integer(visitor)
  }

  fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    self.deserialize_integer(visitor)
  }

  fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    self.deserialize_integer(visitor)
  }

  fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    self.deserialize_integer(visitor)
  }

  fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::F32(v) => visitor.visit_f32(v),
      Content::F64(v) => visitor.visit_f64(v),
      Content::U64(v) => visitor.visit_u64(v),
      Content::I64(v) => visitor.visit_i64(v),
      _ => Err(self.invalid_type(&visitor)),
    }
  }

  fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::F64(v) => visitor.visit_f64(v),
      Content::U64(v) => visitor.visit_u64(v),
      Content::I64(v) => visitor.visit_i64(v),
      _ => Err(self.invalid_type(&visitor)),
    }
  }

  fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::Char(v) => visitor.visit_char(v),
      Content::String(v) => visitor.visit_string(v),
      Content::Str(v) => visitor.visit_borrowed_str(v),
      _ => Err(self.invalid_type(&visitor)),
    }
  }

  fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    self.deserialize_string(visitor)
  }

  fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::String(v) => visitor.visit_string(v),
      Content::Str(v) => visitor.visit_borrowed_str(v),
      Content::ByteBuf(v) => visitor.visit_byte_buf(v),
      Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
      _ => Err(self.invalid_type(&visitor)),
    }
  }

  fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    self.deserialize_byte_buf(visitor)
  }

  fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::String(v) => visitor.visit_string(v),
      Content::Str(v) => visitor.visit_borrowed_str(v),
      Content::ByteBuf(v) => visitor.visit_byte_buf(v),
      Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
      Content::Seq(v) => visit_content_seq(v, visitor),
      _ => Err(self.invalid_type(&visitor)),
    }
  }

  fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::None => visitor.visit_none(),
      Content::Some(v) => visitor.visit_some(ContentDeserializer::new(*v)),
      Content::Unit => visitor.visit_unit(),
      _ => visitor.visit_some(self),
    }
  }

  fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::Unit => visitor.visit_unit(),
      _ => Err(self.invalid_type(&visitor)),
    }
  }

  fn deserialize_unit_struct<V>(
    self,
    _name: &'static str,
    visitor: V,
  ) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::Map(ref v) if v.is_empty() => visitor.visit_unit(),
      _ => self.deserialize_any(visitor),
    }
  }

  fn deserialize_newtype_struct<V>(self, _name: &str, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::Newtype(v) => visitor.visit_newtype_struct(ContentDeserializer::new(*v)),
      _ => visitor.visit_newtype_struct(self),
    }
  }

  fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::Seq(v) => visit_content_seq(v, visitor),
      _ => Err(self.invalid_type(&visitor)),
    }
  }

  fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    self.deserialize_seq(visitor)
  }

  fn deserialize_tuple_struct<V>(
    self,
    _name: &'static str,
    _len: usize,
    visitor: V,
  ) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    self.deserialize_seq(visitor)
  }

  fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::Map(v) => visit_content_map(v, visitor),
      _ => Err(self.invalid_type(&visitor)),
    }
  }

  fn deserialize_struct<V>(
    self,
    _name: &'static str,
    _fields: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::Seq(v) => visit_content_seq(v, visitor),
      Content::Map(v) => visit_content_map(v, visitor),
      _ => Err(self.invalid_type(&visitor)),
    }
  }

  fn deserialize_enum<V>(
    self,
    _name: &str,
    _variants: &'static [&'static str],
    visitor: V,
  ) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    let (variant, value) = match self.content {
      Content::Map(value) => {
        let mut iter = value.into_iter();
        let (variant, value) = match iter.next() {
          Some(v) => v,
          None => {
            return Err(Error::invalid_value(
              Unexpected::Map,
              &"map with a single key",
            ))
          },
        };
        if iter.next().is_some() {
          return Err(Error::invalid_value(
            Unexpected::Map,
            &"map with a single key",
          ))
        }
        (variant, Some(value))
      },
      s @ Content::String(_) | s @ Content::Str(_) => (s, None),
      other => return Err(Error::invalid_type(other.unexpected(), &"string or map")),
    };

    visitor.visit_enum(EnumDeserializer::new(variant, value))
  }

  fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    match self.content {
      Content::String(v) => visitor.visit_string(v),
      Content::Str(v) => visitor.visit_borrowed_str(v),
      Content::ByteBuf(v) => visitor.visit_byte_buf(v),
      Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
      Content::U8(v) => visitor.visit_u8(v),
      Content::U64(v) => visitor.visit_u64(v),
      _ => Err(self.invalid_type(&visitor)),
    }
  }

  fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
  where
    V: Visitor<'de>,
  {
    drop(self);
    visitor.visit_unit()
  }
}

impl<'de, E> IntoDeserializer<'de, E> for ContentDeserializer<'de, E>
where
  E: Error,
{
  type Deserializer = Self;

  fn into_deserializer(self) -> Self {
    self
  }
}


pub struct TaggedContent<'de, T> {
  pub tag: T,
  pub content: Content<'de>,
}

pub struct TaggedContentVisitor<'de, T> {
  tag_name: &'static str,
  value: PhantomData<TaggedContent<'de, T>>,
}

impl<'de, T> TaggedContentVisitor<'de, T> {
  pub fn new(name: &'static str) -> Self {
    TaggedContentVisitor {
      tag_name: name,
      value: PhantomData,
    }
  }
}

impl<'de, T> DeserializeSeed<'de> for TaggedContentVisitor<'de, T>
where
  T: Deserialize<'de>,
{
  type Value = TaggedContent<'de, T>;

  fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_any(self)
  }
}

impl<'de, T> Visitor<'de> for TaggedContentVisitor<'de, T>
where
  T: Deserialize<'de>,
{
  type Value = TaggedContent<'de, T>;

  fn expecting(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    fmt.write_str("internally tagged enum")
  }

  fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
  where
    S: SeqAccess<'de>,
  {
    let tag = match seq.next_element()? {
      Some(tag) => tag,
      None => return Err(Error::missing_field(self.tag_name)),
    };
    let rest = SeqAccessDeserializer::new(seq);
    Ok(TaggedContent {
      tag,
      content: Content::deserialize(rest)?,
    })
  }

  fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
  where
    M: MapAccess<'de>,
  {
    let mut tag = None;
    let mut vec = Vec::with_capacity(map.size_hint().unwrap_or(1024));
    while let Some(k) = map.next_key_seed(TagOrContentVisitor::new(self.tag_name))? {
      match k {
        TagOrContent::Tag => {
          if tag.is_some() {
            return Err(Error::duplicate_field(self.tag_name))
          }
          tag = Some(map.next_value()?);
        },
        TagOrContent::Content(k) => {
          let v = map.next_value()?;
          vec.push((k, v));
        },
      }
    }
    match tag {
      None => Err(Error::missing_field(self.tag_name)),
      Some(tag) => Ok(TaggedContent {
        tag,
        content: Content::Map(vec),
      }),
    }
  }
}

enum TagOrContent<'de> {
  Tag,
  Content(Content<'de>),
}

struct TagOrContentVisitor<'de> {
  name: &'static str,
  value: PhantomData<TagOrContent<'de>>,
}

impl<'de> TagOrContentVisitor<'de> {
  fn new(name: &'static str) -> Self {
    TagOrContentVisitor {
      name,
      value: PhantomData,
    }
  }
}

impl<'de> DeserializeSeed<'de> for TagOrContentVisitor<'de> {
  type Value = TagOrContent<'de>;

  fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_any(self)
  }
}

impl<'de> Visitor<'de> for TagOrContentVisitor<'de> {
  type Value = TagOrContent<'de>;

  fn expecting(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    write!(fmt, "a type tag `{}` or any other value", self.name)
  }

  fn visit_bool<F>(self, value: bool) -> Result<Self::Value, F>
  where
    F: Error,
  {
    ContentVisitor::new()
      .visit_bool(value)
      .map(TagOrContent::Content)
  }

  fn visit_i8<F>(self, value: i8) -> Result<Self::Value, F>
  where
    F: Error,
  {
    ContentVisitor::new()
      .visit_i8(value)
      .map(TagOrContent::Content)
  }

  fn visit_i16<F>(self, value: i16) -> Result<Self::Value, F>
  where
    F: Error,
  {
    ContentVisitor::new()
      .visit_i16(value)
      .map(TagOrContent::Content)
  }

  fn visit_i32<F>(self, value: i32) -> Result<Self::Value, F>
  where
    F: Error,
  {
    ContentVisitor::new()
      .visit_i32(value)
      .map(TagOrContent::Content)
  }

  fn visit_i64<F>(self, value: i64) -> Result<Self::Value, F>
  where
    F: Error,
  {
    ContentVisitor::new()
      .visit_i64(value)
      .map(TagOrContent::Content)
  }

  fn visit_u8<F>(self, value: u8) -> Result<Self::Value, F>
  where
    F: Error,
  {
    ContentVisitor::new()
      .visit_u8(value)
      .map(TagOrContent::Content)
  }

  fn visit_u16<F>(self, value: u16) -> Result<Self::Value, F>
  where
    F: Error,
  {
    ContentVisitor::new()
      .visit_u16(value)
      .map(TagOrContent::Content)
  }

  fn visit_u32<F>(self, value: u32) -> Result<Self::Value, F>
  where
    F: Error,
  {
    ContentVisitor::new()
      .visit_u32(value)
      .map(TagOrContent::Content)
  }

  fn visit_u64<F>(self, value: u64) -> Result<Self::Value, F>
  where
    F: Error,
  {
    ContentVisitor::new()
      .visit_u64(value)
      .map(TagOrContent::Content)
  }

  fn visit_f32<F>(self, value: f32) -> Result<Self::Value, F>
  where
    F: Error,
  {
    ContentVisitor::new()
      .visit_f32(value)
      .map(TagOrContent::Content)
  }

  fn visit_f64<F>(self, value: f64) -> Result<Self::Value, F>
  where
    F: Error,
  {
    ContentVisitor::new()
      .visit_f64(value)
      .map(TagOrContent::Content)
  }

  fn visit_char<F>(self, value: char) -> Result<Self::Value, F>
  where
    F: Error,
  {
    ContentVisitor::new()
      .visit_char(value)
      .map(TagOrContent::Content)
  }

  fn visit_str<F>(self, value: &str) -> Result<Self::Value, F>
  where
    F: Error,
  {
    if value == self.name {
      Ok(TagOrContent::Tag)
    } else {
      ContentVisitor::new()
        .visit_str(value)
        .map(TagOrContent::Content)
    }
  }

  fn visit_borrowed_str<F>(self, value: &'de str) -> Result<Self::Value, F>
  where
    F: Error,
  {
    if value == self.name {
      Ok(TagOrContent::Tag)
    } else {
      ContentVisitor::new()
        .visit_borrowed_str(value)
        .map(TagOrContent::Content)
    }
  }

  fn visit_string<F>(self, value: String) -> Result<Self::Value, F>
  where
    F: Error,
  {
    if value == self.name {
      Ok(TagOrContent::Tag)
    } else {
      ContentVisitor::new()
        .visit_string(value)
        .map(TagOrContent::Content)
    }
  }

  fn visit_bytes<F>(self, value: &[u8]) -> Result<Self::Value, F>
  where
    F: Error,
  {
    if value == self.name.as_bytes() {
      Ok(TagOrContent::Tag)
    } else {
      ContentVisitor::new()
        .visit_bytes(value)
        .map(TagOrContent::Content)
    }
  }

  fn visit_borrowed_bytes<F>(self, value: &'de [u8]) -> Result<Self::Value, F>
  where
    F: Error,
  {
    if value == self.name.as_bytes() {
      Ok(TagOrContent::Tag)
    } else {
      ContentVisitor::new()
        .visit_borrowed_bytes(value)
        .map(TagOrContent::Content)
    }
  }

  fn visit_byte_buf<F>(self, value: Vec<u8>) -> Result<Self::Value, F>
  where
    F: Error,
  {
    if value == self.name.as_bytes() {
      Ok(TagOrContent::Tag)
    } else {
      ContentVisitor::new()
        .visit_byte_buf(value)
        .map(TagOrContent::Content)
    }
  }

  fn visit_unit<F>(self) -> Result<Self::Value, F>
  where
    F: Error,
  {
    ContentVisitor::new()
      .visit_unit()
      .map(TagOrContent::Content)
  }

  fn visit_none<F>(self) -> Result<Self::Value, F>
  where
    F: Error,
  {
    ContentVisitor::new()
      .visit_none()
      .map(TagOrContent::Content)
  }

  fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
  where
    D: Deserializer<'de>,
  {
    ContentVisitor::new()
      .visit_some(deserializer)
      .map(TagOrContent::Content)
  }

  fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
  where
    D: Deserializer<'de>,
  {
    ContentVisitor::new()
      .visit_newtype_struct(deserializer)
      .map(TagOrContent::Content)
  }

  fn visit_seq<V>(self, visitor: V) -> Result<Self::Value, V::Error>
  where
    V: SeqAccess<'de>,
  {
    ContentVisitor::new()
      .visit_seq(visitor)
      .map(TagOrContent::Content)
  }

  fn visit_map<V>(self, visitor: V) -> Result<Self::Value, V::Error>
  where
    V: MapAccess<'de>,
  {
    ContentVisitor::new()
      .visit_map(visitor)
      .map(TagOrContent::Content)
  }

  fn visit_enum<V>(self, visitor: V) -> Result<Self::Value, V::Error>
  where
    V: EnumAccess<'de>,
  {
    ContentVisitor::new()
      .visit_enum(visitor)
      .map(TagOrContent::Content)
  }
}
