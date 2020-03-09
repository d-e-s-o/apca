// Copyright (C) 2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::de::Error as SerdeError;
use serde::de::Unexpected;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serializer;


/// Parse a `u64` from a string.
fn parse_u64<'de, D>(s: &str) -> Result<u64, D::Error>
where
  D: Deserializer<'de>,
{
  u64::from_str_radix(&s, 10)
    .map_err(|_| SerdeError::invalid_value(Unexpected::Str(&s), &"an unsigned integer"))
}

/// Deserialize a string encoded `u64`.
pub fn u64_from_str<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
  D: Deserializer<'de>,
{
  parse_u64::<D>(&String::deserialize(deserializer)?)
}

/// Deserialize an optional `u64` from a string.
pub fn optional_u64_from_str<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
  D: Deserializer<'de>,
{
  match Option::<String>::deserialize(deserializer)? {
    Some(s) => Some(parse_u64::<D>(&s)).transpose(),
    None => Ok(None),
  }
}

/// Serialize a `u64` value as a string.
pub fn u64_to_str<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  serializer.serialize_str(&value.to_string())
}
