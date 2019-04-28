// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

#![deny(
  dead_code,
  illegal_floating_point_literal_pattern,
  improper_ctypes,
  intra_doc_link_resolution_failure,
  late_bound_lifetime_arguments,
  missing_copy_implementations,
  missing_debug_implementations,
  missing_docs,
  no_mangle_generic_items,
  non_shorthand_field_patterns,
  overflowing_literals,
  path_statements,
  patterns_in_fns_without_body,
  plugin_as_library,
  private_in_public,
  proc_macro_derive_resolution_fallback,
  safe_packed_borrows,
  stable_features,
  trivial_bounds,
  trivial_numeric_casts,
  type_alias_bounds,
  tyvar_behind_raw_pointer,
  unconditional_recursion,
  unions_with_drop_fields,
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
#![warn(
  bad_style,
  future_incompatible,
  nonstandard_style,
  renamed_and_removed_lints,
  rust_2018_compatibility,
  rust_2018_idioms,
)]

//! A create for interacting with the Alpaca API.

#[macro_use]
mod endpoint;

/// A module comprising the functionality backing interactions with the
/// API.
pub mod api;

mod client;
mod env;
mod error;
mod events;

use std::borrow::Cow;

pub use crate::client::Client;
pub use crate::error::Error;

type Str = Cow<'static, str>;
