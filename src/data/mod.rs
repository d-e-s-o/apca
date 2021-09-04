// Copyright (C) 2020 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

/// Definitions for the first version of the Alpaca Data API.
pub mod v1;
/// Definitions for the second version of the Alpaca Data API.
pub mod v2;

/// The API base URL used for retrieving market data.
pub(crate) const DATA_BASE_URL: &str = "https://data.alpaca.markets";
