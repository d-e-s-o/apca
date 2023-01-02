// Copyright (C) 2019-2023 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

/// Definitions pertaining the user's account.
pub mod account;
/// Definitions pertaining account activities.
pub mod account_activities;
/// Definitions pertaining the user's account configuration.
pub mod account_config;
/// Definitions pertaining the account portfolio.
pub mod account_portfolio;
/// Definitions surrounding assets.
pub mod asset;
/// Functionality for listing available assets.
pub mod assets;
/// Functionality for retrieving market open/close timing information
/// for specific dates.
pub mod calendar;
/// Functionality for retrieving market open/close timing information
/// for the current trading day.
pub mod clock;
/// Definitions surrounding orders.
pub mod order;
/// Functionality for listing orders.
pub mod orders;
/// Definitions surrounding open positions.
pub mod position;
/// Functionality for listing open positions.
pub mod positions;
/// Definitions for trade related updates.
pub mod updates;
/// Definitions surrounding watchlists.
pub mod watchlist;
/// Functionality for listing watchlists.
pub mod watchlists;

mod de;

#[cfg(test)]
mod order_util;
