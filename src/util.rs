// Copyright (C) 2020-2021 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use num_decimal::Num;

use serde::Deserialize;
use serde::Deserializer;

/// Deserialize a `Num` from a string, parsing the value as signed first
/// and then dropping the sign.
pub fn abs_num_from_str<'de, D>(deserializer: D) -> Result<Num, D::Error>
where
  D: Deserializer<'de>,
{
  Num::deserialize(deserializer).map(|num| if num.is_negative() { num * -1 } else { num })
}
