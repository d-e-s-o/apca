// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Serialize;

use url::form_urlencoded::Serializer;

use crate::api::v2::order::Order;
use crate::endpoint::Endpoint;
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


/// The representation of a GET request to the /v2/orders endpoint.
#[derive(Clone, Copy, Debug)]
pub struct Get {}

EndpointDef! {
  Get,
  Ok => Vec<Order>, [
    /// The list of orders was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []
}

impl Endpoint for Get {
  type Input = OrdersReq;
  type Output = Vec<Order>;
  type Error = GetError;

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

  use test_env_log::test;

  use crate::api::v1::order_util::order_aapl;
  use crate::api::v2::order;
  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::Error;


  #[test(tokio::test)]
  async fn list_orders() -> Result<(), Error> {
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let request = OrdersReq { limit: 50 };

    // Holy fucking shit!! We need to get the order ID passed through to
    // the various futures. We cannot just close over it from the outer
    // scope for lifetime conflicts. We also can't just use move
    // closures because that moves the client object as well. So we end
    // up with this dance to pass the order ID through the pipeline.
    let order = order_aapl(&client).await?;
    let result = client.issue::<Get>(request.clone()).await;
    let _ = client.issue::<order::Delete>(order.id).await?;
    let before = result?;
    let after = client.issue::<Get>(request.clone()).await?;

    let before = Into::<Vec<_>>::into(before);
    let after = Into::<Vec<_>>::into(after);

    assert!(before.into_iter().find(|x| x.id == order.id).is_some());
    assert!(after.into_iter().find(|x| x.id == order.id).is_none());
    Ok(())
  }
}
