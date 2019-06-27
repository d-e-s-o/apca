// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use futures::future::Future;
use futures::future::ok;

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

type BoxFut<T, E> = Box<dyn Future<Item = T, Error = E>>;

/// An extension trait for the `Client` that provides convenient
/// access to often required functionality for testing purposes.
pub trait ClientExt {
  /// Submit an unsatisfiable limit order for one share of AAPL.
  fn order_aapl(&self) -> Result<BoxFut<order::Order, order::PostError>, Error>;

  /// Safely cancel an order, panicking on error.
  fn cancel_order(&self, id: order::Id) -> BoxFut<(), ()>;
}

impl ClientExt for Client {
  fn order_aapl(&self) -> Result<BoxFut<order::Order, order::PostError>, Error> {
    let request = order::OrderReq {
      symbol: Symbol::SymExchgCls("AAPL".to_string(), Exchange::Nasdaq, Class::UsEquity),
      quantity: 1,
      side: Side::Buy,
      type_: Type::Limit,
      time_in_force: TimeInForce::Day,
      limit_price: Some(Num::from_int(1)),
      stop_price: None,
    };
    self.issue::<order::Post>(request).map(|x| Box::new(x) as _)
  }

  fn cancel_order(&self, id: order::Id) -> BoxFut<(), ()> {
    let fut = self.issue::<order::Delete>(id).unwrap().then(|x| {
      let _ = x.unwrap();
      ok(())
    });
    Box::new(fut)
  }
}
