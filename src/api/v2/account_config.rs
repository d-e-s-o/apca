// Copyright (C) 2020-2024 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use http::Method;
use http_endpoint::Bytes;

use serde::Deserialize;
use serde::Serialize;
use serde_json::to_vec as to_json;

use crate::Str;


/// An enum representing the possible trade confirmation settings.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TradeConfirmation {
  /// Send an e-mail to confirm trades.
  #[serde(rename = "all")]
  Email,
  /// Provide no confirmation for trades.
  #[serde(rename = "none")]
  None,
}


/// A response as returned by the /v2/account/configurations endpoint.
// TODO: Not all fields are hooked up yet.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Configuration {
  /// Whether and how trades are confirmed.
  #[serde(rename = "trade_confirm_email")]
  pub trade_confirmation: TradeConfirmation,
  /// If enabled, new orders are blocked.
  #[serde(rename = "suspend_trade")]
  pub trading_suspended: bool,
  /// If enabled, the account can only submit buy orders.
  #[serde(rename = "no_shorting")]
  pub no_shorting: bool,
}


Endpoint! {
  /// The representation of a GET request to the
  /// /v2/account/configurations endpoint.
  pub Get(()),
  Ok => Configuration, [
    /// The account configuration was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  #[inline]
  fn path(_input: &Self::Input) -> Str {
    "/v2/account/configurations".into()
  }
}


Endpoint! {
  /// The representation of a PATCH request to the
  /// /v2/account/configurations endpoint.
  pub Change(Configuration),
  Ok => Configuration, [
    /// The account configuration was updated successfully.
    /* 200 */ OK,
  ],
  Err => ChangeError, [
    /// One of the new values is invalid/unacceptable.
    /* 400 */ BAD_REQUEST => InvalidValues,
  ]

  #[inline]
  fn method() -> Method {
    Method::PATCH
  }

  #[inline]
  fn path(_input: &Self::Input) -> Str {
    "/v2/account/configurations".into()
  }

  fn body(input: &Self::Input) -> Result<Option<Bytes>, Self::ConversionError> {
    let json = to_json(input)?;
    let bytes = Bytes::from(json);
    Ok(Some(bytes))
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_str as from_json;

  use test_log::test;

  use crate::api_info::ApiInfo;
  use crate::Client;


  #[test]
  fn parse_reference_configuration() {
    let response = r#"{
  "dtbp_check": "entry",
  "no_shorting": false,
  "suspend_trade": false,
  "trade_confirm_email": "all"
}"#;

    let config = from_json::<Configuration>(response).unwrap();
    assert_eq!(config.trade_confirmation, TradeConfirmation::Email);
    assert!(!config.trading_suspended);
    assert!(!config.no_shorting);
  }

  #[test(tokio::test)]
  async fn retrieve_and_update_configuration() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let config = client.issue::<Get>(&()).await.unwrap();

    // We invert the trade confirmation strategy, which should be a
    // change not affecting any tests running concurrently.
    let new_confirmation = match config.trade_confirmation {
      TradeConfirmation::Email => TradeConfirmation::None,
      TradeConfirmation::None => TradeConfirmation::Email,
    };

    let changed = Configuration {
      trade_confirmation: new_confirmation,
      ..config
    };
    let change_result = client.issue::<Change>(&changed).await;
    // Also retrieve the configuration again.
    let get_result = client.issue::<Get>(&()).await;
    // Revert back to the original setting.
    let reverted = client.issue::<Change>(&config).await.unwrap();

    assert_eq!(change_result.unwrap(), changed);
    assert_eq!(get_result.unwrap(), changed);
    assert_eq!(reverted, config);
  }
}
