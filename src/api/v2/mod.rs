// Copyright (C) 2019-2021 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

/// Definitions pertaining the user's account.
pub mod account;
/// Definitions pertaining account activities.
pub mod account_activities;
/// Definitions pertaining the user's account configuration.
pub mod account_config;
/// Definitions surrounding assets.
pub mod asset;
/// Functionality for listing available assets.
pub mod assets;
/// Functionality for retrieving market open/close timing information.
pub mod clock;
/// Definitions for account and trade related events.
pub mod events;
/// Definitions surrounding orders.
pub mod order;
/// Functionality for listing orders.
pub mod orders;
/// Definitions surrounding open positions.
pub mod position;
/// Functionality for listing open positions.
pub mod positions;

mod de;
mod util;

#[cfg(test)]
mod order_util;
