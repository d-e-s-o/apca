// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

pub use crate::api::v1::position::Position;
pub use crate::api::v1::position::PositionReq;
pub use crate::api::v1::position::Side;

use crate::endpoint::Endpoint;
use crate::Str;


/// The representation of a GET request to the /v2/positions/<symbol>
/// endpoint.
#[derive(Clone, Copy, Debug)]
pub struct Get {}

EndpointDef! {
  Get,
  Ok => Position, [
    /// The position with the given ID was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// No position was found with the given ID.
    /* 404 */ NOT_FOUND => NotFound,
  ]
}

impl Endpoint for Get {
  type Input = PositionReq;
  type Output = Position;
  type Error = GetError;

  fn path(input: &Self::Input) -> Str {
    format!("/v2/positions/{}", input.symbol).into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use test_env_log::test;

  use crate::api::v1::asset;
  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::Error;


  #[test(tokio::test)]
  async fn retrieve_position() -> Result<(), Error> {
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let request = PositionReq {
      symbol: asset::Symbol::Sym("SPY".to_string()),
    };
    let result = client.issue::<Get>(request).await;

    // We don't know whether there is an open position and we can't
    // simply create one as the market may be closed. So really the best
    // thing we can do is to make sure that we either get a valid
    // response or an indication that no position has been found.
    match result {
      Ok(pos) => {
        assert_eq!(pos.symbol, "SPY");
        assert_eq!(pos.asset_class, asset::Class::UsEquity);
      },
      Err(err) => match err {
        GetError::NotFound(_) => (),
        _ => panic!("Received unexpected error: {:?}", err),
      },
    }
    Ok(())
  }
}
