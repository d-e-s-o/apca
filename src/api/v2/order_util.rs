// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use http_endpoint::Error as EndpointError;

use num_decimal::Num;

use crate::api::v2::order;
use crate::api::v2::order::Side;
use crate::api::v2::order::Type;
use crate::Client;

pub async fn order_aapl(client: &Client) -> Result<order::Order, EndpointError> {
  let request = order::OrderReqInit {
    type_: Type::Limit,
    limit_price: Some(Num::from(1)),
    ..Default::default()
  }
  .init("AAPL", Side::Buy, 1);

  client
    .issue::<order::Post>(request)
    .await
    .map_err(EndpointError::from)
}
