// Copyright (C) 2020-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::Str;

use num_decimal::Num;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde_variant::to_variant_name;


/// Deserialize a `Num` from a string, parsing the value as signed first
/// and then dropping the sign.
pub(crate) fn abs_num_from_str<'de, D>(deserializer: D) -> Result<Num, D::Error>
where
  D: Deserializer<'de>,
{
  Num::deserialize(deserializer).map(|num| if num.is_negative() { num * -1 } else { num })
}


/// Deserialize a `Vec` from a string that could contain a `null`.
pub(crate) fn vec_from_str<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
  D: Deserializer<'de>,
  T: Deserialize<'de>,
{
  let vec = Option::<Vec<T>>::deserialize(deserializer)?;
  Ok(vec.unwrap_or_default())
}


/// Deserialize a `Vec<String>` from a string (that could be `null`)
/// with comma separated elements.
pub(crate) fn vec_from_comma_separated_str<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
  D: Deserializer<'de>,
{
  let vec = Option::<String>::deserialize(deserializer)?
    .map(|string| string.split(',').map(str::to_owned).collect())
    .unwrap_or_default();

  Ok(vec)
}


/// Serialize a slice into a string of textual representations of the
/// elements, retrieved by applying a function to each, and then
/// separated by comma.
pub(crate) fn slice_to_str<S, F, T>(
  slice: &[T],
  name_fn: F,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: Serializer,
  F: Fn(&T) -> Str,
  T: Serialize,
{
  if !slice.is_empty() {
    // `serde_urlencoded` seemingly does not know how to handle a
    // `Vec`. So what we do is we convert each and every element to a
    // string and then concatenate them, separating each by comma.
    let s = slice.iter().map(name_fn).collect::<Vec<_>>().join(",");
    serializer.serialize_str(&s)
  } else {
    serializer.serialize_none()
  }
}


/// Serialize a slice into a string of textual representations of the
/// elements separated by comma.
///
/// # Notes
/// - this function should only be used for cases where `T` is an enum
///   type
pub(crate) fn enum_slice_to_str<S, T>(slice: &[T], serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
  T: Serialize,
{
  fn name_fn<T>(variant: &T) -> Str
  where
    T: Serialize,
  {
    // We know that we are dealing with an enum variant and the
    // function will never return an error for those, so it's fine
    // to unwrap.
    to_variant_name(variant).unwrap().into()
  }

  slice_to_str(slice, name_fn, serializer)
}

/// Serialize a slice of strings into a comma-separated string combining
/// the individual strings.
pub(crate) fn string_slice_to_str<S>(slice: &[String], serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  #[allow(clippy::ptr_arg)]
  fn name_fn(string: &String) -> Str {
    string.clone().into()
  }

  slice_to_str(slice, name_fn, serializer)
}
