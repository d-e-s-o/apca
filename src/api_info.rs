// Copyright (C) 2019-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env::var_os;
use std::ffi::OsString;

use url::Url;

use crate::api::API_BASE_URL;
use crate::data::DATA_BASE_URL;
use crate::data::DATA_STREAM_BASE_URL;
use crate::Error;

/// The base URL of the Trading API to use.
const ENV_API_BASE_URL: &str = "APCA_API_BASE_URL";
/// The URL of the websocket stream portion of the Trading API to use.
const ENV_API_STREAM_URL: &str = "APCA_API_STREAM_URL";
/// The environment variable representing the key ID.
const ENV_KEY_ID: &str = "APCA_API_KEY_ID";
/// The environment variable representing the secret key.
const ENV_SECRET: &str = "APCA_API_SECRET_KEY";


/// Convert a Trading API base URL into the corresponding one for
/// websocket streaming.
fn make_api_stream_url(base_url: Url) -> Result<Url, Error> {
  let mut url = base_url;
  url.set_scheme("wss").map_err(|()| {
    Error::Str(format!("unable to change URL scheme for {}: invalid URL?", url).into())
  })?;
  url.set_path("stream");
  Ok(url)
}


/// An object encapsulating the information used for working with the
/// Alpaca API.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ApiInfo {
  /// The base URL for the Trading API.
  pub api_base_url: Url,
  /// The websocket stream URL for the Trading API.
  pub api_stream_url: Url,
  /// The base URL for data retrieval.
  pub data_base_url: Url,
  /// The websocket base URL for streaming of data.
  pub data_stream_base_url: Url,
  /// The key ID to use for authentication.
  pub key_id: String,
  /// The secret to use for authentication.
  pub secret: String,
}

impl ApiInfo {
  /// Create an `ApiInfo` from the required data. Note that using this
  /// constructor the websocket URL will be inferred based on the base
  /// URL provided.
  ///
  /// # Errors
  /// - [`Error::Url`](crate::Error::Url) If `api_base_url` cannot be parsed
  ///   into a [`url::Url`](url::Url).
  pub fn from_parts(
    api_base_url: impl AsRef<str>,
    key_id: impl ToString,
    secret: impl ToString,
  ) -> Result<Self, Error> {
    let api_base_url = Url::parse(api_base_url.as_ref())?;
    let api_stream_url = make_api_stream_url(api_base_url.clone())?;

    Ok(Self {
      api_base_url,
      api_stream_url,
      // We basically only work with statically defined URL parts here
      // which we know can be parsed successfully, so unwrapping is
      // fine.
      data_base_url: Url::parse(DATA_BASE_URL).unwrap(),
      data_stream_base_url: Url::parse(DATA_STREAM_BASE_URL).unwrap(),
      key_id: key_id.to_string(),
      secret: secret.to_string(),
    })
  }

  /// Create an `ApiInfo` object with information from the environment.
  ///
  /// This constructor retrieves API related information from the
  /// environment and performs some preliminary validation on it. The
  /// following information is used:
  /// - the Alpaca Trading API base URL is retrieved from the
  ///   `APCA_API_BASE_URL` variable
  /// - the Alpaca Trading API stream URL is retrieved from the
  ///   `APCA_API_STREAM_URL` variable
  /// - the Alpaca account key ID is retrieved from the
  ///   `APCA_API_KEY_ID` variable
  /// - the Alpaca account secret is retrieved from the
  ///   `APCA_API_SECRET_KEY` variable
  ///
  /// # Notes
  /// - Neither of the two data APIs can be configured via the
  ///   environment currently; defaults will be used
  #[allow(unused_qualifications)]
  pub fn from_env() -> Result<Self, Error> {
    let api_base_url = var_os(ENV_API_BASE_URL)
      .unwrap_or_else(|| OsString::from(API_BASE_URL))
      .into_string()
      .map_err(|_| {
        Error::Str(
          format!(
            "{} environment variable is not a valid string",
            ENV_API_BASE_URL
          )
          .into(),
        )
      })?;
    let api_base_url = Url::parse(&api_base_url)?;

    let api_stream_url = var_os(ENV_API_STREAM_URL)
      .map(Result::<_, Error>::Ok)
      .unwrap_or_else(|| {
        // If the user did not provide an explicit websocket URL then
        // infer the one to use based on the API base URL.
        let url = make_api_stream_url(api_base_url.clone())?;
        Ok(OsString::from(url.as_str()))
      })?
      .into_string()
      .map_err(|_| {
        Error::Str(
          format!(
            "{} environment variable is not a valid string",
            ENV_API_STREAM_URL
          )
          .into(),
        )
      })?;
    let api_stream_url = Url::parse(&api_stream_url)?;

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
      api_base_url,
      api_stream_url,
      // We basically only work with statically defined URL parts here
      // which we know can be parsed successfully, so unwrapping is
      // fine.
      data_base_url: Url::parse(DATA_BASE_URL).unwrap(),
      data_stream_base_url: Url::parse(DATA_STREAM_BASE_URL).unwrap(),
      key_id,
      secret,
    })
  }
}


#[cfg(test)]
mod tests {
  use super::*;


  /// Check that we can create an [`ApiInfo`] object from its
  /// constituent parts.
  #[test]
  fn from_parts() {
    let api_base_url = "https://paper-api.alpaca.markets/";
    let key_id = "XXXXXXXXXXXXXXXXXXXX";
    let secret = "YYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY";

    let api_info = ApiInfo::from_parts(api_base_url, key_id, secret).unwrap();
    assert_eq!(api_info.api_base_url.as_str(), api_base_url);
    assert_eq!(api_info.key_id, key_id);
    assert_eq!(api_info.secret, secret);
  }
}
