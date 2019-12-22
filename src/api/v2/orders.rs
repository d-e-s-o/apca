// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use url::form_urlencoded::Serializer;

pub use crate::api::v1::orders::OrdersReq;

use crate::api::v2::order::Order;
use crate::endpoint::Endpoint;
use crate::Str;


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

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::Error;


  #[test(tokio::test)]
  async fn list_orders() -> Result<(), Error> {
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let request = OrdersReq { limit: 50 };

    // We merely check that no error is reported.
    let _ = client.issue::<Get>(request).await?;
    Ok(())
  }
}
