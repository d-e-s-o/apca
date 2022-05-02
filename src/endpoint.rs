// Copyright (C) 2019-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;
use serde_json::Error as JsonError;
use serde_urlencoded::ser::Error as UrlEncodeError;


/// An error type comprising various conversion errors we may encounter.
#[derive(Debug, Error)]
pub enum ConversionError {
  /// A variant used when a JSON conversion failed.
  #[error("failed to convert from/to JSON")]
  Json(#[from] JsonError),
  /// A variant used when we fail to URL-encode a piece of data.
  #[error("failed to URL-encode data")]
  UrlEncode(#[from] UrlEncodeError),
  /// A variant used when a Reqwest fails to convert from an HttpRequest.
  #[error("failed to URL-encode data")]
  ReqwestError(#[from] reqwest::Error),
}

use thiserror::Error;


/// An error as reported by API endpoints.
// Note that actually this type should probably be specific to the API
// version in question. However, at this point we only support v2, so we
// luck out here.
#[derive(Clone, Debug, Deserialize, Error, PartialEq)]
#[error("{message} ({code})")]
pub struct ApiError {
  /// An error code as provided by Alpaca.
  #[serde(rename = "code")]
  pub code: u64,
  /// A message as provided by Alpaca.
  #[serde(rename = "message")]
  pub message: String,
}


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
        // TODO: This status actually got changed to 403, which now
        //       shadows other error conditions by specific endpoints
        //       (e.g., insufficient funds when submitting an order).
        /* 401 */ UNAUTHORIZED => AuthenticationFailed,
        /// The rate limit was exceeded, causing the request to be
        /// denied.
        /* 429 */ TOO_MANY_REQUESTS => RateLimitExceeded,
        $($(#[$err_docs])* $err_status => $variant,)*
      ],
      ConversionErr => crate::endpoint::ConversionError,
      ApiErr => crate::endpoint::ApiError,

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
        ::serde_json::from_slice::<Self::Output>(body).map_err(Self::ConversionError::from)
      }

      fn parse_err(body: &[u8]) -> Result<Self::ApiError, Vec<u8>> {
        ::serde_json::from_slice::<Self::ApiError>(body).map_err(|_| body.to_vec())
      }
    }
  };
}
