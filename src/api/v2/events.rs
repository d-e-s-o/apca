// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::SystemTime;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;

use time_util::optional_system_time_from_str;
use time_util::optional_system_time_to_rfc3339;

use crate::api::v2::account;
use crate::api::v2::order;
use crate::events::EventStream;
use crate::events::StreamType;


/// A representation of an account update that we receive through the
/// "account_updates" stream.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AccountUpdate {
  /// The corresponding account's ID.
  #[serde(rename = "id")]
  pub id: account::Id,
  /// The time the account was created at.
  #[serde(
    rename = "created_at",
    deserialize_with = "optional_system_time_from_str",
    serialize_with = "optional_system_time_to_rfc3339",
  )]
  pub created_at: Option<SystemTime>,
  /// The time the account was updated last.
  #[serde(
    rename = "updated_at",
    deserialize_with = "optional_system_time_from_str",
    serialize_with = "optional_system_time_to_rfc3339",
  )]
  pub updated_at: Option<SystemTime>,
  /// The time the account was deleted at.
  #[serde(
    rename = "deleted_at",
    deserialize_with = "optional_system_time_from_str",
    serialize_with = "optional_system_time_to_rfc3339",
  )]
  pub deleted_at: Option<SystemTime>,
  /// The account's status.
  #[serde(rename = "status")]
  pub status: String,
  /// The currency the account uses.
  #[serde(rename = "currency")]
  pub currency: String,
  /// Cash balance.
  #[serde(rename = "cash")]
  pub cash: Num,
  /// Withdrawable cash amount.
  #[serde(rename = "cash_withdrawable")]
  pub withdrawable_cash: Num,
}


/// A type used for requesting a subscription to the "account_updates"
/// event stream.
#[derive(Clone, Copy, Debug)]
pub enum AccountUpdates {}

impl EventStream for AccountUpdates {
  type Event = AccountUpdate;

  fn stream() -> StreamType {
    StreamType::AccountUpdates
  }
}


/// The status of a trade, as reported as part of a `TradeUpdate`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum TradeStatus {
  /// The order has been received by Alpaca, and routed to exchanges for
  /// execution.
  #[serde(rename = "new")]
  New,
  /// The order has been partially filled.
  #[serde(rename = "partial_fill")]
  PartialFill,
  /// The order has been filled, and no further updates will occur for
  /// the order.
  #[serde(rename = "fill")]
  Filled,
  /// The order is done executing for the day, and will not receive
  /// further updates until the next trading day.
  #[serde(rename = "done_for_day")]
  DoneForDay,
  /// The order has been canceled, and no further updates will occur for
  /// the order.
  #[serde(rename = "canceled")]
  Canceled,
  /// The order has expired, and no further updates will occur.
  #[serde(rename = "expired")]
  Expired,
  /// The order is waiting to be canceled.
  #[serde(rename = "pending_cancel")]
  PendingCancel,
  /// The order has been stopped, and a trade is guaranteed for the
  /// order, usually at a stated price or better, but has not yet
  /// occurred.
  #[serde(rename = "stopped")]
  Stopped,
  /// The order has been rejected, and no further updates will occur for
  /// the order.
  #[serde(rename = "rejected")]
  Rejected,
  /// The order has been suspended, and is not eligible for trading.
  /// This state only occurs on rare occasions.
  #[serde(rename = "suspended")]
  Suspended,
  #[serde(rename = "pending_new")]
  /// The order has been received by Alpaca, and routed to the
  /// exchanges, but has not yet been accepted for execution.
  PendingNew,
  /// The order has been completed for the day (either filled or done
  /// for day), but remaining settlement calculations are still pending.
  #[serde(rename = "calculated")]
  Calculated,
}

impl TradeStatus {
  /// Convert a `TradeStatus` into an `order::Status`.
  pub fn to_order_status(self) -> order::Status {
    match self {
      Self::New => order::Status::New,
      Self::PartialFill => order::Status::PartiallyFilled,
      Self::Filled => order::Status::Filled,
      Self::DoneForDay => order::Status::DoneForDay,
      Self::Canceled => order::Status::Canceled,
      Self::Expired => order::Status::Expired,
      Self::PendingCancel => order::Status::PendingCancel,
      Self::Stopped => order::Status::Stopped,
      Self::Rejected => order::Status::Rejected,
      Self::Suspended => order::Status::Suspended,
      Self::PendingNew => order::Status::PendingNew,
      Self::Calculated => order::Status::Calculated,
    }
  }
}


/// A representation of a trade update the we receive through the
/// "trade_updates" stream.
// TODO: There is also a timestamp field that we may want to hook up.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TradeUpdate {
  /// The event that occurred.
  #[serde(rename = "event")]
  pub event: TradeStatus,
  /// The order associated with the trade.
  #[serde(rename = "order")]
  pub order: order::Order,
}

/// A type used for requesting a subscription to the "trade_updates"
/// event stream.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(unused)]
pub enum TradeUpdates {}

impl EventStream for TradeUpdates {
  type Event = TradeUpdate;

  fn stream() -> StreamType {
    StreamType::TradeUpdates
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use futures::future::ok;
  use futures::StreamExt;
  use futures::TryStreamExt;

  use http_endpoint::Error as EndpointError;

  use serde_json::from_str as from_json;
  use serde_json::to_string as to_json;

  use test_env_log::test;

  use url::Url;

  use crate::api::API_BASE_URL;
  use crate::api::v2::order;
  use crate::api::v2::order_util::order_aapl;
  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::Error;


  #[test]
  fn deserialize_and_serialize_trade_event() {
    let response = r#"{
  "event":"canceled",
  "order":{
    "asset_class":"us_equity",
    "asset_id":"3ece3182-5903-4902-b963-f875a0f416e7",
    "canceled_at":"2020-01-19T06:19:40.137087268Z",
    "client_order_id":"be7d3030-a53e-47ee-9dd3-d9ff3460a174",
    "created_at":"2020-01-19T06:19:34.344561Z",
    "expired_at":null,
    "extended_hours":false,
    "failed_at":null,
    "filled_at":null,
    "filled_avg_price":null,
    "filled_qty":"0",
    "id":"7bb4a536-d59b-4e65-aacf-a8b118d815f4",
    "legs":null,
    "limit_price":"1",
    "order_type":"limit",
    "qty":"1",
    "replaced_at":null,
    "replaced_by":null,
    "replaces":null,
    "side":"buy",
    "status":"canceled",
    "stop_price":null,
    "submitted_at":"2020-01-19T06:19:34.32909Z",
    "symbol":"VMW",
    "time_in_force":"opg",
    "type":"limit",
    "updated_at":"2020-01-19T06:19:40.147946209Z"
  }
}"#;

    // It's hard to compare two JSON objects semantically when all we
    // have is their textual representation (as white spaces may be
    // different and map items reordered). So we just serialize,
    // deserialize, and serialize again, checking that what we
    // ultimately end up with is what we started off with.
    let update = from_json::<TradeUpdate>(&response).unwrap();
    let json = to_json(&update).unwrap();
    let new = from_json::<TradeUpdate>(&json).unwrap();
    assert_eq!(new, update);
  }

  #[test(tokio::test)]
  async fn stream_trade_events() -> Result<(), Error> {
    // TODO: There may be something amiss here. If we don't cancel the
    //       order we never get an event about a new trade. That does
    //       not seem to be in our code, though, as the behavior is the
    //       same when streaming events using Alpaca's Python client.
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let stream = client.subscribe::<TradeUpdates>().await?;
    let order = order_aapl(&client).await?;
    let _ = client
      .issue::<order::Delete>(order.id)
      .await
      .map_err(EndpointError::from)?;

    // Unfortunately due to various braindeadnesses on the Rust &
    // futures side of things there is no sane way for us to provide a
    // stream that implements `Unpin`, which is a requirement for
    // `next`. Given that this is a test we just fudge that by pinning
    // the stream on the heap.
    let trade = Box::pin(stream)
      .try_filter_map(|res| {
        assert!(res.is_ok(), "error: {:?}", res.unwrap_err());
        ok(res.ok())
      })
      // There could be other trades happening concurrently but we
      // are only interested in ones belonging to the order we
      // submitted as part of this test.
      .try_skip_while(|trade| ok(trade.order.id != order.id))
      .next()
      .await
      .unwrap()?;

    assert_eq!(order.id, trade.order.id);
    assert_eq!(order.asset_id, trade.order.asset_id);
    assert_eq!(order.symbol, trade.order.symbol);
    assert_eq!(order.asset_class, trade.order.asset_class);
    assert_eq!(order.type_, trade.order.type_);
    assert_eq!(order.side, trade.order.side);
    assert_eq!(order.time_in_force, trade.order.time_in_force);
    Ok(())
  }

  #[test(tokio::test)]
  async fn stream_with_invalid_credentials() -> Result<(), Error> {
    let api_base = Url::parse(API_BASE_URL)?;
    let api_info = ApiInfo {
      base_url: api_base,
      key_id: "invalid".to_string(),
      secret: "invalid-too".to_string(),
    };

    let client = Client::new(api_info);
    let result = client.subscribe::<TradeUpdates>().await;

    match result {
      Ok(_) => panic!("operation succeeded unexpectedly"),
      Err(Error::Str(ref e)) if e == "authentication not successful" => (),
      Err(e) => panic!("received unexpected error: {}", e),
    }
    Ok(())
  }
}
