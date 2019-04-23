// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

//! A create for interacting with the Alpaca API.

#[macro_use]
mod macros;

/// A module comprising the functionality backing interactions with the
/// API.
pub mod api;

mod error;
mod requestor;

use std::borrow::Cow;

pub use crate::error::Error;
pub use crate::requestor::Requestor;

type Str = Cow<'static, str>;

/// The base URL to the API to use.
const ENV_API: &str = "APCA_API_BASE_URL";
/// The environment variable representing the key ID.
const ENV_KEY_ID: &str = "APCA_API_KEY_ID";
/// The environment variable representing the secret key.
const ENV_SECRET: &str = "APCA_API_SECRET_KEY";
