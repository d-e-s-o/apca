// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use futures::future::Future;

use num_decimal::Num;

use crate::api::v1::order;
use crate::api::v1::order::Side;
use crate::api::v1::order::TimeInForce;
use crate::api::v1::order::Type;
use crate::Error;
use crate::Requestor;

type BoxFut<T, E> = Box<dyn Future<Item = T, Error = E>>;

/// An extension trait for the `Requestor` that provides convenient
/// access to often required functionality for testing purposes.
pub trait RequestorExt {
  /// Submit an unsatisfiable limit order for one share of AAPL.
  fn order_aapl(&self) -> Result<BoxFut<order::PostOk, order::PostError>, Error>;
}

impl RequestorExt for Requestor {
  fn order_aapl(&self) -> Result<BoxFut<order::PostOk, order::PostError>, Error> {
    let request = order::OrderReq {
      symbol: "AAPL:NASDAQ:us_equity".to_string(),
      quantity: 1,
      side: Side::Buy,
      type_: Type::Limit,
      time_in_force: TimeInForce::Day,
      limit_price: Some(Num::from_int(1)),
      stop_price: None,
    };
    self.issue::<order::Post>(request).map(|x| Box::new(x) as _)
  }
}
