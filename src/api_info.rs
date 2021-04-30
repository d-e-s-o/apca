// Copyright (C) 2019-2021 Daniel Mueller <deso@posteo.net>
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
  /// Create an `ApiInfoBuilder`
  pub fn builder() -> ApiInfoBuilder {ApiInfoBuilder::new()}

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
    Self::builder()
      .base_url(base_url)
      .key_id(key_id)
      .secret(secret)
      .build()
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

    let me = ApiInfoBuilder::new()
               .base_url(get_env(ENV_API_URL).unwrap_or(Ok(API_BASE_URL.to_string()))?)
               .key_id(get_env(ENV_KEY_ID).unwrap_or(Err(err_var_missing(ENV_KEY_ID)))?)
               .secret(get_env(ENV_SECRET).unwrap_or(Err(err_var_missing(ENV_SECRET)))?)
               .build()?;

    Ok(me)
  }
}

#[derive(Debug, Default)]
pub struct ApiInfoBuilder {
  errors: Vec<Error>,
  base_url: Option<Url>,
  key_id: Option<String>,
  secret: Option<String>,
}

impl ApiInfoBuilder {
  pub fn new() -> Self {Default::default()}

  pub fn build(mut self) -> Result<ApiInfo, Error> {
    macro_rules! fail_if_unset {
      ($test:expr, $errors:expr) => {
        match $test {
          Some(_) => (),
          None => {
            let msg = format!("ApiInfoBuilder: {} was not set, but is required!", stringify!($test));
            $errors.push(Error::Str(msg.into()));
          }
        }
      }
    }

    fail_if_unset!(&self.base_url, &mut self.errors);
    fail_if_unset!(&self.key_id,   &mut self.errors);
    fail_if_unset!(&self.secret,   &mut self.errors);

    if self.errors.len() > 0 {
      Err(Error::Many(self.errors))
    } else {
      let me = ApiInfo {
        base_url: self.base_url.clone().unwrap(),
        key_id: self.key_id.unwrap(),
        secret: self.secret.unwrap(),
      };

      Ok(me)
    }
  }

  pub fn base_url(mut self, url: impl AsRef<str>) -> Self {
    let url = Url::parse(url.as_ref());
    match url {
      Ok(url) => {
        self.base_url = Some(url);
      },
      Err(e) => self.errors.push(e.into()),
    };

    self
  }

  pub fn key_id(mut self, key_id: impl ToString) -> Self {
    self.key_id = Some(key_id.to_string());

    self
  }

  pub fn secret(mut self, secret: impl ToString) -> Self {
    self.secret = Some(secret.to_string());

    self
  }
}

mod test {
  use super::*;
  use std::borrow::Borrow;

  #[test]
  pub fn api_builder_should_error_when_none_set() {
    // ARRANGE
    let builder = ApiInfoBuilder::new();

    // ACT
    let result = builder.build();

    // ASSERT
    assert!(matches!(result, Err(_)));
    let err = result.unwrap_err();
    assert!(matches!(err, Error::Many(_)));

    if let Error::Many(errs) = err {
      assert!(errs.len() == 3);
      assert!(errs.into_iter().all(|err| match err {
        Error::Str(cow) => {
          let str: &str = cow.borrow();
          str.contains("base_url was not set") ||
          str.contains("secret was not set") ||
          str.contains("key_id was not set")
        },
        _ => false
      }));
    }
  }
}
