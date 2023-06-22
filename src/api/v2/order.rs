// Copyright (C) 2019-2023 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ops::Deref;
use std::ops::Not;

use chrono::DateTime;
use chrono::Utc;

use http::Method;
use http_endpoint::Bytes;

use num_decimal::Num;

use serde::de::IntoDeserializer;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde_json::from_slice as from_json;
use serde_json::to_vec as to_json;
use serde_urlencoded::to_string as to_query;

use uuid::Uuid;

use crate::api::v2::asset;
use crate::util::vec_from_str;
use crate::Str;


/// An ID uniquely identifying an order.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Id(pub Uuid);

impl Deref for Id {
  type Target = Uuid;

  #[inline]
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}


/// The status an order can have.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
  /// The order is awaiting replacement.
  #[serde(rename = "pending_replace")]
  PendingReplace,
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
  /// The order is still being held. This may be the case for legs of
  /// bracket-style orders that are not active yet because the primary
  /// order has not filled yet.
  #[serde(rename = "held")]
  Held,
  /// Any other status that we have not accounted for.
  ///
  /// Note that having any such status should be considered a bug.
  #[serde(other, rename(serialize = "unknown"))]
  Unknown,
}

impl Status {
  /// Check whether the status is terminal, i.e., no more changes will
  /// occur to the associated order.
  #[inline]
  pub fn is_terminal(self) -> bool {
    matches!(
      self,
      Self::Replaced | Self::Filled | Self::Canceled | Self::Expired | Self::Rejected
    )
  }
}


/// The side an order is on.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Side {
  /// Buy an asset.
  #[serde(rename = "buy")]
  Buy,
  /// Sell an asset.
  #[serde(rename = "sell")]
  Sell,
}

impl Not for Side {
  type Output = Self;

  #[inline]
  fn not(self) -> Self::Output {
    match self {
      Self::Buy => Self::Sell,
      Self::Sell => Self::Buy,
    }
  }
}


/// The class an order belongs to.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Class {
  /// Any non-bracket order (i.e., regular market, limit, or stop loss
  /// orders).
  #[serde(rename = "simple")]
  Simple,
  /// A bracket order is a chain of three orders that can be used to manage your
  /// position entry and exit. It is a common use case of an
  /// one-triggers & one-cancels-other order.
  #[serde(rename = "bracket")]
  Bracket,
  /// A One-cancels-other is a set of two orders with the same side
  /// (buy/buy or sell/sell) and currently only exit order is supported.
  /// Such an order can be used to add two legs to an already filled
  /// order.
  #[serde(rename = "oco")]
  OneCancelsOther,
  /// A one-triggers-other order that can either have a take-profit or
  /// stop-loss leg set. It essentially attached a single leg to an
  /// entry order.
  #[serde(rename = "oto")]
  OneTriggersOther,
}

impl Default for Class {
  #[inline]
  fn default() -> Self {
    Self::Simple
  }
}


/// The type of an order.
// Note that we currently do not support `stop_limit` orders.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
  /// A trailing stop order.
  #[serde(rename = "trailing_stop")]
  TrailingStop,
}

impl Default for Type {
  #[inline]
  fn default() -> Self {
    Self::Market
  }
}


/// A description of the time for which an order is valid.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimeInForce {
  /// The order is good for the day, and it will be canceled
  /// automatically at the end of Regular Trading Hours if unfilled.
  #[serde(rename = "day")]
  Day,
  /// The order is only executed if the entire order quantity can
  /// be filled, otherwise the order is canceled.
  #[serde(rename = "fok")]
  FillOrKill,
  /// The order requires all or part of the order to be executed
  /// immediately. Any unfilled portion of the order is canceled.
  #[serde(rename = "ioc")]
  ImmediateOrCancel,
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

impl Default for TimeInForce {
  #[inline]
  fn default() -> Self {
    Self::Day
  }
}


#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "take_profit")]
struct TakeProfitSerde {
  #[serde(rename = "limit_price")]
  limit_price: Num,
}


/// The take profit part of a bracket, one-cancels-other, or
/// one-triggers-other order.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(from = "TakeProfitSerde", into = "TakeProfitSerde")]
pub enum TakeProfit {
  /// The limit price to use.
  Limit(Num),
}

impl From<TakeProfitSerde> for TakeProfit {
  fn from(other: TakeProfitSerde) -> Self {
    Self::Limit(other.limit_price)
  }
}

impl From<TakeProfit> for TakeProfitSerde {
  fn from(other: TakeProfit) -> Self {
    match other {
      TakeProfit::Limit(limit_price) => Self { limit_price },
    }
  }
}


#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "stop_loss")]
struct StopLossSerde {
  #[serde(rename = "stop_price")]
  stop_price: Num,
  #[serde(rename = "limit_price", skip_serializing_if = "Option::is_none")]
  limit_price: Option<Num>,
}


/// The stop loss part of a bracket, one-cancels-other, or
/// one-triggers-other order.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(from = "StopLossSerde", into = "StopLossSerde")]
pub enum StopLoss {
  /// The stop loss price to use.
  Stop(Num),
  /// The stop loss and stop limit price to use.
  StopLimit(Num, Num),
}

impl From<StopLossSerde> for StopLoss {
  fn from(other: StopLossSerde) -> Self {
    if let Some(limit_price) = other.limit_price {
      Self::StopLimit(other.stop_price, limit_price)
    } else {
      Self::Stop(other.stop_price)
    }
  }
}

impl From<StopLoss> for StopLossSerde {
  fn from(other: StopLoss) -> Self {
    match other {
      StopLoss::Stop(stop_price) => Self {
        stop_price,
        limit_price: None,
      },
      StopLoss::StopLimit(stop_price, limit_price) => Self {
        stop_price,
        limit_price: Some(limit_price),
      },
    }
  }
}


/// An abstraction to be able to handle orders in both notional and quantity units.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Amount {
  /// Wrapper for the quantity field.
  Quantity {
    /// A number of shares to order. This can be a fractional number if
    /// trading fractionals or a whole number if not.
    #[serde(rename = "qty")]
    quantity: Num,
  },
  /// Wrapper for the notional field.
  Notional {
    /// A dollar amount to use for the order. This can result in
    /// fractional quantities.
    #[serde(rename = "notional")]
    notional: Num,
  },
}

impl Amount {
  /// Helper method to initialize a quantity.
  #[inline]
  pub fn quantity(amount: impl Into<Num>) -> Self {
    Self::Quantity {
      quantity: amount.into(),
    }
  }

  /// Helper method to initialize a notional.
  #[inline]
  pub fn notional(amount: impl Into<Num>) -> Self {
    Self::Notional {
      notional: amount.into(),
    }
  }
}


/// A helper for initializing `OrderReq` objects.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OrderReqInit {
  /// See `OrderReq::class`.
  pub class: Class,
  /// See `OrderReq::type_`.
  pub type_: Type,
  /// See `OrderReq::time_in_force`.
  pub time_in_force: TimeInForce,
  /// See `OrderReq::limit_price`.
  pub limit_price: Option<Num>,
  /// See `OrderReq::stop_price`.
  pub stop_price: Option<Num>,
  /// See `OrderReq::trail_price`.
  pub trail_price: Option<Num>,
  /// See `OrderReq::trail_percent`.
  pub trail_percent: Option<Num>,
  /// See `OrderReq::take_profit`.
  pub take_profit: Option<TakeProfit>,
  /// See `OrderReq::stop_loss`.
  pub stop_loss: Option<StopLoss>,
  /// See `OrderReq::extended_hours`.
  pub extended_hours: bool,
  /// See `OrderReq::client_order_id`.
  pub client_order_id: Option<String>,
  #[doc(hidden)]
  pub _non_exhaustive: (),
}

impl OrderReqInit {
  /// Create an `OrderReq` from an `OrderReqInit`.
  ///
  /// The provided symbol is assumed to be a "simple" symbol and not any
  /// of the composite forms of the [`Symbol`][asset::Symbol] enum. That
  /// is, it is not being parsed but directly treated as the
  /// [`Sym`][asset::Symbol::Sym] variant.
  pub fn init<S>(self, symbol: S, side: Side, amount: Amount) -> OrderReq
  where
    S: Into<String>,
  {
    OrderReq {
      symbol: asset::Symbol::Sym(symbol.into()),
      amount,
      side,
      class: self.class,
      type_: self.type_,
      time_in_force: self.time_in_force,
      limit_price: self.limit_price,
      stop_price: self.stop_price,
      take_profit: self.take_profit,
      stop_loss: self.stop_loss,
      extended_hours: self.extended_hours,
      client_order_id: self.client_order_id,
      trail_price: self.trail_price,
      trail_percent: self.trail_percent,
    }
  }
}


/// A POST request to be made to the /v2/orders endpoint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OrderReq {
  /// Symbol or asset ID to identify the asset to trade.
  #[serde(rename = "symbol")]
  pub symbol: asset::Symbol,
  /// Amount of shares to trade.
  #[serde(flatten)]
  pub amount: Amount,
  /// The side the order is on.
  #[serde(rename = "side")]
  pub side: Side,
  /// The order class.
  #[serde(rename = "order_class")]
  pub class: Class,
  /// The type of the order.
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
  /// The dollar value away from the high water mark.
  #[serde(rename = "trail_price")]
  pub trail_price: Option<Num>,
  /// The percent value away from the high water mark.
  #[serde(rename = "trail_percent")]
  pub trail_percent: Option<Num>,
  /// Take profit information for bracket-style orders.
  #[serde(rename = "take_profit")]
  pub take_profit: Option<TakeProfit>,
  /// Stop loss information for bracket-style orders.
  #[serde(rename = "stop_loss")]
  pub stop_loss: Option<StopLoss>,
  /// Whether or not the order is eligible to execute during
  /// pre-market/after hours. Note that a value of `true` can only be
  /// combined with limit orders that are good for the day (i.e.,
  /// `TimeInForce::Day`).
  #[serde(rename = "extended_hours")]
  pub extended_hours: bool,
  /// Client unique order ID (free form string).
  ///
  /// This ID is entirely under control of the client, but kept and
  /// passed along by Alpaca. It can be used for associating additional
  /// information with an order, from the client.
  ///
  /// The documented maximum length is 48 characters.
  #[serde(rename = "client_order_id")]
  pub client_order_id: Option<String>,
}


/// A helper for initializing `ChangeReq` objects.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChangeReqInit {
  /// See `ChangeReq::quantity`.
  pub quantity: Option<Num>,
  /// See `ChangeReq::time_in_force`.
  pub time_in_force: Option<TimeInForce>,
  /// See `ChangeReq::limit_price`.
  pub limit_price: Option<Num>,
  /// See `ChangeReq::stop_price`.
  pub stop_price: Option<Num>,
  /// See `ChangeReq::trail`.
  pub trail: Option<Num>,
  /// See `ChangeReq::client_order_id`.
  pub client_order_id: Option<String>,
  #[doc(hidden)]
  pub _non_exhaustive: (),
}

impl ChangeReqInit {
  /// Create an `ChangeReq` from an `ChangeReqInit`.
  pub fn init(self) -> ChangeReq {
    ChangeReq {
      quantity: self.quantity,
      time_in_force: self.time_in_force,
      limit_price: self.limit_price,
      stop_price: self.stop_price,
      trail: self.trail,
      client_order_id: self.client_order_id,
    }
  }
}


/// A PATCH request to be made to the /v2/orders/{order-id} endpoint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChangeReq {
  /// Number of shares to trade.
  #[serde(rename = "qty")]
  pub quantity: Option<Num>,
  /// How long the order will be valid.
  #[serde(rename = "time_in_force")]
  pub time_in_force: Option<TimeInForce>,
  /// The limit price.
  #[serde(rename = "limit_price")]
  pub limit_price: Option<Num>,
  /// The stop price.
  #[serde(rename = "stop_price")]
  pub stop_price: Option<Num>,
  /// The new value of the `trail_price` or `trail_percent` value.
  #[serde(rename = "trail")]
  pub trail: Option<Num>,
  /// Client unique order ID (free form string).
  #[serde(rename = "client_order_id")]
  pub client_order_id: Option<String>,
}


/// A deserialization function for order classes that may be an empty
/// string.
///
/// If the order class is empty, the default one will be used.
fn empty_to_default<'de, D>(deserializer: D) -> Result<Class, D::Error>
where
  D: Deserializer<'de>,
{
  let class = <&str>::deserialize(deserializer)?;
  if class.is_empty() {
    Ok(Class::default())
  } else {
    Class::deserialize(class.into_deserializer())
  }
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
  #[serde(rename = "created_at")]
  pub created_at: DateTime<Utc>,
  /// Timestamp this order was updated at last.
  #[serde(rename = "updated_at")]
  pub updated_at: Option<DateTime<Utc>>,
  /// Timestamp this order was submitted at.
  #[serde(rename = "submitted_at")]
  pub submitted_at: Option<DateTime<Utc>>,
  /// Timestamp this order was filled at.
  #[serde(rename = "filled_at")]
  pub filled_at: Option<DateTime<Utc>>,
  /// Timestamp this order expired at.
  #[serde(rename = "expired_at")]
  pub expired_at: Option<DateTime<Utc>>,
  /// Timestamp this order expired at.
  #[serde(rename = "canceled_at")]
  pub canceled_at: Option<DateTime<Utc>>,
  /// The order's asset class.
  #[serde(rename = "asset_class")]
  pub asset_class: asset::Class,
  /// The ID of the asset represented by the order.
  #[serde(rename = "asset_id")]
  pub asset_id: asset::Id,
  /// The symbol of the asset being traded.
  #[serde(rename = "symbol")]
  pub symbol: String,
  /// The amount being requested.
  #[serde(flatten)]
  pub amount: Amount,
  /// The quantity that was filled.
  #[serde(rename = "filled_qty")]
  pub filled_quantity: Num,
  /// The type of order.
  #[serde(rename = "type")]
  pub type_: Type,
  /// The order class.
  #[serde(rename = "order_class", deserialize_with = "empty_to_default")]
  pub class: Class,
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
  /// The dollar value away from the high water mark.
  #[serde(rename = "trail_price")]
  pub trail_price: Option<Num>,
  /// The percent value away from the high water mark.
  #[serde(rename = "trail_percent")]
  pub trail_percent: Option<Num>,
  /// The average price at which the order was filled.
  #[serde(rename = "filled_avg_price")]
  pub average_fill_price: Option<Num>,
  /// If true, the order is eligible for execution outside regular
  /// trading hours.
  #[serde(rename = "extended_hours")]
  pub extended_hours: bool,
  /// Additional legs of the order.
  ///
  /// Such an additional leg could be, for example, the order for the
  /// take profit part of a bracket-style order.
  #[serde(rename = "legs", deserialize_with = "vec_from_str")]
  pub legs: Vec<Order>,
}


Endpoint! {
  /// The representation of a GET request to the /v2/orders/{order-id}
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
    format!("/v2/orders/{}", input.as_simple()).into()
  }
}


Endpoint! {
  /// The representation of a GET request to the
  /// /v2/orders:by_client_order_id endpoint.
  pub GetByClientId(String),
  Ok => Order, [
    /// The order object for the given ID was retrieved successfully.
    /* 200 */ OK,
  ],
  // TODO: We really should reuse `GetError` as it is defined for the
  //       `Get` endpoint here, but that requires significant changes to
  //       the `http-endpoint` crate.
  Err => GetByClientIdError, [
    /// No order was found with the given client ID.
    /* 404 */ NOT_FOUND => NotFound,
  ]

  #[inline]
  fn path(_input: &Self::Input) -> Str {
    "/v2/orders:by_client_order_id".into()
  }

  fn query(input: &Self::Input) -> Result<Option<Str>, Self::ConversionError> {
    #[derive(Serialize)]
    struct ClientOrderId<'s> {
      #[serde(rename = "client_order_id")]
      order_id: &'s str,
    }

    let order_id = ClientOrderId {
      order_id: input,
    };
    Ok(Some(to_query(order_id)?.into()))
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
    /// The order submission was not permitted. That can have multiple
    /// reasons, including (but not necessarily limited to):
    /// - not enough funds are available
    /// - the order is of a certain order type that cannot be submitted
    ///   at this time of day (e.g., market-open orders must be
    ///   submitted after 7:00pm and before 9:28am and will be rejected
    ///   at other times)
    /* 403 */ FORBIDDEN => NotPermitted,
    /// Some data in the request was invalid.
    /* 422 */ UNPROCESSABLE_ENTITY => InvalidInput,
  ]

  #[inline]
  fn method() -> Method {
    Method::POST
  }

  #[inline]
  fn path(_input: &Self::Input) -> Str {
    "/v2/orders".into()
  }

  fn body(input: &Self::Input) -> Result<Option<Bytes>, Self::ConversionError> {
    let json = to_json(input)?;
    let bytes = Bytes::from(json);
    Ok(Some(bytes))
  }
}


Endpoint! {
  /// The representation of a PATCH request to the /v2/orders/{order-id}
  /// endpoint.
  pub Patch((Id, ChangeReq)),
  Ok => Order, [
    /// The order object for the given ID was changed successfully.
    /* 200 */ OK,
  ],
  Err => PatchError, [
    /// The order change was not permitted. That can have multiple
    /// reasons, including (but not necessarily limited to):
    /// - not enough funds are available
    /// - the order is of a certain order type that cannot be submitted
    ///   at this time of day (e.g., market-open orders must be
    ///   submitted after 7:00pm and before 9:28am and will be rejected
    ///   at other times)
    /* 403 */ FORBIDDEN => NotPermitted,
    /// No order was found with the given ID.
    /* 404 */ NOT_FOUND => NotFound,
    /// Some data in the request was invalid.
    /* 422 */ UNPROCESSABLE_ENTITY => InvalidInput,
  ]

  #[inline]
  fn method() -> Method {
    Method::PATCH
  }

  fn path(input: &Self::Input) -> Str {
    let (id, _) = input;
    format!("/v2/orders/{}", id.as_simple()).into()
  }

  fn body(input: &Self::Input) -> Result<Option<Bytes>, Self::ConversionError> {
    let (_, request) = input;
    let json = to_json(request)?;
    let bytes = Bytes::from(json);
    Ok(Some(bytes))
  }
}


EndpointNoParse! {
  /// The representation of a DELETE request to the /v2/orders/{order-id}
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

  #[inline]
  fn method() -> Method {
    Method::DELETE
  }

  fn path(input: &Self::Input) -> Str {
    format!("/v2/orders/{}", input.as_simple()).into()
  }

  #[inline]
  fn parse(body: &[u8]) -> Result<Self::Output, Self::ConversionError> {
    debug_assert_eq!(body, b"");
    Ok(())
  }

  fn parse_err(body: &[u8]) -> Result<Self::ApiError, Vec<u8>> {
    from_json::<Self::ApiError>(body).map_err(|_| body.to_vec())
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::str::FromStr as _;

  use futures::TryFutureExt;

  use serde_json::from_slice as from_json;

  use test_log::test;

  use uuid::Uuid;

  use crate::api::v2::asset;
  use crate::api::v2::asset::Exchange;
  use crate::api::v2::asset::Symbol;
  use crate::api::v2::order_util::order_aapl;
  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::RequestError;


  /// Check that we can serialize a [`Side`] object.
  #[test]
  fn emit_side() {
    assert_eq!(to_json(&Side::Buy).unwrap(), br#""buy""#);
    assert_eq!(to_json(&Side::Sell).unwrap(), br#""sell""#);
  }

  /// Check that we can properly negate a [`Side`] object.
  #[test]
  fn negate_side() {
    assert_eq!(!Side::Buy, Side::Sell);
    assert_eq!(!Side::Sell, Side::Buy);
  }

  /// Check that we can serialize a [`Type`] object.
  #[test]
  fn emit_type() {
    assert_eq!(to_json(&Type::Market).unwrap(), br#""market""#);
    assert_eq!(to_json(&Type::Limit).unwrap(), br#""limit""#);
    assert_eq!(to_json(&Type::Stop).unwrap(), br#""stop""#);
  }

  /// Make sure that we can serialize and deserialize order legs.
  #[test]
  fn serialize_deserialize_legs() {
    let take_profit = TakeProfit::Limit(Num::new(3, 2));
    let json = to_json(&take_profit).unwrap();
    assert_eq!(json, br#"{"limit_price":"1.5"}"#);
    assert_eq!(from_json::<TakeProfit>(&json).unwrap(), take_profit);

    let stop_loss = StopLoss::Stop(Num::from(42));
    let json = to_json(&stop_loss).unwrap();
    assert_eq!(json, br#"{"stop_price":"42"}"#);
    assert_eq!(from_json::<StopLoss>(&json).unwrap(), stop_loss);

    let stop_loss = StopLoss::StopLimit(Num::from(13), Num::from(96));
    let json = to_json(&stop_loss).unwrap();
    let expected = br#"{"stop_price":"13","limit_price":"96"}"#;
    assert_eq!(json, &expected[..]);
    assert_eq!(from_json::<StopLoss>(&json).unwrap(), stop_loss);
  }

  /// Check that we can parse the `Amount::quantity` variant properly.
  #[test]
  fn parse_quantity_amount() {
    let serialized = br#"{
    "qty": "15"
}"#;
    let amount = from_json::<Amount>(serialized).unwrap();
    assert_eq!(amount, Amount::quantity(15));
  }

  /// Check that we can parse the `Amount::notional` variant properly.
  #[test]
  fn parse_notional_amount() {
    let serialized = br#"{
    "notional": "15.12"
}"#;
    let amount = from_json::<Amount>(serialized).unwrap();
    assert_eq!(amount, Amount::notional(Num::from_str("15.12").unwrap()));
  }

  /// Verify that we can deserialize and serialize a reference order.
  #[test]
  fn deserialize_serialize_reference_order() {
    let json = br#"{
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
    "order_class": "oto",
    "side": "buy",
    "time_in_force": "day",
    "limit_price": "107.00",
    "stop_price": "106.00",
    "filled_avg_price": "106.25",
    "status": "accepted",
    "extended_hours": false,
    "legs": null
}"#;

    let id = Id(Uuid::parse_str("904837e3-3b76-47ec-b432-046db621571b").unwrap());
    let order = from_json::<Order>(&to_json(&from_json::<Order>(json).unwrap()).unwrap()).unwrap();
    assert_eq!(order.id, id);
    assert_eq!(
      order.created_at,
      DateTime::parse_from_rfc3339("2018-10-05T05:48:59Z").unwrap()
    );
    assert_eq!(order.symbol, "AAPL");
    assert_eq!(order.amount, Amount::quantity(15));
    assert_eq!(order.type_, Type::Market);
    assert_eq!(order.class, Class::OneTriggersOther);
    assert_eq!(order.time_in_force, TimeInForce::Day);
    assert_eq!(order.limit_price, Some(Num::from(107)));
    assert_eq!(order.stop_price, Some(Num::from(106)));
    assert_eq!(order.average_fill_price, Some(Num::new(10625, 100)));
  }

  /// Verify that we can deserialize an order with an empty order class.
  ///
  /// Unfortunately, the Alpaca API may return such an empty class for
  /// requests that don't explicitly set the class.
  #[test]
  fn deserialize_order_with_empty_order_class() {
    let json = br#"{
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
    "order_class": "",
    "side": "buy",
    "time_in_force": "day",
    "limit_price": "107.00",
    "stop_price": "106.00",
    "filled_avg_price": "106.25",
    "status": "accepted",
    "extended_hours": false,
    "legs": null
}"#;

    let order = from_json::<Order>(json).unwrap();
    assert_eq!(order.class, Class::Simple);
  }

  /// Check that we can serialize and deserialize an [`OrderReq`].
  #[test]
  fn serialize_deserialize_order_request() {
    let request = OrderReqInit {
      type_: Type::TrailingStop,
      trail_price: Some(Num::from(50)),
      ..Default::default()
    }
    .init("SPY", Side::Buy, Amount::quantity(1));

    let json = to_json(&request).unwrap();
    assert_eq!(from_json::<OrderReq>(&json).unwrap(), request);
  }

  /// Check that we can serialize and deserialize a [`ChangeReq`].
  #[test]
  fn serialize_deserialize_change_request() {
    let request = ChangeReqInit {
      quantity: Some(Num::from(37)),
      time_in_force: Some(TimeInForce::UntilCanceled),
      trail: Some(Num::from(42)),
      ..Default::default()
    }
    .init();

    let json = to_json(&request).unwrap();
    assert_eq!(from_json::<ChangeReq>(&json).unwrap(), request);
  }

  /// Verify that we can submit a limit order.
  #[test(tokio::test)]
  async fn submit_limit_order() {
    async fn test(extended_hours: bool) -> Result<(), RequestError<PostError>> {
      let symbol = Symbol::SymExchgCls("SPY".to_string(), Exchange::Arca, asset::Class::UsEquity);
      let request = OrderReq {
        symbol,
        amount: Amount::quantity(1),
        side: Side::Buy,
        class: Class::default(),
        type_: Type::Limit,
        time_in_force: TimeInForce::default(),
        limit_price: Some(Num::from(1)),
        stop_price: None,
        trail_price: None,
        trail_percent: None,
        take_profit: None,
        stop_loss: None,
        extended_hours,
        client_order_id: None,
      };

      let api_info = ApiInfo::from_env().unwrap();
      let client = Client::new(api_info);

      let order = client.issue::<Post>(&request).await?;
      client.issue::<Delete>(&order.id).await.unwrap();

      assert_eq!(order.symbol, "SPY");
      assert_eq!(order.amount, Amount::quantity(1));
      assert_eq!(order.side, Side::Buy);
      assert_eq!(order.type_, Type::Limit);
      assert_eq!(order.class, Class::default());
      assert_eq!(order.time_in_force, TimeInForce::Day);
      assert_eq!(order.limit_price, Some(Num::from(1)));
      assert_eq!(order.stop_price, None);
      assert_eq!(order.extended_hours, extended_hours);
      Ok(())
    }

    test(false).await.unwrap();

    // When an extended hours order is submitted between 6pm and 8pm,
    // the Alpaca API reports an error:
    // > {"code":42210000,"message":"extended hours orders between 6:00pm
    // >   and 8:00pm is not supported"}
    //
    // So we need to treat this case specially.
    let result = test(true).await;
    match result {
      Ok(()) | Err(RequestError::Endpoint(PostError::NotPermitted(..))) => (),
      err => panic!("unexpected error: {err:?}"),
    };
  }

  /// Check that we can properly submit a trailing stop price order.
  #[test(tokio::test)]
  async fn submit_trailing_stop_price_order() {
    let request = OrderReqInit {
      type_: Type::TrailingStop,
      trail_price: Some(Num::from(50)),
      ..Default::default()
    }
    .init("SPY", Side::Buy, Amount::quantity(1));

    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let order = client.issue::<Post>(&request).await.unwrap();
    client.issue::<Delete>(&order.id).await.unwrap();

    assert_eq!(order.symbol, "SPY");
    assert_eq!(order.amount, Amount::quantity(1));
    assert_eq!(order.side, Side::Buy);
    assert_eq!(order.type_, Type::TrailingStop);
    assert_eq!(order.time_in_force, TimeInForce::Day);
    assert_eq!(order.limit_price, None);
    // We don't check the stop price here. It may be set to a value that
    // we can't know in advance.
    assert_eq!(order.trail_price, Some(Num::from(50)));
    assert_eq!(order.trail_percent, None);
  }

  /// Check that we can properly submit a trailing stop percent order.
  #[test(tokio::test)]
  async fn submit_trailing_stop_percent_order() {
    let request = OrderReqInit {
      type_: Type::TrailingStop,
      trail_percent: Some(Num::from(10)),
      ..Default::default()
    }
    .init("SPY", Side::Buy, Amount::quantity(1));

    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let order = client.issue::<Post>(&request).await.unwrap();
    client.issue::<Delete>(&order.id).await.unwrap();

    assert_eq!(order.symbol, "SPY");
    assert_eq!(order.amount, Amount::quantity(1));
    assert_eq!(order.side, Side::Buy);
    assert_eq!(order.type_, Type::TrailingStop);
    assert_eq!(order.time_in_force, TimeInForce::Day);
    assert_eq!(order.limit_price, None);
    // We don't check the stop price here. It may be set to a value that
    // we can't know in advance.
    assert_eq!(order.trail_price, None);
    assert_eq!(order.trail_percent, Some(Num::from(10)));
  }

  #[test(tokio::test)]
  async fn submit_bracket_order() {
    let request = OrderReqInit {
      class: Class::Bracket,
      type_: Type::Limit,
      limit_price: Some(Num::from(2)),
      take_profit: Some(TakeProfit::Limit(Num::from(3))),
      stop_loss: Some(StopLoss::Stop(Num::from(1))),
      ..Default::default()
    }
    .init("SPY", Side::Buy, Amount::quantity(1));

    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let order = client.issue::<Post>(&request).await.unwrap();
    client.issue::<Delete>(&order.id).await.unwrap();

    for leg in &order.legs {
      client.issue::<Delete>(&leg.id).await.unwrap();
    }

    assert_eq!(order.symbol, "SPY");
    assert_eq!(order.amount, Amount::quantity(1));
    assert_eq!(order.side, Side::Buy);
    assert_eq!(order.type_, Type::Limit);
    assert_eq!(order.class, Class::Bracket);
    assert_eq!(order.time_in_force, TimeInForce::Day);
    assert_eq!(order.limit_price, Some(Num::from(2)));
    assert_eq!(order.stop_price, None);
    assert!(!order.extended_hours);
    assert_eq!(order.legs.len(), 2);
    assert_eq!(order.legs[0].status, Status::Held);
    assert_eq!(order.legs[1].status, Status::Held);
  }

  #[test(tokio::test)]
  async fn submit_one_triggers_other_order() {
    let request = OrderReqInit {
      class: Class::OneTriggersOther,
      type_: Type::Limit,
      limit_price: Some(Num::from(2)),
      stop_loss: Some(StopLoss::Stop(Num::from(1))),
      ..Default::default()
    }
    .init("SPY", Side::Buy, Amount::quantity(1));

    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let order = client.issue::<Post>(&request).await.unwrap();
    client.issue::<Delete>(&order.id).await.unwrap();

    for leg in &order.legs {
      client.issue::<Delete>(&leg.id).await.unwrap();
    }

    assert_eq!(order.symbol, "SPY");
    assert_eq!(order.amount, Amount::quantity(1));
    assert_eq!(order.side, Side::Buy);
    assert_eq!(order.type_, Type::Limit);
    assert_eq!(order.class, Class::OneTriggersOther);
    assert_eq!(order.time_in_force, TimeInForce::Day);
    assert_eq!(order.limit_price, Some(Num::from(2)));
    assert_eq!(order.stop_price, None);
    assert!(!order.extended_hours);
    assert_eq!(order.legs.len(), 1);
    assert_eq!(order.legs[0].status, Status::Held);
  }

  /// Test submission of orders of various time in force types.
  #[test(tokio::test)]
  async fn submit_other_order_types() {
    async fn test(time_in_force: TimeInForce) {
      let api_info = ApiInfo::from_env().unwrap();
      let client = Client::new(api_info);

      let request = OrderReqInit {
        type_: Type::Limit,
        class: Class::Simple,
        time_in_force,
        limit_price: Some(Num::from(1)),
        ..Default::default()
      }
      .init("AAPL", Side::Buy, Amount::quantity(1));

      match client.issue::<Post>(&request).await {
        Ok(order) => {
          client.issue::<Delete>(&order.id).await.unwrap();

          assert_eq!(order.time_in_force, time_in_force);
        },
        // Submission of those orders may fail at certain times of the
        // day as per the Alpaca documentation. So ignore those errors.
        Err(RequestError::Endpoint(PostError::NotPermitted(..))) => (),
        Err(err) => panic!("Received unexpected error: {err:?}"),
      }
    }

    test(TimeInForce::FillOrKill).await;
    test(TimeInForce::ImmediateOrCancel).await;
    test(TimeInForce::UntilMarketOpen).await;
    test(TimeInForce::UntilMarketClose).await;
  }

  /// Check that we see the expected error being reported when
  /// attempting to submit an unsatisfiable order.
  #[test(tokio::test)]
  async fn submit_unsatisfiable_order() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let request = OrderReqInit {
      type_: Type::Limit,
      limit_price: Some(Num::from(1000)),
      ..Default::default()
    }
    .init("AAPL", Side::Buy, Amount::quantity(100_000));

    let result = client.issue::<Post>(&request).await;
    let err = result.unwrap_err();

    match err {
      RequestError::Endpoint(PostError::NotPermitted(..)) => (),
      _ => panic!("Received unexpected error: {err:?}"),
    };
  }

  /// Test that we can submit an order with a notional amount.
  #[test(tokio::test)]
  async fn submit_unsatisfiable_notional_order() {
    let request =
      OrderReqInit::default().init("SPY", Side::Buy, Amount::notional(Num::from(10_000_000)));

    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let result = client.issue::<Post>(&request).await;
    let err = result.unwrap_err();

    match err {
      RequestError::Endpoint(PostError::NotPermitted(..)) => (),
      _ => panic!("Received unexpected error: {err:?}"),
    };
  }

  /// Test that we can submit an order with a fractional quantity.
  #[test(tokio::test)]
  async fn submit_unsatisfiable_fractional_order() {
    let qty = Num::from(1_000_000) + Num::new(1, 2);
    let request = OrderReqInit::default().init("SPY", Side::Buy, Amount::quantity(qty));

    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let result = client.issue::<Post>(&request).await;
    let err = result.unwrap_err();

    match err {
      RequestError::Endpoint(PostError::NotPermitted(..)) => (),
      _ => panic!("Received unexpected error: {err:?}"),
    };
  }

  /// Check that we get back the expected error when attempting to
  /// cancel an invalid (non-existent) order.
  #[test(tokio::test)]
  async fn cancel_invalid_order() {
    let id = Id(Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap());
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let result = client.issue::<Delete>(&id).await;
    let err = result.unwrap_err();

    match err {
      RequestError::Endpoint(DeleteError::NotFound(..)) => (),
      _ => panic!("Received unexpected error: {err:?}"),
    };
  }

  /// Check that we can retrieve an order given its ID.
  #[test(tokio::test)]
  async fn retrieve_order_by_id() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let posted = order_aapl(&client).await.unwrap();
    let result = client.issue::<Get>(&posted.id).await;
    client.issue::<Delete>(&posted.id).await.unwrap();
    let gotten = result.unwrap();

    // We can't simply compare the two orders for equality, because some
    // time stamps as well as the status may differ.
    assert_eq!(posted.id, gotten.id);
    assert_eq!(posted.asset_class, gotten.asset_class);
    assert_eq!(posted.asset_id, gotten.asset_id);
    assert_eq!(posted.symbol, gotten.symbol);
    assert_eq!(posted.amount, gotten.amount);
    assert_eq!(posted.type_, gotten.type_);
    assert_eq!(posted.side, gotten.side);
    assert_eq!(posted.time_in_force, gotten.time_in_force);
  }

  #[test(tokio::test)]
  async fn retrieve_non_existent_order() {
    let id = Id(Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap());
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let result = client.issue::<Get>(&id).await;
    let err = result.unwrap_err();

    match err {
      RequestError::Endpoint(GetError::NotFound(..)) => (),
      _ => panic!("Received unexpected error: {err:?}"),
    };
  }

  #[test(tokio::test)]
  async fn extended_hours_market_order() {
    let request = OrderReqInit {
      extended_hours: true,
      ..Default::default()
    }
    .init("SPY", Side::Buy, Amount::quantity(1));

    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    // We are submitting a market order with extended_hours, that is
    // invalid as per the Alpaca documentation.
    let result = client.issue::<Post>(&request).await;
    let err = result.unwrap_err();

    match err {
      RequestError::Endpoint(PostError::InvalidInput(..)) => (),
      _ => panic!("Received unexpected error: {err:?}"),
    };
  }

  /// Check that we can change an existing order.
  #[test(tokio::test)]
  async fn change_order() {
    let request = OrderReqInit {
      type_: Type::Limit,
      limit_price: Some(Num::from(1)),
      ..Default::default()
    }
    .init("AAPL", Side::Buy, Amount::quantity(1));

    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let order = client.issue::<Post>(&request).await.unwrap();

    let request = ChangeReqInit {
      quantity: Some(Num::from(2)),
      time_in_force: Some(TimeInForce::UntilCanceled),
      limit_price: Some(Num::from(2)),
      ..Default::default()
    }
    .init();

    let result = client.issue::<Patch>(&(order.id, request)).await;
    let id = if let Ok(replaced) = &result {
      replaced.id
    } else {
      order.id
    };

    client.issue::<Delete>(&id).await.unwrap();

    match result {
      Ok(order) => {
        assert_eq!(order.amount, Amount::quantity(2));
        assert_eq!(order.time_in_force, TimeInForce::UntilCanceled);
        assert_eq!(order.limit_price, Some(Num::from(2)));
        assert_eq!(order.stop_price, None);
      },
      Err(RequestError::Endpoint(PatchError::InvalidInput(..))) => {
        // When the market is closed a patch request will never succeed
        // and always report an error along the lines of:
        // "unable to replace order, order isn't sent to exchange yet".
        // We can't do much more than accept this behavior.
      },
      e => panic!("received unexpected error: {e:?}"),
    }
  }

  /// Test changing of a trailing stop order.
  #[test(tokio::test)]
  async fn change_trail_stop_order() {
    let request = OrderReqInit {
      type_: Type::TrailingStop,
      trail_price: Some(Num::from(20)),
      ..Default::default()
    }
    .init("SPY", Side::Buy, Amount::quantity(1));

    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let order = client.issue::<Post>(&request).await.unwrap();
    assert_eq!(order.trail_price, Some(Num::from(20)));

    let request = ChangeReqInit {
      trail: Some(Num::from(30)),
      ..Default::default()
    }
    .init();

    let result = client.issue::<Patch>(&(order.id, request)).await;
    let id = if let Ok(replaced) = &result {
      replaced.id
    } else {
      order.id
    };

    client.issue::<Delete>(&id).await.unwrap();

    match result {
      Ok(order) => {
        assert_eq!(order.trail_price, Some(Num::from(30)));
      },
      Err(RequestError::Endpoint(PatchError::InvalidInput(..))) => (),
      e => panic!("received unexpected error: {e:?}"),
    }
  }

  /// Check that we can submit an order with a custom client order ID
  /// and then retrieve the order object back via this identifier.
  #[test(tokio::test)]
  async fn submit_with_client_order_id() {
    // We need a truly random identifier here, because Alpaca will never
    // forget any client order ID and any ID previously used one cannot
    // be reused again.
    let client_order_id = Uuid::new_v4().as_simple().to_string();

    let request = OrderReqInit {
      type_: Type::Limit,
      limit_price: Some(Num::from(1)),
      client_order_id: Some(client_order_id.clone()),
      ..Default::default()
    }
    .init("SPY", Side::Buy, Amount::quantity(1));

    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let (issued, retrieved) = client
      .issue::<Post>(&request)
      .and_then(|order| async {
        let retrieved = client.issue::<GetByClientId>(&client_order_id).await;
        client.issue::<Delete>(&order.id).await.unwrap();
        Ok((order, retrieved.unwrap()))
      })
      .await
      .unwrap();

    assert_eq!(issued.client_order_id, client_order_id);
    assert_eq!(retrieved.client_order_id, client_order_id);
    assert_eq!(retrieved.id, issued.id);

    // We should not be able to submit another order with the same
    // client ID.
    let err = client.issue::<Post>(&request).await.unwrap_err();

    match err {
      RequestError::Endpoint(PostError::InvalidInput(..)) => (),
      _ => panic!("Received unexpected error: {err:?}"),
    };
  }

  /// Test that we can change the client order ID of an order.
  #[test(tokio::test)]
  async fn change_client_order_id() {
    let request = OrderReqInit {
      type_: Type::Limit,
      limit_price: Some(Num::from(1)),
      ..Default::default()
    }
    .init("SPY", Side::Buy, Amount::quantity(1));

    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let order = client.issue::<Post>(&request).await.unwrap();

    let client_order_id = Uuid::new_v4().as_simple().to_string();
    let request = ChangeReqInit {
      client_order_id: Some(client_order_id.clone()),
      ..Default::default()
    }
    .init();

    let patch_result = client.issue::<Patch>(&(order.id, request)).await;
    let id = if let Ok(replaced) = &patch_result {
      replaced.id
    } else {
      order.id
    };

    let get_result = client.issue::<GetByClientId>(&client_order_id).await;
    let () = client.issue::<Delete>(&id).await.unwrap();

    match patch_result {
      Ok(..) => {
        let order = get_result.unwrap();
        assert_eq!(order.symbol, "SPY");
        assert_eq!(order.type_, Type::Limit);
        assert_eq!(order.limit_price, Some(Num::from(1)));
      },
      Err(RequestError::Endpoint(PatchError::InvalidInput(..))) => (),
      e => panic!("received unexpected error: {e:?}"),
    }
  }
}
