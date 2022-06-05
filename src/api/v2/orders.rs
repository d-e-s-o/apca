// Copyright (C) 2019-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;
use serde::Serialize;
use serde_urlencoded::to_string as to_query;

use crate::api::v2::order::Order;
use crate::util::string_slice_to_str;
use crate::util::vec_from_comma_separated_str;
use crate::Str;

/// The status of orders to list.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum Status {
  /// List open orders only.
  #[serde(rename = "open")]
  Open,
  /// List closed orders only.
  #[serde(rename = "closed")]
  Closed,
  /// List all orders.
  #[serde(rename = "all")]
  All,
}


/// A GET request to be made to the /v2/orders endpoint.
// Note that we do not expose or supply all parameters that the Alpaca
// API supports.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OrdersReq {
  /// A list of simple symbols used as filters for the returned orders.
  #[serde(
    rename = "symbols",
    default,
    deserialize_with = "vec_from_comma_separated_str",
    serialize_with = "string_slice_to_str"
  )]
  pub symbols: Vec<String>,
  /// The status of orders to list.
  #[serde(rename = "status")]
  pub status: Status,
  /// The maximum number of orders in response. Defaults to 50 and max
  /// is 500.
  #[serde(rename = "limit")]
  pub limit: Option<usize>,
  /// If false the result will not roll up multi-leg orders under the
  /// legs field of the primary order.
  #[serde(rename = "nested")]
  pub nested: bool,
}

impl Default for OrdersReq {
  fn default() -> Self {
    Self {
      symbols: Vec::new(),
      status: Status::Open,
      limit: None,
      // Nested orders merely appear as legs in each order being
      // returned. As such, having them included is very non-intrusive
      // and should be a reasonable default.
      nested: true,
    }
  }
}


Endpoint! {
  /// The representation of a GET request to the /v2/orders endpoint.
  pub Get(OrdersReq),
  Ok => Vec<Order>, [
    /// The list of orders was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  #[inline]
  fn path(_input: &Self::Input) -> Str {
    "/v2/orders".into()
  }

  fn query(input: &Self::Input) -> Result<Option<Str>, Self::ConversionError> {
    Ok(Some(to_query(input)?.into()))
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use futures::future::ok;
  use futures::pin_mut;
  use futures::StreamExt;
  use futures::TryStreamExt;

  use num_decimal::Num;

  use serde_json::from_slice as from_json;
  use serde_json::to_vec as to_json;
  use serde_urlencoded::from_str as from_query;
  use serde_urlencoded::to_string as to_query;

  use test_log::test;

  use crate::api::v2::order;
  use crate::api::v2::order_util::order_aapl;
  use crate::api::v2::order_util::order_stock;
  use crate::api::v2::updates;
  use crate::api_info::ApiInfo;
  use crate::Client;


  /// Make sure that we can serialize and deserialize an `OrdersReq`.
  #[test]
  fn serialize_deserialize_request() {
    let mut request = OrdersReq {
      symbols: vec!["ABC".into()],
      status: Status::Closed,
      limit: Some(42),
      nested: true,
    };

    let json = to_json(&request).unwrap();
    assert_eq!(from_json::<OrdersReq>(&json).unwrap(), request);

    request.symbols.clear();
    let json = to_json(&request).unwrap();
    assert_eq!(from_json::<OrdersReq>(&json).unwrap(), request);
  }

  /// Make sure that we can serialize and deserialize an `OrdersReq`
  /// from a query string.
  #[test]
  fn serialize_deserialize_query_request() {
    let mut request = OrdersReq {
      symbols: vec!["ABC".into()],
      status: Status::Closed,
      limit: Some(42),
      nested: true,
    };

    let query = to_query(&request).unwrap();
    assert_eq!(from_query::<OrdersReq>(&query).unwrap(), request);

    request.symbols.clear();
    let query = to_query(&request).unwrap();
    assert_eq!(from_query::<OrdersReq>(&query).unwrap(), request);
  }

  /// Cancel an order and wait for the corresponding cancellation event
  /// to arrive.
  async fn cancel_order(client: &Client, id: order::Id) {
    let (stream, _subscription) = client.subscribe::<updates::OrderUpdates>().await.unwrap();
    pin_mut!(stream);

    client.issue::<order::Delete>(&id).await.unwrap();

    // Wait until we see the "canceled" event.
    let _update = stream
      .try_filter_map(|res| {
        let update = res.unwrap();
        ok(Some(update))
      })
      // There could be other orders happening concurrently but we are
      // only interested in ones belonging to the order canceled
      // earlier.
      .try_skip_while(|update| {
        ok(update.order.id != id || !matches!(update.event, updates::OrderStatus::Canceled))
      })
      .next()
      .await
      .unwrap()
      .unwrap();
  }

  /// Check that we can list existing orders.
  #[test(tokio::test)]
  async fn list_orders() {
    async fn test(status: Status) {
      let api_info = ApiInfo::from_env().unwrap();
      let client = Client::new(api_info);
      let request = OrdersReq {
        status,
        ..Default::default()
      };

      let order = order_aapl(&client).await.unwrap();
      let result = client.issue::<Get>(&request).await;
      cancel_order(&client, order.id).await;

      let before = result.unwrap();
      let after = client.issue::<Get>(&request).await.unwrap();

      let before = Into::<Vec<_>>::into(before);
      let after = Into::<Vec<_>>::into(after);

      match status {
        Status::Open => {
          assert!(before.into_iter().any(|x| x.id == order.id));
          assert!(!after.into_iter().any(|x| x.id == order.id));
        },
        Status::Closed => {
          assert!(!before.into_iter().any(|x| x.id == order.id));
          assert!(after.into_iter().any(|x| x.id == order.id));
        },
        Status::All => {
          assert!(before.into_iter().any(|x| x.id == order.id));
          assert!(after.into_iter().any(|x| x.id == order.id));
        },
      }
    }

    test(Status::Open).await;
    test(Status::Closed).await;
    test(Status::All).await;
  }

  /// Verify that we can list nested orders.
  #[test(tokio::test)]
  async fn list_nested_order() {
    let request = order::OrderReqInit {
      class: order::Class::OneTriggersOther,
      type_: order::Type::Limit,
      limit_price: Some(Num::from(2)),
      take_profit: Some(order::TakeProfit::Limit(Num::from(3))),
      ..Default::default()
    }
    .init("SPY", order::Side::Buy, order::Amount::quantity(1));

    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let order = client.issue::<order::Post>(&request).await.unwrap();
    assert_eq!(order.legs.len(), 1);

    let request = OrdersReq {
      status: Status::Open,
      ..Default::default()
    };
    let list = client.issue::<Get>(&request).await.unwrap();
    client.issue::<order::Delete>(&order.id).await.unwrap();

    let mut filtered = list.into_iter().filter(|o| o.id == order.id);
    let listed = filtered.next().unwrap();
    assert_eq!(listed.legs.len(), 1);
    // There shouldn't be any other orders with the given ID.
    assert_eq!(filtered.next(), None);
  }

  /// Test that orders can be correctly filtered by a list of symbols.
  #[test(tokio::test)]
  async fn symbol_filter_orders() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    // Get the number of current open orders and the number of open GOOG
    // orders. This allows the test to function based on the current
    // state of the account rather than requiring preconditions to be
    // met.
    let request = OrdersReq::default();
    let orders = client.issue::<Get>(&request).await.unwrap();
    let num_goog = orders.iter().filter(|x| x.symbol == "GOOG").count();
    let num_ibm = orders.iter().filter(|x| x.symbol == "IBM").count();

    let buy_order = order_stock(&client, "GOOG")
      .await
      .expect("Failed to create GOOG order");
    let request = OrdersReq {
      symbols: vec!["IBM".to_string()],
      ..Default::default()
    };
    let ibm_orders = client.issue::<Get>(&request).await;
    let request = OrdersReq {
      symbols: vec!["GOOG".to_string()],
      ..Default::default()
    };
    let goog_orders = client.issue::<Get>(&request).await;

    cancel_order(&client, buy_order.id).await;

    assert_eq!(ibm_orders.unwrap().len(), num_ibm);
    assert_eq!(goog_orders.unwrap().len(), num_goog + 1);
  }
}
