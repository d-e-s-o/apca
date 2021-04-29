// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env::var_os;
use std::ffi::OsString;

use url::Url;

use crate::api::API_BASE_URL;
use crate::Error;

/// The base URL to the API to use.
const ENV_API_URL: &str = "APCA_API_BASE_URL";
/// The environment variable representing the key ID.
const ENV_KEY_ID: &str = "APCA_API_KEY_ID";
/// The environment variable representing the secret key.
const ENV_SECRET: &str = "APCA_API_SECRET_KEY";

/// An object encapsulating the information used for working with the
/// Alpaca API.
#[derive(Clone, Debug, PartialEq)]
pub struct ApiInfo {
  /// The base URL for the API.
  pub(crate) base_url: Url,
  /// The key ID to use for authentication.
  pub(crate) key_id: String,
  /// The secret to use for authentication.
  pub(crate) secret: String,
}

impl ApiInfo {
  /// Create an `ApiInfo` from the required data.
  ///
  /// # Errors
  /// - `crate::Error::Url` If `base_url` cannot be parsed into a `url::Url`.
  pub fn from_parts(
    base_url: impl AsRef<str>,
    key_id: impl ToString,
    secret: impl ToString,
  ) -> Result<Self, Error> {
    let me = Self {
      base_url: Url::parse(base_url.as_ref())?,
      key_id: key_id.to_string(),
      secret: secret.to_string(),
    };

    Ok(me)
  }

  /// Create an `ApiInfo` object with information from the environment.
  ///
  /// This constructor retrieves API related information from the
  /// environment and performs some preliminary validation on it. The
  /// following information is used:
  /// - the Alpaca API base URL is retrieved from the APCA_API_BASE_URL
  ///   variable
  /// - the Alpaca account key ID is retrieved from the APCA_API_KEY_ID
  ///   variable
  /// - the Alpaca account secret is retrieved from the APCA_API_SECRET_KEY
  ///   variable
  pub fn from_env() -> Result<Self, Error> {
    let err_var_unparseable = |key: &str| -> Error {
      Error::Str(format!("{} environment variable is not a valid string", key).into())
    };
    let err_var_missing = |key: &str| -> Error {
      Error::Str(format!("{} environment variable not found", key).into())
    };

    let get_env = |key: &str| -> Option<Result<String, Error>> {
      var_os(key)
        .map(OsString::into_string)
        .map(|res| res.map_err(|_| err_var_unparseable(key)))
    };

    let me = Self {
      base_url: get_env(ENV_API_URL)
        .unwrap_or(Ok(API_BASE_URL.to_string()))
        .and_then(|val| Url::parse(&val).map_err(Into::into))?,
      key_id: get_env(ENV_KEY_ID).unwrap_or(Err(err_var_missing(ENV_KEY_ID)))?,
      secret: get_env(ENV_SECRET).unwrap_or(Err(err_var_missing(ENV_SECRET)))?,
    };

    Ok(me)
  }
}
