// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;

use crate::api::v2::asset;
use crate::Str;


/// A GET request to be made to the /v2/positions/<symbol> endpoint.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PositionReq {
  /// Symbol or asset ID to identify the asset to trade.
  #[serde(rename = "symbol")]
  pub symbol: asset::Symbol,
}


/// The side of a position.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum Side {
  /// A long position of an asset.
  #[serde(rename = "long")]
  Long,
}


/// A single position as returned by the /v2/positions endpoint on a GET
/// request.
#[derive(Clone, Debug, Deserialize, PartialEq)]
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
  #[serde(rename = "qty")]
  pub quantity: Num,
  /// The side the position is on.
  #[serde(rename = "side")]
  pub side: Side,
  /// The total dollar amount of the position.
  #[serde(rename = "market_value")]
  pub market_value: Num,
  /// The total cost basis in dollar.
  #[serde(rename = "cost_basis")]
  pub cost_basis: Num,
  /// The total unrealized profit/loss in dollar.
  #[serde(rename = "unrealized_pl")]
  pub unrealized_gain_total: Num,
  /// The total unrealized profit/loss percent (as a factor of 1).
  #[serde(rename = "unrealized_plpc")]
  pub unrealized_gain_total_percent: Num,
  /// The unrealized profit/loss in dollar for the day.
  #[serde(rename = "unrealized_intraday_pl")]
  pub unrealized_gain_today: Num,
  /// The unrealized profit/loss percent for the day (as a factor of 1).
  #[serde(rename = "unrealized_intraday_plpc")]
  pub unrealized_gain_today_percent: Num,
  /// The current asset price per share.
  #[serde(rename = "current_price")]
  pub current_price: Num,
  /// The last day's asset price per share.
  #[serde(rename = "lastday_price")]
  pub last_day_price: Num,
  /// The percent change from last day price (as a factor of 1).
  #[serde(rename = "change_today")]
  pub change_today: Num,
}


/// The representation of a GET request to the /v2/positions/<symbol>
/// endpoint.
#[derive(Clone, Copy, Debug)]
pub struct Get {}

EndpointDef! {
  Get(PositionReq),
  Ok => Position, [
    /// The position with the given ID was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// No position was found with the given ID.
    /* 404 */ NOT_FOUND => NotFound,
  ]

  fn path(input: &Self::Input) -> Str {
    format!("/v2/positions/{}", input.symbol).into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_str as from_json;

  use test_env_log::test;

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::Error;


  #[test]
  fn parse_reference_position() {
    let response = r#"{
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

    let pos = from_json::<Position>(&response).unwrap();
    assert_eq!(pos.symbol, "AAPL");
    assert_eq!(pos.exchange, asset::Exchange::Nasdaq);
    assert_eq!(pos.asset_class, asset::Class::UsEquity);
    assert_eq!(pos.average_entry_price, Num::from_int(100));
    assert_eq!(pos.quantity, Num::from_int(5));
    assert_eq!(pos.side, Side::Long);
    assert_eq!(pos.market_value, Num::from_int(600));
    assert_eq!(pos.cost_basis, Num::from_int(500));
    assert_eq!(pos.unrealized_gain_total, Num::from_int(100));
    assert_eq!(pos.unrealized_gain_total_percent, Num::new(20, 100));
    assert_eq!(pos.unrealized_gain_today, Num::from_int(10));
    assert_eq!(pos.unrealized_gain_today_percent, Num::new(84, 10000));
    assert_eq!(pos.current_price, Num::from_int(120));
    assert_eq!(pos.last_day_price, Num::from_int(119));
    assert_eq!(pos.change_today, Num::new(84, 10000));
  }

  #[test(tokio::test)]
  async fn retrieve_position() -> Result<(), Error> {
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let request = PositionReq {
      symbol: asset::Symbol::Sym("SPY".to_string()),
    };
    let result = client.issue::<Get>(request).await;

    // We don't know whether there is an open position and we can't
    // simply create one as the market may be closed. So really the best
    // thing we can do is to make sure that we either get a valid
    // response or an indication that no position has been found.
    match result {
      Ok(pos) => {
        assert_eq!(pos.symbol, "SPY");
        assert_eq!(pos.asset_class, asset::Class::UsEquity);
      }
      Err(err) => match err {
        GetError::NotFound(_) => (),
        _ => panic!("Received unexpected error: {:?}", err),
      },
    }
    Ok(())
  }
}
