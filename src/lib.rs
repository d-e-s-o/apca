// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

//! A create for interacting with the Alpaca API.

#[macro_use]
mod macros;

/// A module comprising the functionality backing interactions with the
/// API.
pub mod api;

mod error;

use std::borrow::Cow;

type Str = Cow<'static, str>;
