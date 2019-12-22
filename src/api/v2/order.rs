// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::SystemTime;

use hyper::Body;
use hyper::Method;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Error as JsonError;
use serde_json::to_string as to_json;

pub use crate::api::v1::order::Id;
pub use crate::api::v1::order::Side;
pub use crate::api::v1::order::Status;
pub use crate::api::v1::order::TimeInForce;
pub use crate::api::v1::order::Type;

use crate::api::time_util::optional_system_time;
use crate::api::time_util::system_time;
use crate::api::v2::asset;
use crate::endpoint::Endpoint;
use crate::Str;


/// A POST request to be made to the /v2/orders endpoint.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct OrderReq {
  /// Symbol or asset ID to identify the asset to trade.
  #[serde(rename = "symbol")]
  pub symbol: asset::Symbol,
  /// Number of shares to trade.
  #[serde(rename = "qty")]
  pub quantity: u64,
  /// The side the order is on.
  #[serde(rename = "side")]
  pub side: Side,
  /// `market`, `limit`, `stop`, or `stop_limit`.
  #[serde(rename = "type")]
  pub type_: Type,
  /// How long the order will be valid.
  #[serde(rename = "time_in_force")]
  pub time_in_force: TimeInForce,
  /// The limit price.
  #[serde(rename = "limit_price")]
  pub limit_price: Option<Num>,
  /// The stop price.
  #[serde(rename = "stop_price")]
  pub stop_price: Option<Num>,
  /// Whether or not the order is eligible to execute during
  /// pre-market/after hours. Note that a value of `true` can only be
  /// combined with limit orders that are good for the day (i.e.,
  /// `TimeInForce::Day`).
  #[serde(rename = "extended_hours")]
  pub extended_hours: bool,
}

/// A single order as returned by the /v2/orders endpoint on a GET
/// request.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Order {
  /// The order's ID.
  #[serde(rename = "id")]
  pub id: Id,
  /// Client unique order ID.
  #[serde(rename = "client_order_id")]
  pub client_order_id: String,
  /// The status of the order.
  #[serde(rename = "status")]
  pub status: Status,
  /// Timestamp this order was created at.
  #[serde(rename = "created_at", deserialize_with = "system_time")]
  pub created_at: SystemTime,
  /// Timestamp this order was updated at last.
  #[serde(rename = "updated_at", deserialize_with = "optional_system_time")]
  pub updated_at: Option<SystemTime>,
  /// Timestamp this order was submitted at.
  #[serde(rename = "submitted_at", deserialize_with = "optional_system_time")]
  pub submitted_at: Option<SystemTime>,
  /// Timestamp this order was filled at.
  #[serde(rename = "filled_at", deserialize_with = "optional_system_time")]
  pub filled_at: Option<SystemTime>,
  /// Timestamp this order expired at.
  #[serde(rename = "expired_at", deserialize_with = "optional_system_time")]
  pub expired_at: Option<SystemTime>,
  /// Timestamp this order expired at.
  #[serde(rename = "canceled_at", deserialize_with = "optional_system_time")]
  pub canceled_at: Option<SystemTime>,
  /// The order's asset class.
  #[serde(rename = "asset_class")]
  pub asset_class: asset::Class,
  /// The ID of the asset represented by the order.
  #[serde(rename = "asset_id")]
  pub asset_id: asset::Id,
  /// The symbol of the asset being traded.
  #[serde(rename = "symbol")]
  pub symbol: String,
  /// The quantity being requested.
  #[serde(rename = "qty")]
  pub quantity: Num,
  /// The quantity that was filled.
  #[serde(rename = "filled_qty")]
  pub filled_quantity: Num,
  /// The type of order.
  #[serde(rename = "type")]
  pub type_: Type,
  /// The side the order is on.
  #[serde(rename = "side")]
  pub side: Side,
  /// A representation of how long the order will be valid.
  #[serde(rename = "time_in_force")]
  pub time_in_force: TimeInForce,
  /// The limit price.
  #[serde(rename = "limit_price")]
  pub limit_price: Option<Num>,
  /// The stop price.
  #[serde(rename = "stop_price")]
  pub stop_price: Option<Num>,
  /// If true, the order is eligible for execution outside regular
  /// trading hours.
  #[serde(rename = "extended_hours")]
  pub extended_hours: bool,
}


/// The representation of a GET request to the /v2/orders/<order-id>
/// endpoint.
#[derive(Clone, Copy, Debug)]
pub struct Get {}

EndpointDef! {
  Get,
  Ok => Order, [
    /// The order object for the given ID was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// No order was found with the given ID.
    /* 404 */ NOT_FOUND => NotFound,
  ]
}

impl Endpoint for Get {
  type Input = Id;
  type Output = Order;
  type Error = GetError;

  fn path(input: &Self::Input) -> Str {
    format!("/v2/orders/{}", input.to_simple()).into()
  }
}


/// The representation of a POST request to the /v2/orders endpoint.
#[derive(Clone, Copy, Debug)]
pub struct Post {}

EndpointDef! {
  Post,
  Ok => Order, [
    /// The order was submitted successfully.
    /* 200 */ OK,
  ],
  Err => PostError, [
    /// Not enough funds are available to submit the order.
    /* 403 */ FORBIDDEN => InsufficientFunds,
    /// Some data in the request was invalid.
    /* 422 */ UNPROCESSABLE_ENTITY => InvalidInput,
  ]
}

impl Endpoint for Post {
  type Input = OrderReq;
  type Output = Order;
  type Error = PostError;

  fn method() -> Method {
    Method::POST
  }

  fn path(_input: &Self::Input) -> Str {
    "/v2/orders".into()
  }

  fn body(input: &Self::Input) -> Result<Body, JsonError> {
    let json = to_json(input)?;
    let body = Body::from(json);
    Ok(body)
  }
}


/// The representation of a DELETE request to the /v2/orders/<order-id>
/// endpoint.
#[derive(Clone, Copy, Debug)]
pub struct Delete {}

EndpointDef! {
  Delete,
  Ok => (), [
    /// The order was canceled successfully.
    /* 204 */ NO_CONTENT,
  ],
  Err => DeleteError, [
    /// No order was found with the given ID.
    /* 404 */ NOT_FOUND => NotFound,
    /// The order can no longer be canceled.
    /* 422 */ UNPROCESSABLE_ENTITY => NotCancelable,
  ]
}

impl Endpoint for Delete {
  type Input = Id;
  type Output = ();
  type Error = DeleteError;

  fn method() -> Method {
    Method::DELETE
  }

  fn path(input: &Self::Input) -> Str {
    format!("/v2/orders/{}", input.to_simple()).into()
  }

  fn parse(body: &[u8]) -> Result<Self::Output, Self::Error> {
    debug_assert_eq!(body, b"");
    Ok(())
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_str as from_json;

  use test_env_log::test;

  use uuid::Uuid;

  use crate::api::v2::asset::Class;
  use crate::api::v2::asset::Exchange;
  use crate::api::v2::asset::Symbol;
  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::Error;


  #[test]
  fn parse_reference_order() {
    let response = r#"{
    "id": "904837e3-3b76-47ec-b432-046db621571b",
    "client_order_id": "904837e3-3b76-47ec-b432-046db621571b",
    "created_at": "2018-10-05T05:48:59Z",
    "updated_at": "2018-10-05T05:48:59Z",
    "submitted_at": "2018-10-05T05:48:59Z",
    "filled_at": "2018-10-05T05:48:59Z",
    "expired_at": "2018-10-05T05:48:59Z",
    "canceled_at": "2018-10-05T05:48:59Z",
    "failed_at": "2018-10-05T05:48:59Z",
    "asset_id": "904837e3-3b76-47ec-b432-046db621571b",
    "symbol": "AAPL",
    "asset_class": "us_equity",
    "qty": "15",
    "filled_qty": "0",
    "type": "market",
    "side": "buy",
    "time_in_force": "day",
    "limit_price": "107.00",
    "stop_price": "106.00",
    "filled_avg_price": "106.00",
    "status": "accepted",
    "extended_hours": false
}"#;

    let id = Id(Uuid::parse_str("904837e3-3b76-47ec-b432-046db621571b").unwrap());
    let order = from_json::<Order>(&response).unwrap();
    assert_eq!(order.id, id);
    assert_eq!(order.symbol, "AAPL");
    assert_eq!(order.quantity, Num::from_int(15));
    assert_eq!(order.type_, Type::Market);
    assert_eq!(order.time_in_force, TimeInForce::Day);
    assert_eq!(order.limit_price, Some(Num::from_int(107)));
    assert_eq!(order.stop_price, Some(Num::from_int(106)));
  }

  #[test(tokio::test)]
  async fn submit_limit_order() -> Result<(), Error> {
    async fn test(extended_hours: bool) -> Result<(), Error> {
      let request = OrderReq {
        symbol: Symbol::SymExchgCls("SPY".to_string(), Exchange::Arca, Class::UsEquity),
        quantity: 1,
        side: Side::Buy,
        type_: Type::Limit,
        time_in_force: TimeInForce::Day,
        limit_price: Some(Num::from_int(1)),
        stop_price: None,
        extended_hours,
      };
      let api_info = ApiInfo::from_env()?;
      let client = Client::new(api_info);

      let order = client.issue::<Post>(request).await?;
      let _ = client.issue::<Delete>(order.id).await?;

      assert_eq!(order.symbol, "SPY");
      assert_eq!(order.quantity, Num::from_int(1));
      assert_eq!(order.side, Side::Buy);
      assert_eq!(order.type_, Type::Limit);
      assert_eq!(order.time_in_force, TimeInForce::Day);
      assert_eq!(order.limit_price, Some(Num::from_int(1)));
      assert_eq!(order.stop_price, None);
      assert_eq!(order.extended_hours, extended_hours);
      Ok(())
    }

    test(true).await?;
    test(false).await?;
    Ok(())
  }

  #[test(tokio::test)]
  async fn extended_hours_market_order() -> Result<(), Error> {
    let request = OrderReq {
      symbol: Symbol::SymExchgCls("SPY".to_string(), Exchange::Arca, Class::UsEquity),
      quantity: 1,
      side: Side::Buy,
      type_: Type::Limit,
      time_in_force: TimeInForce::Day,
      limit_price: None,
      stop_price: None,
      extended_hours: true,
    };
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);

    // We are submitted a market order with extended_hours, that is
    // invalid as per the Alpaca documentation.
    let result = client.issue::<Post>(request).await;
    let err = result.unwrap_err();

    match err {
      PostError::InvalidInput(_) => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
    Ok(())
  }
}
