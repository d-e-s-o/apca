// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

/// Definitions for the first version of the Alpaca API.
pub mod v1;

mod time_util;

/// The API base URL used for paper trading.
pub(crate) const API_BASE_URL: &str = "https://paper-api.alpaca.markets";
/// The HTTP header representing the key ID.
pub(crate) const HDR_KEY_ID: &str = "APCA-API-KEY-ID";
/// The HTTP header representing the secret key.
pub(crate) const HDR_SECRET: &str = "APCA-API-SECRET-KEY";
