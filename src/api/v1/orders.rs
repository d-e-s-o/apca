// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Serialize;

use url::form_urlencoded::Serializer;

use crate::api::v1::order::Order;
use crate::endpoint::Endpoint;
use crate::Str;


/// A GET request to be made to the /v1/orders endpoint.
// Note that we do not expose or supply all parameters that the Alpaca
// API supports.
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct OrdersReq {
  /// The maximum number of orders in response. Defaults to 50 and max
  /// is 500.
  #[serde(rename = "limit")]
  pub limit: u64,
}


/// The representation of a GET request to the /v1/orders endpoint.
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
    "/v1/orders".into()
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

  use futures::future::Future;
  use futures::future::ok;

  use test_env_log::test;

  use tokio::runtime::current_thread::block_on_all;

  use crate::api::v1::order;
  use crate::api::v1::order_util::ClientExt;
  use crate::Client;
  use crate::Error;


  #[test]
  fn list_orders() -> Result<(), Error> {
    let client = Client::from_env()?;
    let request = OrdersReq { limit: 50 };

    // Holy fucking shit!! We need to get the order ID passed through to
    // the various futures. We cannot just close over it from the outer
    // scope for lifetime conflicts. We also can't just use move
    // closures because that moves the client object as well. So we end
    // up with this dance to pass the order ID through the pipeline.
    let future = client.order_aapl()?.map_err(Error::from).and_then(|order| {
      ok(order.id)
        .join({
          client
            .issue::<Get>(request.clone())
            .unwrap()
            .map_err(Error::from)
        })
        .then(|res| {
          let (id, res) = res.unwrap();
          ok((id, res)).join({
            client
              .issue::<order::Delete>(id)
              .unwrap()
              .map_err(Error::from)
          })
        })
        .and_then(|res| {
          let (id, before) = res;
          ok((id, before)).join(client.issue::<Get>(request).unwrap().map_err(Error::from))
        })
    });

    let (((id, before), _), after) = block_on_all(future)?;
    let before = Into::<Vec<_>>::into(before);
    let after = Into::<Vec<_>>::into(after);

    assert!(before.into_iter().find(|x| x.id == id).is_some());
    assert!(after.into_iter().find(|x| x.id == id).is_none());
    Ok(())
  }
}
