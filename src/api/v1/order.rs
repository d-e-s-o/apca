// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ops::Deref;

use hyper::Body;
use hyper::Chunk;
use hyper::http::request::Builder;
use hyper::Request;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;
use serde_json::to_string as to_json;

use uuid::Uuid;

use crate::Error;
use crate::requestor::Endpoint;
use crate::Str;


/// An ID uniquely identifying an order.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct Id(pub Uuid);

impl Deref for Id {
  type Target = Uuid;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}


/// The status an order can have.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum Status {
  /// The order has been received by Alpaca, and routed to exchanges for
  /// execution. This is the usual initial state of an order.
  #[serde(rename = "new")]
  New,
  /// The order has been partially filled.
  #[serde(rename = "partially_filled")]
  PartiallyFilled,
  /// The order has been filled, and no further updates will occur for
  /// the order.
  #[serde(rename = "filled")]
  Filled,
  /// The order is done executing for the day, and will not receive
  /// further updates until the next trading day.
  #[serde(rename = "done_for_day")]
  DoneForDay,
  /// The order has been canceled, and no further updates will occur for
  /// the order. This can be either due to a cancel request by the user,
  /// or the order has been canceled by the exchanges due to its
  /// time-in-force.
  #[serde(rename = "canceled")]
  Canceled,
  /// The order has expired, and no further updates will occur for the
  /// order.
  #[serde(rename = "expired")]
  Expired,
  /// The order has been received by Alpaca, but hasn't yet been routed
  /// to the execution venue. This state only occurs on rare occasions.
  #[serde(rename = "accepted")]
  Accepted,
  /// The order has been received by Alpaca, and routed to the
  /// exchanges, but has not yet been accepted for execution. This state
  /// only occurs on rare occasions.
  #[serde(rename = "pending_new")]
  PendingNew,
  /// The order has been received by exchanges, and is evaluated for
  /// pricing. This state only occurs on rare occasions.
  #[serde(rename = "accepted_for_bidding")]
  AcceptedForBidding,
  /// The order is waiting to be canceled. This state only occurs on
  /// rare occasions.
  #[serde(rename = "pending_cancel")]
  PendingCancel,
  /// The order has been stopped, and a trade is guaranteed for the
  /// order, usually at a stated price or better, but has not yet
  /// occurred. This state only occurs on rare occasions.
  #[serde(rename = "stopped")]
  Stopped,
  /// The order has been rejected, and no further updates will occur for
  /// the order. This state occurs on rare occasions and may occur based
  /// on various conditions decided by the exchanges.
  #[serde(rename = "rejected")]
  Rejected,
  /// The order has been suspended, and is not eligible for trading.
  /// This state only occurs on rare occasions.
  #[serde(rename = "suspended")]
  Suspended,
  /// The order has been completed for the day (either filled or done
  /// for day), but remaining settlement calculations are still pending.
  /// This state only occurs on rare occasions.
  #[serde(rename = "calculated")]
  Calculated,
}


/// The side an order is on.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum Side {
  /// Buy an asset.
  #[serde(rename = "buy")]
  Buy,
  /// Sell an asset.
  #[serde(rename = "sell")]
  Sell,
}


/// The type of an order.
// Note that we currently do not support `stop_limit` orders.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum Type {
  /// A market order.
  #[serde(rename = "market")]
  Market,
  /// A limit order.
  #[serde(rename = "limit")]
  Limit,
  /// A stop on quote order.
  #[serde(rename = "stop")]
  Stop,
  /// A stop limit order.
  #[serde(rename = "stop_limit")]
  StopLimit,
}


/// A description of the time for which an order is valid.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum TimeInForce {
  /// The order is good for the day, and it will be canceled
  /// automatically at the end of Regular Trading Hours if unfilled.
  #[serde(rename = "day")]
  Day,
  /// The order is good until canceled.
  #[serde(rename = "gtc")]
  UntilCanceled,
}


/// A POST request to be made to the /v1/orders endpoint.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct OrderReq {
  /// Symbol or asset ID to identify the asset to trade.
  #[serde(rename = "symbol")]
  pub symbol: String,
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
}

/// A single order as returned by the /v1/orders endpoint on a GET
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
  #[serde(rename = "created_at")]
  pub created_at: String,
  /// Timestamp this order was updated at last.
  #[serde(rename = "updated_at")]
  pub updated_at: Option<String>,
  /// Timestamp this order was submitted at.
  #[serde(rename = "submitted_at")]
  pub submitted_at: Option<String>,
  /// Timestamp this order was filled at.
  #[serde(rename = "filled_at")]
  pub filled_at: Option<String>,
  /// Timestamp this order expired at.
  #[serde(rename = "expired_at")]
  pub expired_at: Option<String>,
  /// Timestamp this order expired at.
  #[serde(rename = "canceled_at")]
  pub canceled_at: Option<String>,
  /// The order's asset class.
  #[serde(rename = "asset_class")]
  pub asset_class: String,
  /// The ID of the asset represented by the order.
  #[serde(rename = "asset_id")]
  pub asset_id: String,
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
}


/// The representation of a POST request to the /v1/orders endpoint.
#[derive(Clone, Copy, Debug)]
pub struct Post {}

EndpointDef! {
  Post,
  Ok => Order, PostOk, [
    /* 200 */ OK,
  ],
  Err => PostError, [
    /* 403 */ FORBIDDEN => InsufficientFunds,
    /* 422 */ UNPROCESSABLE_ENTITY => InvalidInput,
  ]
}

impl Endpoint for Post {
  type Input = OrderReq;
  type Output = PostOk;
  type Error = PostError;

  fn path(_input: &Self::Input) -> Str {
    "/v1/orders".into()
  }

  fn builder(url: &str, _input: &Self::Input) -> Builder {
    Request::post(url)
  }

  fn request(builder: &mut Builder, input: &Self::Input) -> Result<Request<Body>, Error> {
    let json = to_json(input)?;
    let body = Body::from(Chunk::from(json));
    builder.body(body).map_err(Error::from)
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_str as from_json;
  use serde_json::to_string as to_json;


  #[test]
  fn emit_side() {
    assert_eq!(to_json(&Side::Buy).unwrap(), r#""buy""#);
    assert_eq!(to_json(&Side::Sell).unwrap(), r#""sell""#);
  }

  #[test]
  fn emit_type() {
    assert_eq!(to_json(&Type::Market).unwrap(), r#""market""#);
    assert_eq!(to_json(&Type::Limit).unwrap(), r#""limit""#);
    assert_eq!(to_json(&Type::Stop).unwrap(), r#""stop""#);
  }

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
    "status": "accepted"
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
}
