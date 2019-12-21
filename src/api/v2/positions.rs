// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::api::v2::position::Position;
use crate::endpoint::Endpoint;
use crate::Str;


/// The representation of a GET request to the /v2/positions endpoint.
#[derive(Clone, Copy, Debug)]
pub struct Get {}

EndpointDef! {
  Get,
  Ok => Vec<Position>, [
    /// The list of positions was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [ ]
}

impl Endpoint for Get {
  type Input = ();
  type Output = Vec<Position>;
  type Error = GetError;

  fn path(_input: &Self::Input) -> Str {
    "/v2/positions".into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use test_env_log::test;

  use tokio01::runtime::current_thread::block_on_all;

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::Error;


  #[test]
  fn list_positions() -> Result<(), Error> {
    // We can't do much here except check that the request is not
    // reporting any errors.
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info)?;
    let future = client.issue::<Get>(())?;
    let _ = block_on_all(future)?;
    Ok(())
  }
}
