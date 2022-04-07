// Copyright (C) 2019-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ops::Not;

use http::Method;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;

use crate::api::v2::asset;
use crate::api::v2::order;
use crate::util::abs_num_from_str;
use crate::Str;


/// The side of a position.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum Side {
  /// A long position of an asset.
  #[serde(rename = "long")]
  Long,
  /// A short position of an asset.
  #[serde(rename = "short")]
  Short,
}

impl Not for Side {
  type Output = Self;

  #[inline]
  fn not(self) -> Self::Output {
    match self {
      Self::Long => Self::Short,
      Self::Short => Self::Long,
    }
  }
}


/// A single position as returned by the /v2/positions endpoint on a GET
/// request.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Position {
  /// The ID of the asset represented by the position.
  #[serde(rename = "asset_id")]
  pub asset_id: asset::Id,
  /// The symbol of the asset being traded.
  #[serde(rename = "symbol")]
  pub symbol: String,
  /// The exchange the position is held at.
  #[serde(rename = "exchange")]
  pub exchange: asset::Exchange,
  /// The position's asset class.
  #[serde(rename = "asset_class")]
  pub asset_class: asset::Class,
  /// The average entry price of the position.
  #[serde(rename = "avg_entry_price")]
  pub average_entry_price: Num,
  /// The number of shares.
  #[serde(rename = "qty", deserialize_with = "abs_num_from_str")]
  pub quantity: Num,
  /// The side the position is on.
  #[serde(rename = "side")]
  pub side: Side,
  /// The total dollar amount of the position.
  #[serde(rename = "market_value")]
  pub market_value: Option<Num>,
  /// The total cost basis in dollar.
  #[serde(rename = "cost_basis")]
  pub cost_basis: Num,
  /// The total unrealized profit/loss in dollar.
  #[serde(rename = "unrealized_pl")]
  pub unrealized_gain_total: Option<Num>,
  /// The total unrealized profit/loss percent (as a factor of 1).
  #[serde(rename = "unrealized_plpc")]
  pub unrealized_gain_total_percent: Option<Num>,
  /// The unrealized profit/loss in dollar for the day.
  #[serde(rename = "unrealized_intraday_pl")]
  pub unrealized_gain_today: Option<Num>,
  /// The unrealized profit/loss percent for the day (as a factor of 1).
  #[serde(rename = "unrealized_intraday_plpc")]
  pub unrealized_gain_today_percent: Option<Num>,
  /// The current asset price per share.
  #[serde(rename = "current_price")]
  pub current_price: Option<Num>,
  /// The last day's asset price per share.
  #[serde(rename = "lastday_price")]
  pub last_day_price: Option<Num>,
  /// The percent change from last day price (as a factor of 1).
  #[serde(rename = "change_today")]
  pub change_today: Option<Num>,
}


Endpoint! {
  /// The representation of a GET request to the /v2/positions/<symbol>
  /// endpoint.
  pub Get(asset::Symbol),
  Ok => Position, [
    /// The position with the given ID was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// No position was found with the given ID.
    /* 404 */ NOT_FOUND => NotFound,
  ]

  #[inline]
  fn path(input: &Self::Input) -> Str {
    format!("/v2/positions/{}", input).into()
  }
}


Endpoint! {
  /// The representation of a DELETE request to the
  /// /v2/positions/<symbol> endpoint.
  pub Delete(asset::Symbol),
  Ok => order::Order, [
    /// The position was liquidated successfully.
    /* 200 */ OK,
  ],
  Err => DeleteError, [
    /// No position was found for the given symbol/asset ID.
    /* 404 */ NOT_FOUND => NotFound,
  ]

  #[inline]
  fn method() -> Method {
    Method::DELETE
  }

  #[inline]
  fn path(input: &Self::Input) -> Str {
    format!("/v2/positions/{}", input).into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_str as from_json;
  use serde_json::to_string as to_json;

  use test_log::test;

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::RequestError;


  /// Check that we can negate a `Side` object.
  #[test]
  fn negate_side() {
    assert_eq!(!Side::Long, Side::Short);
    assert_eq!(!Side::Short, Side::Long);
  }

  /// Make sure that we can deserialize and serialize a `Position`
  /// object.
  #[test]
  fn deserialize_serialize_reference_position() {
    let json = r#"{
    "asset_id": "904837e3-3b76-47ec-b432-046db621571b",
    "symbol": "AAPL",
    "exchange": "NASDAQ",
    "asset_class": "us_equity",
    "avg_entry_price": "100.0",
    "qty": "5",
    "side": "long",
    "market_value": "600.0",
    "cost_basis": "500.0",
    "unrealized_pl": "100.0",
    "unrealized_plpc": "0.20",
    "unrealized_intraday_pl": "10.0",
    "unrealized_intraday_plpc": "0.0084",
    "current_price": "120.0",
    "lastday_price": "119.0",
    "change_today": "0.0084"
}"#;

    let pos =
      from_json::<Position>(&to_json(&from_json::<Position>(json).unwrap()).unwrap()).unwrap();
    assert_eq!(pos.symbol, "AAPL");
    assert_eq!(pos.exchange, asset::Exchange::Nasdaq);
    assert_eq!(pos.asset_class, asset::Class::UsEquity);
    assert_eq!(pos.average_entry_price, Num::from(100));
    assert_eq!(pos.quantity, Num::from(5));
    assert_eq!(pos.side, Side::Long);
    assert_eq!(pos.market_value, Some(Num::from(600)));
    assert_eq!(pos.cost_basis, Num::from(500));
    assert_eq!(pos.unrealized_gain_total, Some(Num::from(100)));
    assert_eq!(pos.unrealized_gain_total_percent, Some(Num::new(20, 100)));
    assert_eq!(pos.unrealized_gain_today, Some(Num::from(10)));
    assert_eq!(pos.unrealized_gain_today_percent, Some(Num::new(84, 10000)));
    assert_eq!(pos.current_price, Some(Num::from(120)));
    assert_eq!(pos.last_day_price, Some(Num::from(119)));
    assert_eq!(pos.change_today, Some(Num::new(84, 10000)));
  }

  /// Check that we can parse a position with a fractional quantity.
  #[test]
  fn parse_fractional_position() {
    let response = r#"{
    "asset_id": "904837e3-3b76-47ec-b432-046db621571b",
    "symbol": "AAPL",
    "exchange": "NASDAQ",
    "asset_class": "us_equity",
    "avg_entry_price": "100.0",
    "qty": "0.5",
    "side": "long",
    "market_value": "600.0",
    "cost_basis": "500.0",
    "unrealized_pl": "100.0",
    "unrealized_plpc": "0.20",
    "unrealized_intraday_pl": "10.0",
    "unrealized_intraday_plpc": "0.0084",
    "current_price": "120.0",
    "lastday_price": "119.0",
    "change_today": "0.0084"
}"#;

    let pos = from_json::<Position>(response).unwrap();
    assert_eq!(pos.symbol, "AAPL");
    assert_eq!(pos.exchange, asset::Exchange::Nasdaq);
    assert_eq!(pos.asset_class, asset::Class::UsEquity);
    assert_eq!(pos.average_entry_price, Num::from(100));
    assert_eq!(pos.quantity, Num::new(1, 2));
    assert_eq!(pos.side, Side::Long);
    assert_eq!(pos.market_value, Some(Num::from(600)));
    assert_eq!(pos.cost_basis, Num::from(500));
    assert_eq!(pos.unrealized_gain_total, Some(Num::from(100)));
    assert_eq!(pos.unrealized_gain_total_percent, Some(Num::new(20, 100)));
    assert_eq!(pos.unrealized_gain_today, Some(Num::from(10)));
    assert_eq!(pos.unrealized_gain_today_percent, Some(Num::new(84, 10000)));
    assert_eq!(pos.current_price, Some(Num::from(120)));
    assert_eq!(pos.last_day_price, Some(Num::from(119)));
    assert_eq!(pos.change_today, Some(Num::new(84, 10000)));
  }

  /// Check that we can parse a short position.
  #[test]
  fn parse_short_position() {
    let response = r#"{
      "asset_id":"d704f4fd-c735-44f8-a7fa-7a50fef08fe4",
      "symbol":"XLK",
      "exchange":"ARCA",
      "asset_class":"us_equity",
      "qty":"-24",
      "avg_entry_price":"82.69",
      "side":"short",
      "market_value":"-2011.44",
      "cost_basis":"-1984.56",
      "unrealized_pl":"-26.88",
      "unrealized_plpc":"-0.0135445640343451",
      "unrealized_intraday_pl":"-26.88",
      "unrealized_intraday_plpc":"-0.0135445640343451",
      "current_price":"83.81",
      "lastday_price":"88.91",
      "change_today":"-0.0573613766730402"
    }"#;

    let pos = from_json::<Position>(response).unwrap();
    assert_eq!(pos.symbol, "XLK");
    assert_eq!(pos.quantity, Num::from(24));
  }

  /// Check that we can retrieve an open position, if one exists.
  #[test(tokio::test)]
  async fn retrieve_position() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let symbol = asset::Symbol::Sym("SPY".to_string());
    let result = client.issue::<Get>(&symbol).await;

    // We don't know whether there is an open position and we can't
    // simply create one as the market may be closed. So really the best
    // thing we can do is to make sure that we either get a valid
    // response or an indication that no position has been found.
    match result {
      Ok(pos) => {
        assert_eq!(pos.symbol, "SPY");
        assert_eq!(pos.asset_class, asset::Class::UsEquity);
      },
      Err(err) => match err {
        RequestError::Endpoint(GetError::NotFound(..)) => (),
        _ => panic!("Received unexpected error: {:?}", err),
      },
    }
  }
}
