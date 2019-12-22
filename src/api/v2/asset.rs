// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;
use serde::Serialize;

pub use crate::api::v1::asset::Class;
pub use crate::api::v1::asset::Exchange;
pub use crate::api::v1::asset::Id;
pub use crate::api::v1::asset::Status;
pub use crate::api::v1::asset::Symbol;

use crate::endpoint::Endpoint;
use crate::Str;


/// The representation of an asset as used by Alpaca.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Asset {
  /// The asset's ID.
  #[serde(rename = "id")]
  pub id: Id,
  /// The asset's class.
  #[serde(rename = "class")]
  pub class: Class,
  /// The exchange the asset is traded at.
  #[serde(rename = "exchange")]
  pub exchange: Exchange,
  /// The asset's symbol.
  #[serde(rename = "symbol")]
  pub symbol: String,
  /// The asset's status.
  #[serde(rename = "status")]
  pub status: Status,
  /// Whether the asset is tradable on Alpaca or not.
  #[serde(rename = "tradable")]
  pub tradable: bool,
  /// Whether the asset is marginable or not.
  #[serde(rename = "marginable")]
  pub marginable: bool,
  /// Whether the asset is shortable or not.
  #[serde(rename = "shortable")]
  pub shortable: bool,
  /// Whether the asset is considered easy-to-borrow or not.
  ///
  /// A value of `true` is a prerequisite for being able to short it.
  #[serde(rename = "easy_to_borrow")]
  pub easy_to_borrow: bool,
}


/// A GET request to be made to the /v2/assets endpoint.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AssetReq {
  /// The symbol of the asset in question.
  pub symbol: Symbol,
}


/// The representation of a GET request to the /v2/assets/<symbol> endpoint.
#[derive(Clone, Copy, Debug)]
pub struct Get {}

EndpointDef! {
  Get,
  Ok => Asset, [
    /// The asset object for the given symbol was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// No asset was found for the given symbol.
    /* 404 */ NOT_FOUND => NotFound,
  ]
}

impl Endpoint for Get {
  type Input = AssetReq;
  type Output = Asset;
  type Error = GetError;

  fn path(input: &Self::Input) -> Str {
    format!("/v2/assets/{}", input.symbol).into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_str as from_json;

  use test_env_log::test;

  use uuid::Uuid;

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::Error;


  #[test]
  fn parse_reference_asset() {
    let response = r#"{
  "id": "904837e3-3b76-47ec-b432-046db621571b",
  "class": "us_equity",
  "exchange": "NASDAQ",
  "symbol": "AAPL",
  "status": "active",
  "tradable": true,
  "marginable": true,
  "shortable": true,
  "easy_to_borrow": true
}"#;

    let id = Id(Uuid::parse_str("904837e3-3b76-47ec-b432-046db621571b").unwrap());
    let asset = from_json::<Asset>(&response).unwrap();
    assert_eq!(asset.id, id);
    assert_eq!(asset.class, Class::UsEquity);
    assert_eq!(asset.exchange, Exchange::Nasdaq);
    assert_eq!(asset.symbol, "AAPL");
    assert_eq!(asset.status, Status::Active);
    assert_eq!(asset.tradable, true);
    assert_eq!(asset.marginable, true);
    assert_eq!(asset.shortable, true);
    assert_eq!(asset.easy_to_borrow, true);
  }

  #[test(tokio::test)]
  async fn retrieve_asset() -> Result<(), Error> {
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let symbol = Symbol::Sym("SPY".to_string());
    let request = AssetReq { symbol };
    let asset = client.issue::<Get>(request).await?;

    let id = Id(Uuid::parse_str("b28f4066-5c6d-479b-a2af-85dc1a8f16fb").unwrap());
    assert_eq!(asset.id, id);
    assert_eq!(asset.class, Class::UsEquity);
    assert_eq!(asset.exchange, Exchange::Arca);
    assert_eq!(asset.symbol, "SPY");
    assert_eq!(asset.status, Status::Active);
    assert_eq!(asset.tradable, true);
    Ok(())
  }
}
