// Copyright (C) 2019-2021 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;
use serde::Serialize;


/// An enumeration of the different event streams.
#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum StreamType {
  /// A stream for trade updates.
  #[serde(rename = "trade_updates")]
  TradeUpdates,
}
