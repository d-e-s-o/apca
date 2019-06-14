// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ops::Deref;
use std::time::SystemTime;

use num_decimal::Num;

use serde::Deserialize;

use uuid::Uuid;

use crate::api::time_util::system_time;
use crate::endpoint::Endpoint;
use crate::Str;


/// A type representing an account ID.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct Id(pub Uuid);

impl Deref for Id {
  type Target = Uuid;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}


/// A response as returned by the /v1/account endpoint.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Account {
  /// Account ID.
  #[serde(rename = "id")]
  pub id: Id,
  /// The account's status.
  // TODO: Not yet implemented.
  #[serde(rename = "status")]
  pub status: String,
  /// The currency the account uses.
  #[serde(rename = "currency")]
  pub currency: String,
  /// Tradable buying power.
  #[serde(rename = "buying_power")]
  pub buying_power: Num,
  /// Cash balance.
  #[serde(rename = "cash")]
  pub cash: Num,
  /// Withdrawable cash amount.
  #[serde(rename = "cash_withdrawable")]
  pub withdrawable_cash: Num,
  /// Total value of cash + holding positions.
  #[serde(rename = "portfolio_value")]
  pub portfolio_value: Num,
  /// Whether or not the account has been flagged as a pattern day
  /// trader.
  #[serde(rename = "pattern_day_trader")]
  pub day_trader: bool,
  /// If true, the account is not allowed to place orders.
  #[serde(rename = "trading_blocked")]
  pub trading_blocked: bool,
  /// If true, the account is not allowed to request money transfers.
  #[serde(rename = "transfers_blocked")]
  pub transfers_blocked: bool,
  /// If true, the account activity by user is prohibited.
  #[serde(rename = "account_blocked")]
  pub account_blocked: bool,
  /// Timestamp this account was created at.
  #[serde(rename = "created_at", deserialize_with = "system_time")]
  pub created_at: SystemTime,
}


/// The representation of a GET request to the /v1/accounts endpoint.
#[derive(Clone, Copy, Debug)]
pub struct Get {}

EndpointDef! {
  Get,
  Ok => Account, [
    /// The account information was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []
}

impl Endpoint for Get {
  type Input = ();
  type Output = Account;
  type Error = GetError;

  fn path(_input: &Self::Input) -> Str {
    "/v1/account".into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::time::Duration;
  use std::time::UNIX_EPOCH;

  use serde_json::from_str as from_json;

  use test_env_log::test;

  use tokio::runtime::current_thread::block_on_all;

  use url::Url;

  use crate::api::API_BASE_URL;
  use crate::Client;
  use crate::Error;


  #[test]
  fn parse_reference_account() {
    let response = r#"{
  "id": "904837e3-3b76-47ec-b432-046db621571b",
  "status": "ACTIVE",
  "currency": "USD",
  "buying_power": "4000.32",
  "cash": "4000.32",
  "cash_withdrawable": "4000.32",
  "portfolio_value": "4321.98",
  "pattern_day_trader": false,
  "trading_blocked": false,
  "transfers_blocked": false,
  "account_blocked": false,
  "created_at": "2018-10-01T13:35:25Z"
}"#;

    let id = Id(Uuid::parse_str("904837e3-3b76-47ec-b432-046db621571b").unwrap());
    let acc = from_json::<Account>(&response).unwrap();
    assert_eq!(acc.id, id);
    assert_eq!(acc.currency, "USD");
    assert_eq!(acc.buying_power, Num::new(400032, 100));
    assert_eq!(acc.withdrawable_cash, Num::new(400032, 100));
    assert_eq!(acc.portfolio_value, Num::new(432198, 100));
    assert_eq!(acc.trading_blocked, false);
    // Expected timestamp inferred via TZ='UTC' date --date='2018-10-01T13:35:25' +'%s'
    assert_eq!(acc.created_at, UNIX_EPOCH + Duration::from_secs(1538400925))
  }

  #[test]
  fn request_account() -> Result<(), Error> {
    let client = Client::from_env()?;
    let future = client.issue::<Get>(())?;
    let account = block_on_all(future)?;

    // Just a few sanity checks to verify that we did receive something
    // meaningful from the correct API endpoint.
    assert_eq!(account.currency, "USD");
    assert!(!account.account_blocked);
    Ok(())
  }

  #[test]
  fn request_account_with_invalid_credentials() -> Result<(), Error> {
    let api_base = Url::parse(API_BASE_URL)?;
    let client = Client::new(api_base, b"invalid".to_vec(), b"invalid-too".to_vec())?;
    let future = client.issue::<Get>(())?;

    let err = block_on_all(future).unwrap_err();
    match err {
      GetError::AuthenticationFailed(_) => (),
      e @ _ => panic!("received unexpected error: {:?}", e),
    }
    Ok(())
  }
}
