// Copyright (C) 2019-2021 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use num_decimal::Num;

use crate::api::v2::order;
use crate::api::v2::order::Side;
use crate::api::v2::order::Type;
use crate::Client;
use crate::RequestError;

pub async fn order_aapl(client: &Client) -> Result<order::Order, RequestError<order::PostError>> {
  let request = order::OrderReqInit {
    type_: Type::Limit,
    limit_price: Some(Num::from(1)),
    ..Default::default()
  }
  .init("AAPL", Side::Buy, 1);

  client.issue::<order::Post>(&request).await
}
