// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ops::Deref;
use std::time::SystemTime;

use hyper::Body;
use hyper::Method;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Error as JsonError;
use serde_json::to_string as to_json;

use uuid::Uuid;

use time_util::optional_system_time_from_str;
use time_util::optional_system_time_to_rfc3339;
use time_util::system_time_from_str;
use time_util::system_time_to_rfc3339;

use crate::api::v2::asset;
use crate::Str;


/// An ID uniquely identifying an order.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Id(pub Uuid);

impl Deref for Id {
  type Target = Uuid;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}


/// The status an order can have.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum Status {
  /// The order has been received by Alpaca, and routed to exchanges for
  /// execution. This is the usual initial state of an order.
  #[serde(rename = "new")]
  New,
  /// The order has changed.
  #[serde(rename = "replaced")]
  Replaced,
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
  /// This order is eligible to execute only in the market opening
  /// auction. Any unfilled orders after the open will be canceled.
  #[serde(rename = "opg")]
  UntilMarketOpen,
  /// This order is eligible to execute only in the market closing
  /// auction. Any unfilled orders after the close will be canceled.
  #[serde(rename = "cls")]
  UntilMarketClose,
}


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


/// A PATCH request to be made to the /v2/orders/<order-id> endpoint.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ChangeReq {
  /// Number of shares to trade.
  #[serde(rename = "qty")]
  pub quantity: u64,
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


/// A single order as returned by the /v2/orders endpoint on a GET
/// request.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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
  #[serde(
    rename = "created_at",
    deserialize_with = "system_time_from_str",
    serialize_with = "system_time_to_rfc3339",
  )]
  pub created_at: SystemTime,
  /// Timestamp this order was updated at last.
  #[serde(
    rename = "updated_at",
    deserialize_with = "optional_system_time_from_str",
    serialize_with = "optional_system_time_to_rfc3339",
  )]
  pub updated_at: Option<SystemTime>,
  /// Timestamp this order was submitted at.
  #[serde(
    rename = "submitted_at",
    deserialize_with = "optional_system_time_from_str",
    serialize_with = "optional_system_time_to_rfc3339",
  )]
  pub submitted_at: Option<SystemTime>,
  /// Timestamp this order was filled at.
  #[serde(
    rename = "filled_at",
    deserialize_with = "optional_system_time_from_str",
    serialize_with = "optional_system_time_to_rfc3339",
  )]
  pub filled_at: Option<SystemTime>,
  /// Timestamp this order expired at.
  #[serde(
    rename = "expired_at",
    deserialize_with = "optional_system_time_from_str",
    serialize_with = "optional_system_time_to_rfc3339",
  )]
  pub expired_at: Option<SystemTime>,
  /// Timestamp this order expired at.
  #[serde(
    rename = "canceled_at",
    deserialize_with = "optional_system_time_from_str",
    serialize_with = "optional_system_time_to_rfc3339",
  )]
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


Endpoint! {
  /// The representation of a GET request to the /v2/orders/<order-id>
  /// endpoint.
  pub Get(Id),
  Ok => Order, [
    /// The order object for the given ID was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// No order was found with the given ID.
    /* 404 */ NOT_FOUND => NotFound,
  ]

  fn path(input: &Self::Input) -> Str {
    format!("/v2/orders/{}", input.to_simple()).into()
  }
}


Endpoint! {
  /// The representation of a POST request to the /v2/orders endpoint.
  pub Post(OrderReq),
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


Endpoint! {
  /// The representation of a PATCH request to the /v2/orders/<order-id>
  /// endpoint.
  pub Patch((Id, ChangeReq)),
  Ok => Order, [
    /// The order object for the given ID was changed successfully.
    /* 200 */ OK,
  ],
  Err => PatchError, [
    /// Not enough funds are available to submit the order.
    /* 403 */ FORBIDDEN => InsufficientFunds,
    /// No order was found with the given ID.
    /* 404 */ NOT_FOUND => NotFound,
    /// Some data in the request was invalid.
    /* 422 */ UNPROCESSABLE_ENTITY => InvalidInput,
  ]

  fn method() -> Method {
    Method::PATCH
  }

  fn path(input: &Self::Input) -> Str {
    let (id, _) = input;
    format!("/v2/orders/{}", id.to_simple()).into()
  }

  fn body(input: &Self::Input) -> Result<Body, JsonError> {
    let (_, request) = input;
    let json = to_json(request)?;
    let body = Body::from(json);
    Ok(body)
  }
}


Endpoint! {
  /// The representation of a DELETE request to the /v2/orders/<order-id>
  /// endpoint.
  pub Delete(Id),
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

  use http_endpoint::Error as EndpointError;

  use hyper::StatusCode;

  use serde_json::from_str as from_json;

  use test_env_log::test;

  use uuid::Uuid;

  use crate::api::v2::asset::Class;
  use crate::api::v2::asset::Exchange;
  use crate::api::v2::asset::Symbol;
  use crate::api::v2::order_util::order_aapl;
  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::Error;


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

      let order = client
        .issue::<Post>(request)
        .await
        .map_err(EndpointError::from)?;
      let _ = client
        .issue::<Delete>(order.id)
        .await
        .map_err(EndpointError::from)?;

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

    test(false).await?;

    // When an extended hours order is submitted between 6pm and 8pm,
    // the Alpaca API reports an error:
    // > {"code":42210000,"message":"extended hours orders between 6:00pm
    // >   and 8:00pm is not supported"}
    //
    // So we need to treat this case specially.
    let result = test(true).await;
    match result {
      Ok(()) |
      Err(Error::HttpStatus(StatusCode::UNPROCESSABLE_ENTITY)) => (),
      err => panic!("unexpected error: {:?}", err),
    };
    Ok(())
  }

  #[test(tokio::test)]
  async fn submit_other_order_types() -> Result<(), Error> {
    async fn test(time_in_force: TimeInForce) -> Result<(), Error> {
      let api_info = ApiInfo::from_env()?;
      let client = Client::new(api_info);
      let request = OrderReq {
        symbol: asset::Symbol::Sym("AAPL".to_string()),
        quantity: 1,
        side: Side::Buy,
        type_: Type::Limit,
        time_in_force,
        limit_price: Some(Num::from_int(1)),
        stop_price: None,
        extended_hours: false,
      };

      match client.issue::<Post>(request).await {
        Ok(order) => {
          let _ = client
            .issue::<Delete>(order.id)
            .await
            .map_err(EndpointError::from)?;

          assert_eq!(order.time_in_force, time_in_force);
        },
        // Submission of those orders may fail at certain times of the
        // day as per the Alpaca documentation. So ignore those errors.
        Err(PostError::InvalidInput(..)) => (),
        Err(err) => panic!("Received unexpected error: {:?}", err),
      }
      Ok(())
    }

    test(TimeInForce::UntilMarketOpen).await?;
    test(TimeInForce::UntilMarketClose).await?;
    Ok(())
  }

  #[test(tokio::test)]
  async fn submit_unsatisfiable_order() -> Result<(), Error> {
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let request = OrderReq {
      symbol: asset::Symbol::Sym("AAPL".to_string()),
      quantity: 100000,
      side: Side::Buy,
      type_: Type::Limit,
      time_in_force: TimeInForce::Day,
      limit_price: Some(Num::from_int(1000)),
      stop_price: None,
      extended_hours: false,
    };
    let result = client.issue::<Post>(request).await;
    let err = result.unwrap_err();

    match err {
      PostError::InsufficientFunds(_) => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
    Ok(())
  }

  #[test(tokio::test)]
  async fn cancel_invalid_order() -> Result<(), Error> {
    let id = Id(Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap());
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let result = client.issue::<Delete>(id).await;
    let err = result.unwrap_err();

    match err {
      DeleteError::NotFound(_) => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
    Ok(())
  }

  #[test(tokio::test)]
  async fn retrieve_order_by_id() -> Result<(), Error> {
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let posted = order_aapl(&client).await?;
    let result = client.issue::<Get>(posted.id).await;
    let _ = client
      .issue::<Delete>(posted.id)
      .await
      .map_err(EndpointError::from)?;
    let gotten = result.map_err(EndpointError::from)?;

    // We can't simply compare the two orders for equality, because some
    // time stamps may differ.
    assert_eq!(posted.id, gotten.id);
    assert_eq!(posted.status, gotten.status);
    assert_eq!(posted.asset_class, gotten.asset_class);
    assert_eq!(posted.asset_id, gotten.asset_id);
    assert_eq!(posted.symbol, gotten.symbol);
    assert_eq!(posted.quantity, gotten.quantity);
    assert_eq!(posted.type_, gotten.type_);
    assert_eq!(posted.side, gotten.side);
    assert_eq!(posted.time_in_force, gotten.time_in_force);
    Ok(())
  }

  #[test(tokio::test)]
  async fn retrieve_non_existent_order() -> Result<(), Error> {
    let id = Id(Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap());
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let result = client.issue::<Get>(id).await;
    let err = result.unwrap_err();

    match err {
      GetError::NotFound(_) => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
    Ok(())
  }

  #[test(tokio::test)]
  async fn extended_hours_market_order() -> Result<(), Error> {
    let request = OrderReq {
      symbol: Symbol::SymExchgCls("SPY".to_string(), Exchange::Arca, Class::UsEquity),
      quantity: 1,
      side: Side::Buy,
      type_: Type::Market,
      time_in_force: TimeInForce::Day,
      limit_price: None,
      stop_price: None,
      extended_hours: true,
    };
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);

    // We are submitting a market order with extended_hours, that is
    // invalid as per the Alpaca documentation.
    let result = client.issue::<Post>(request).await;
    let err = result.unwrap_err();

    match err {
      PostError::InvalidInput(_) => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
    Ok(())
  }

  #[test(tokio::test)]
  async fn change_order() -> Result<(), Error> {
    let request = OrderReq {
      symbol: Symbol::SymExchgCls("AAPL".to_string(), Exchange::Nasdaq, Class::UsEquity),
      quantity: 1,
      side: Side::Buy,
      type_: Type::Limit,
      time_in_force: TimeInForce::Day,
      limit_price: Some(Num::from_int(1)),
      stop_price: None,
      extended_hours: false,
    };

    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let order = client
      .issue::<Post>(request)
      .await
      .map_err(EndpointError::from)?;

    let request = ChangeReq {
      quantity: 2,
      time_in_force: TimeInForce::UntilCanceled,
      limit_price: Some(Num::from_int(2)),
      stop_price: None,
    };
    let result = client
      .issue::<Patch>((order.id, request))
      .await
      .map_err(EndpointError::from);

    client
      .issue::<Delete>(order.id)
      .await
      .map_err(EndpointError::from)?;

    let order = result?;
    assert_eq!(order.quantity, Num::from_int(2));
    assert_eq!(order.time_in_force, TimeInForce::UntilCanceled);
    assert_eq!(order.limit_price, Some(Num::from_int(2)));
    assert_eq!(order.stop_price, None);
    Ok(())
  }
}
