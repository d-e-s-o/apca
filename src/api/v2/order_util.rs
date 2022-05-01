// Copyright (C) 2019-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use num_decimal::Num;

use crate::api::v2::order;
use crate::api::v2::order::Amount;
use crate::api::v2::order::Side;
use crate::api::v2::order::Type;
use crate::Client;
use crate::RequestError;


/// Create a limit order for a single share of the stock with the given
/// symbol.
pub(crate) async fn order_stock<S>(
  client: &Client,
  symbol: S,
) -> Result<order::Order, RequestError<order::PostError>>
where
  S: Into<String>,
{
  let request = order::OrderReqInit {
    type_: Type::Limit,
    limit_price: Some(Num::from(1)),
    ..Default::default()
  }
  .init(symbol, Side::Buy, Amount::quantity(1));

  client.issue::<order::Post>(&request).await
}


/// Create a limit order for a single share of AAPL.
pub(crate) async fn order_aapl(
  client: &Client,
) -> Result<order::Order, RequestError<order::PostError>> {
  order_stock(client, "AAPL").await
}
