// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Serialize;

use url::form_urlencoded::Serializer;

use crate::api::v1::asset::Asset;
use crate::api::v1::asset::Class;
use crate::api::v1::asset::Status;
use crate::endpoint::Endpoint;
use crate::Str;


/// A GET request to be made to the /v1/assets endpoint.
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct AssetsReq {
  /// The status of assets to include in the response.
  #[serde(rename = "status")]
  pub status: Status,
  /// The asset class of which to include assets in the response.
  #[serde(rename = "asset_class")]
  pub class: Class,
}


/// The representation of a GET request to the /v1/assets endpoint.
#[derive(Clone, Copy, Debug)]
struct Get {}

EndpointDef! {
  Get,
  Ok => Vec<Asset>, [
    /// The list of assets was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []
}

impl Endpoint for Get {
  type Input = AssetsReq;
  type Output = Vec<Asset>;
  type Error = GetError;

  fn path(_input: &Self::Input) -> Str {
    "/v1/assets".into()
  }

  fn query(input: &Self::Input) -> Option<Str> {
    let query = Serializer::new(String::new())
      .append_pair("status", input.status.as_ref())
      .append_pair("asset_class", input.class.as_ref())
      .finish();

    Some(query.into())
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use test_env_log::test;

  use tokio::runtime::current_thread::block_on_all;

  use crate::api::v1::asset::Exchange;
  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::Error;


  #[test]
  fn list_assets() -> Result<(), Error> {
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info)?;
    let request = AssetsReq {
      status: Status::Active,
      class: Class::UsEquity,
    };
    let future = client.issue::<Get>(request)?;
    let assets = block_on_all(future)?;
    let asset = assets.iter().find(|x| x.symbol == "AAPL").unwrap();
    assert_eq!(asset.class, Class::UsEquity);
    assert_eq!(asset.exchange, Exchange::Nasdaq);
    assert_eq!(asset.status, Status::Active);
    Ok(())
  }
}
