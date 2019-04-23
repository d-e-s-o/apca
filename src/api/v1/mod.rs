// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

/// Definitions pertaining the user's account.
pub mod account;
/// Definitions surrounding assets.
pub mod asset;
/// Functionality for listing available assets.
pub mod assets;
/// Definitions surrounding orders.
pub mod order;
/// Functionality for listing orders.
pub mod orders;
/// Definitions surrounding open positions.
pub mod position;

#[cfg(test)]
mod order_util;
