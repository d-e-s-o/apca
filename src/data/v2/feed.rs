// Copyright (C) 2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Serialize;


/// An enumeration of the different supported data feeds.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum Feed {
  /// Use the Investors Exchange (IEX) as the data source.
  ///
  /// This feed is available unconditionally, i.e., with the free and
  /// unlimited plans.
  #[serde(rename = "iex")]
  IEX,
  /// Use CTA (administered by NYSE) and UTP (administered by Nasdaq)
  /// SIPs as the data source.
  ///
  /// This feed is only usable with the unlimited market data plan.
  #[serde(rename = "sip")]
  SIP,
}
