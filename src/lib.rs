// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

#![type_length_limit = "2097152"]
#![warn(
  bad_style,
  dead_code,
  future_incompatible,
  illegal_floating_point_literal_pattern,
  improper_ctypes,
  intra_doc_link_resolution_failure,
  late_bound_lifetime_arguments,
  missing_copy_implementations,
  missing_debug_implementations,
  missing_docs,
  no_mangle_generic_items,
  non_shorthand_field_patterns,
  nonstandard_style,
  overflowing_literals,
  path_statements,
  patterns_in_fns_without_body,
  plugin_as_library,
  private_in_public,
  proc_macro_derive_resolution_fallback,
  renamed_and_removed_lints,
  rust_2018_compatibility,
  rust_2018_idioms,
  safe_packed_borrows,
  stable_features,
  trivial_bounds,
  trivial_numeric_casts,
  type_alias_bounds,
  tyvar_behind_raw_pointer,
  unconditional_recursion,
  unreachable_code,
  unreachable_patterns,
  unstable_features,
  unstable_name_collisions,
  unused,
  unused_comparisons,
  unused_import_braces,
  unused_lifetimes,
  unused_qualifications,
  unused_results,
  where_clauses_object_safety,
  while_true,
)]

//! A create for interacting with the Alpaca API.

#[macro_use]
extern crate http_endpoint;

#[macro_use]
mod endpoint;

/// A module comprising the functionality backing interactions with the
/// API.
pub mod api;

mod api_info;
mod client;
mod error;
mod events;
mod time_util;

use std::borrow::Cow;

pub use crate::api_info::ApiInfo;
pub use crate::client::Client;
pub use crate::error::Error;

/// A module providing access to lower level event streaming.
///
/// It is typically only in rare situations that this lower level
/// functionality needs to be used directly.
pub mod event {
  pub use crate::events::stream;
}

type Str = Cow<'static, str>;
