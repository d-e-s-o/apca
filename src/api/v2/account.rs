// Copyright (C) 2019-2021 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ops::Deref;
use std::time::SystemTime;

use num_decimal::Num;

use serde::Deserialize;

use uuid::Uuid;

use time_util::system_time_from_str;

use crate::Str;


/// A type representing an account ID.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct Id(pub Uuid);

impl Deref for Id {
  type Target = Uuid;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}


/// An enumeration of the various states an account can be in.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum Status {
  /// The account is onboarding.
  #[serde(rename = "ONBOARDING")]
  Onboarding,
  /// The account application submission failed for some reason.
  #[serde(rename = "SUBMISSION_FAILED")]
  SubmissionFailed,
  /// The account application has been submitted for review.
  #[serde(rename = "SUBMITTED")]
  Submitted,
  /// The account information is being updated.
  #[serde(rename = "ACCOUNT_UPDATED")]
  Updating,
  /// The final account approval is pending.
  #[serde(rename = "APPROVAL_PENDING")]
  ApprovalPending,
  /// The account is active for trading.
  #[serde(rename = "ACTIVE")]
  Active,
  /// The account application has been rejected.
  #[serde(rename = "REJECTED")]
  Rejected,
  /// Any other account status that we have not accounted for.
  ///
  /// Note that having any such status should be considered a bug.
  #[serde(other)]
  Unknown,
}


/// A response as returned by the /v2/account endpoint.
// TODO: The `sma` field is not yet hooked up.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Account {
  /// Account ID.
  #[serde(rename = "id")]
  pub id: Id,
  /// The account's status.
  #[serde(rename = "status")]
  pub status: Status,
  /// The currency the account uses.
  #[serde(rename = "currency")]
  pub currency: String,
  /// Cash balance.
  #[serde(rename = "cash")]
  pub cash: Num,
  /// Whether or not the account has been flagged as a pattern day
  /// trader.
  #[serde(rename = "pattern_day_trader")]
  pub day_trader: bool,
  /// Whether or not the user has suspended trading operations.
  #[serde(rename = "trade_suspended_by_user")]
  pub trading_suspended: bool,
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
  #[serde(rename = "created_at", deserialize_with = "system_time_from_str")]
  pub created_at: SystemTime,
  /// Flag to denote whether or not the account is permitted to short.
  #[serde(rename = "shorting_enabled")]
  pub shorting_enabled: bool,
  /// Real-time mark-to-market value of all long positions held in the
  /// account.
  #[serde(rename = "long_market_value")]
  pub market_value_long: Num,
  /// Real-time mark-to-market value of all short positions held in the
  /// account.
  #[serde(rename = "short_market_value")]
  pub market_value_short: Num,
  /// The sum of `cash`, `market_value_long`, and `market_value_short`.
  #[serde(rename = "equity")]
  pub equity: Num,
  /// Equity as of previous trading day at 16:00:00 ET.
  #[serde(rename = "last_equity")]
  pub last_equity: Num,
  /// Buying power multiplier that represents account margin
  /// classification. Valid values are:
  /// - 1: the standard limited margin account with 1x buying power
  /// - 2: regular margin account with 2x intra day and overnight buying
  ///      power (the default for all non-pattern-day-trader accounts
  ///      with USD 2000 or more equity),
  /// - 4: pattern day trader account with 4x intra day buying power and
  ///      2x regular overnight buying power
  #[serde(rename = "multiplier")]
  pub multiplier: Num,
  /// The currently available buying power. Calculated based on the
  /// multiplier:
  /// - 1: cash
  /// - 2: max(equity – initial_margin, 0) * 2
  /// - 4: (last_equity - (last) maintenance_margin) * 4
  #[serde(rename = "buying_power")]
  pub buying_power: Num,
  /// Initial margin requirement (this value is continuously updated).
  #[serde(rename = "initial_margin")]
  pub initial_margin: Num,
  /// Maintenance margin requirement (this value is continuously updated).
  #[serde(rename = "maintenance_margin")]
  pub maintenance_margin: Num,
  /// The current number of day trades that have been made in the last
  /// five trading days (including today).
  #[serde(rename = "daytrade_count")]
  pub daytrade_count: u64,
}


Endpoint! {
  /// The representation of a GET request to the /v2/account endpoint.
  pub Get(()),
  Ok => Account, [
    /// The account information was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  fn path(_input: &Self::Input) -> Str {
    "/v2/account".into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_str as from_json;

  use test_env_log::test;

  use time_util::parse_system_time_from_str;

  use uuid::Uuid;

  use crate::api::API_BASE_URL;
  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::RequestError;


  #[test]
  fn parse_reference_account() {
    let response = r#"{
  "id": "904837e3-3b76-47ec-b432-046db621571b",
  "status": "ACTIVE",
  "currency": "USD",
  "buying_power": "0.0",
  "cash": "1000.00",
  "portfolio_value": "5000.00",
  "pattern_day_trader": false,
  "trade_suspended_by_user": false,
  "trading_blocked": false,
  "transfers_blocked": false,
  "account_blocked": false,
  "created_at": "2018-10-01T13:35:25Z",
  "shorting_enabled": true,
  "multiplier": "2",
  "long_market_value": "7000.00",
  "short_market_value": "-3000.00",
  "equity": "5000.00",
  "last_equity": "5000.00",
  "initial_margin": "5000.00",
  "maintenance_margin": "3000.00",
  "daytrade_count": 0,
  "sma": "0.0"
}"#;

    let id = Id(Uuid::parse_str("904837e3-3b76-47ec-b432-046db621571b").unwrap());
    let acc = from_json::<Account>(&response).unwrap();
    assert_eq!(acc.id, id);
    assert_eq!(acc.status, Status::Active);
    assert_eq!(acc.currency, "USD");
    assert_eq!(acc.buying_power, Num::from(0));
    assert_eq!(acc.trading_blocked, false);
    assert_eq!(
      acc.created_at,
      parse_system_time_from_str("2018-10-01T13:35:25Z").unwrap()
    );
    assert_eq!(acc.market_value_long, Num::from(7000));
    assert_eq!(acc.market_value_short, Num::from(-3000));
    assert_eq!(acc.equity, Num::from(5000));
    assert_eq!(acc.last_equity, Num::from(5000));
    assert_eq!(acc.maintenance_margin, Num::from(3000));
    assert_eq!(acc.daytrade_count, 0);
  }

  #[test(tokio::test)]
  async fn request_account() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let account = client.issue::<Get>(()).await.unwrap();

    assert_eq!(account.currency, "USD");
    assert!(!account.account_blocked);

    let multiplier = account.multiplier.to_u64().unwrap();
    assert!(
      multiplier == 1 || multiplier == 2 || multiplier == 4,
      "{}",
      multiplier,
    );
  }

  #[test(tokio::test)]
  async fn request_account_with_invalid_credentials() {
    let api_base = Url::parse(API_BASE_URL).unwrap();
    let api_info = ApiInfo::builder()
                     .base_url(api_base)
                     .key_id("invalid")
                     .secret("invalid-too")
                     .build()
                     .unwrap();

    let client = Client::new(api_info);
    let result = client.issue::<Get>(()).await;

    let err = result.unwrap_err();
    match err {
      RequestError::Endpoint(GetError::AuthenticationFailed(_)) => (),
      e => panic!("received unexpected error: {:?}", e),
    }
  }
}
