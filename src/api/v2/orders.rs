// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Serialize;

use url::form_urlencoded::Serializer;

use crate::api::v2::order::Order;
use crate::Str;


/// A GET request to be made to the /v2/orders endpoint.
// Note that we do not expose or supply all parameters that the Alpaca
// API supports.
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct OrdersReq {
  /// The maximum number of orders in response. Defaults to 50 and max
  /// is 500.
  #[serde(rename = "limit")]
  pub limit: u64,
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
    let query = Serializer::new(String::new())
      .append_pair("limit", &input.limit.to_string())
      .finish();

    Some(query.into())
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use http_endpoint::Error as EndpointError;

  use test_env_log::test;

  use crate::api::v2::order;
  use crate::api::v2::order_util::order_aapl;
  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::Error;


  #[test(tokio::test)]
  async fn list_orders() -> Result<(), Error> {
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let request = OrdersReq { limit: 50 };

    let order = order_aapl(&client).await?;
    let result = client.issue::<Get>(request.clone()).await;
    let _ = client
      .issue::<order::Delete>(order.id)
      .await
      .map_err(EndpointError::from)?;
    let before = result.map_err(EndpointError::from)?;
    let after = client
      .issue::<Get>(request.clone())
      .await
      .map_err(EndpointError::from)?;

    let before = Into::<Vec<_>>::into(before);
    let after = Into::<Vec<_>>::into(after);

    assert!(before.into_iter().find(|x| x.id == order.id).is_some());
    assert!(after.into_iter().find(|x| x.id == order.id).is_none());
    Ok(())
  }
}
