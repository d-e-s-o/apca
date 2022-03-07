// Copyright (C) 2021-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

mod feed;
mod unfold;

/// Definitions for retrieval of market data bars.
pub mod bars;
/// Functionality for retrieval of the most recent quote.
pub mod last_quote;
/// Functionality for retrieving historic quotes.
pub mod quotes;
/// Definitions for real-time streaming of market data.
pub mod stream;

pub use feed::Feed;
