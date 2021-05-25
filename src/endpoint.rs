// Copyright (C) 2019-2021 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use serde::Deserialize;


#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ErrorMessage {
  /// An error code as provided by Alpaca.
  #[serde(rename = "code")]
  pub code: u64,
  /// A message as provided by Alpaca.
  #[serde(rename = "message")]
  pub message: String,
}

impl Display for ErrorMessage {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    write!(fmt, "{} ({})", self.message, self.code)
  }
}

impl Error for ErrorMessage {}


/// A macro used for defining the properties for a request to a
/// particular HTTP endpoint, without automated JSON parsing.
macro_rules! EndpointNoParse {
  ( $(#[$docs:meta])* $pub:vis $name:ident($in:ty),
    Ok => $out:ty, [$($(#[$ok_docs:meta])* $ok_status:ident,)*],
    Err => $err:ident, [$($(#[$err_docs:meta])* $err_status:ident => $variant:ident,)*]
    $($defs:tt)* ) => {

    EndpointDef! {
      $(#[$docs])* $pub $name($in),
      Ok => $out, [$($ok_status,)*],
      Err => $err, [
        // Every request can result in an authentication failure or fall
        // prey to the rate limit and so we include these variants into
        // all our error definitions.
        /// Authentication failed for the request.
        /* 401 */ UNAUTHORIZED => AuthenticationFailed,
        /// The rate limit was exceeded, causing the request to be
        /// denied.
        /* 429 */ TOO_MANY_REQUESTS => RateLimitExceeded,
        $($(#[$err_docs])* $err_status => $variant,)*
      ],
      ConversionErr => ::serde_json::Error,
      ApiErr => crate::endpoint::ErrorMessage,

      $($defs)*
    }
  };
}

/// A macro used for defining the properties for a request to a
/// particular HTTP endpoint.
macro_rules! Endpoint {
  ( $($input:tt)* ) => {
    EndpointNoParse! {
      $($input)*

      fn parse(body: &[u8]) -> Result<Self::Output, Self::ConversionError> {
        ::serde_json::from_slice::<Self::Output>(body)
      }

      fn parse_err(body: &[u8]) -> Result<Self::ApiError, Vec<u8>> {
        ::serde_json::from_slice::<Self::ApiError>(body).map_err(|_| body.to_vec())
      }
    }
  };
}
