// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use num_decimal::Num;

use crate::api::v2::asset::Class;
use crate::api::v2::asset::Exchange;
use crate::api::v2::asset::Symbol;
use crate::api::v2::order;
use crate::api::v2::order::Side;
use crate::api::v2::order::TimeInForce;
use crate::api::v2::order::Type;
use crate::Client;

pub async fn order_aapl(client: &Client) -> Result<order::Order, order::PostError> {
  let request = order::OrderReq {
    symbol: Symbol::SymExchgCls("AAPL".to_string(), Exchange::Nasdaq, Class::UsEquity),
    quantity: 1,
    side: Side::Buy,
    type_: Type::Limit,
    time_in_force: TimeInForce::Day,
    limit_price: Some(Num::from_int(1)),
    stop_price: None,
    extended_hours: false,
  };
  client.issue::<order::Post>(request).await
}
