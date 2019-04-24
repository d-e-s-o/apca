// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env::var_os;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

use url::Url;

use crate::api::API_BASE_URL;
use crate::Error;

/// The base URL to the API to use.
const ENV_API: &str = "APCA_API_BASE_URL";
/// The environment variable representing the key ID.
const ENV_KEY_ID: &str = "APCA_API_KEY_ID";
/// The environment variable representing the secret key.
const ENV_SECRET: &str = "APCA_API_SECRET_KEY";


/// Retrieve API related information from the environment.
///
/// This function retrieves API related information from the environment
/// and performs some preliminary validation on it. In particular, the
/// following information is retrieved:
/// - the Alpaca API base URL is retrieved from the APCA_API_BASE_URL
///   variable
/// - the Alpaca account key ID is retrieved from the APCA_API_KEY_ID
///   variable
/// - the Alpaca account secret is retrieved from the APCA_API_SECRET_KEY
///   variable
pub fn api_info() -> Result<(Url, Vec<u8>, Vec<u8>), Error> {
  let api_base = var_os(ENV_API)
    .unwrap_or_else(|| OsString::from(API_BASE_URL))
    .into_string()
    .map_err(|_| {
      Error::Str(format!("{} environment variable is not a valid string", ENV_API).into())
    })?;
  let api_base = Url::parse(&api_base)?;

  let key_id = var_os(ENV_KEY_ID)
    .ok_or_else(|| Error::Str(format!("{} environment variable not found", ENV_KEY_ID).into()))?;
  let secret = var_os(ENV_SECRET)
    .ok_or_else(|| Error::Str(format!("{} environment variable not found", ENV_SECRET).into()))?;

  Ok((api_base, key_id.into_vec(), secret.into_vec()))
}
