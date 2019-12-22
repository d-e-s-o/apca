// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use futures01::future::Future;
use futures01::future::ok;

use num_decimal::Num;

use crate::api::v1::asset::Class;
use crate::api::v1::asset::Exchange;
use crate::api::v1::asset::Symbol;
use crate::api::v1::order;
use crate::api::v1::order::Side;
use crate::api::v1::order::TimeInForce;
use crate::api::v1::order::Type;
use crate::Client;
use crate::Error;


pub fn order_aapl(
  client: &Client,
) -> Result<impl Future<Item = order::Order, Error = order::PostError>, Error> {
  let request = order::OrderReq {
    symbol: Symbol::SymExchgCls("AAPL".to_string(), Exchange::Nasdaq, Class::UsEquity),
    quantity: 1,
    side: Side::Buy,
    type_: Type::Limit,
    time_in_force: TimeInForce::Day,
    limit_price: Some(Num::from_int(1)),
    stop_price: None,
  };
  client.issue::<order::Post>(request)
}

pub fn cancel_order(client: &Client, id: order::Id) -> impl Future<Item = (), Error = ()> {
  client.issue::<order::Delete>(id).unwrap().then(|x| {
    let _ = x.unwrap();
    ok(())
  })
}
