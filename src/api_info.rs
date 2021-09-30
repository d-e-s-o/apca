// Copyright (C) 2019-2021 The apca Developers
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
  /// - [`Error::Url`](crate::Error::Url) If `base_url` cannot be parsed
  ///   into a [`url::Url`](url::Url).
  pub fn from_parts(
    base_url: impl AsRef<str>,
    key_id: impl ToString,
    secret: impl ToString,
  ) -> Result<Self, Error> {
    Ok(Self {
      base_url: Url::parse(base_url.as_ref())?,
      key_id: key_id.to_string(),
      secret: secret.to_string(),
    })
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
    let base_url = var_os(ENV_API_URL)
      .unwrap_or_else(|| OsString::from(API_BASE_URL))
      .into_string()
      .map_err(|_| {
        Error::Str(format!("{} environment variable is not a valid string", ENV_API_URL).into())
      })?;
    let base_url = Url::parse(&base_url)?;

    let key_id = var_os(ENV_KEY_ID)
      .ok_or_else(|| Error::Str(format!("{} environment variable not found", ENV_KEY_ID).into()))?
      .into_string()
      .map_err(|_| {
        Error::Str(format!("{} environment variable is not a valid string", ENV_KEY_ID).into())
      })?;

    let secret = var_os(ENV_SECRET)
      .ok_or_else(|| Error::Str(format!("{} environment variable not found", ENV_SECRET).into()))?
      .into_string()
      .map_err(|_| {
        Error::Str(format!("{} environment variable is not a valid string", ENV_SECRET).into())
      })?;

    Ok(Self {
      base_url,
      key_id,
      secret,
    })
  }
}
