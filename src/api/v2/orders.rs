// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Serialize;
use serde_urlencoded::to_string as to_query;

use crate::api::v2::order::Order;
use crate::Str;


/// The status of orders to list.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
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
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct OrdersReq {
  /// The status of orders to list.
  #[serde(rename = "status")]
  pub status: Status,
  /// The maximum number of orders in response. Defaults to 50 and max
  /// is 500.
  #[serde(rename = "limit")]
  pub limit: u64,
}

impl Default for OrdersReq {
  fn default() -> Self {
    Self {
      status: Status::Open,
      limit: 50,
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

  fn path(_input: &Self::Input) -> Str {
    "/v2/orders".into()
  }

  fn query(input: &Self::Input) -> Option<Str> {
    // TODO: Realistically there should be no way for this unwrap to
    //       ever panic because our conversion to strings should not be
    //       fallible. But still, ideally we would not have to unwrap.
    Some(to_query(input).unwrap().into())
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use test_env_log::test;

  use crate::api::v2::order;
  use crate::api::v2::order_util::order_aapl;
  use crate::api_info::ApiInfo;
  use crate::Client;


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
      let result = client.issue::<Get>(request.clone()).await;
      let _ = client.issue::<order::Delete>(order.id).await.unwrap();
      let before = result.unwrap();
      let after = client.issue::<Get>(request.clone()).await.unwrap();

      let before = Into::<Vec<_>>::into(before);
      let after = Into::<Vec<_>>::into(after);

      match status {
        Status::Open => {
          assert!(before.into_iter().find(|x| x.id == order.id).is_some());
          assert!(after.into_iter().find(|x| x.id == order.id).is_none());
        },
        Status::Closed => {
          assert!(before.into_iter().find(|x| x.id == order.id).is_none());
          assert!(after.into_iter().find(|x| x.id == order.id).is_some());
        },
        Status::All => {
          assert!(before.into_iter().find(|x| x.id == order.id).is_some());
          assert!(after.into_iter().find(|x| x.id == order.id).is_some());
        },
      }
    }

    test(Status::Open).await;
    test(Status::Closed).await;
    test(Status::All).await;
  }
}
