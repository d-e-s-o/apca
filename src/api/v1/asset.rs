// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ops::Deref;

use serde::Deserialize;
use serde::Serialize;

use uuid::Uuid;

use crate::endpoint::Endpoint;
use crate::Str;

/// An ID uniquely identifying an asset.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct Id(pub Uuid);

impl Deref for Id {
  type Target = Uuid;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}


/// An enumeration of the various asset classes available.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum Class {
  /// US equities.
  #[serde(rename = "us_equity")]
  UsEquity,
}

impl AsRef<str> for Class {
  fn as_ref(&self) -> &'static str {
    match *self {
      Class::UsEquity => "us_equity",
    }
  }
}


/// The status an asset can have.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum Status {
  /// The asset is active.
  #[serde(rename = "active")]
  Active,
  /// The asset is inactive.
  #[serde(rename = "inactive")]
  Inactive,
}

impl AsRef<str> for Status {
  fn as_ref(&self) -> &'static str {
    match *self {
      Status::Active => "active",
      Status::Inactive => "inactive",
    }
  }
}


/// An enumeration of the various supported exchanges.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum Exchange {
  /// American Stock Exchange.
  #[serde(rename = "AMEX")]
  Amex,
  /// XXX
  // TODO: Not quite clear.
  #[serde(rename = "ARCA")]
  Arca,
  /// BATS Global Markets.
  #[serde(rename = "BATS")]
  Bats,
  /// New York Stock Exchange.
  #[serde(rename = "NYSE")]
  Nyse,
  /// Nasdaq Stock Market.
  #[serde(rename = "NASDAQ")]
  Nasdaq,
  /// NYSE Arca.
  #[serde(rename = "NYSEARCA")]
  Nysearca,
}


/// The representation of an asset as used by Alpaca.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Asset {
  /// The asset's ID.
  #[serde(rename = "id")]
  pub id: Id,
  /// The asset's class.
  #[serde(rename = "asset_class")]
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
}


/// A GET request to be made to the /v1/assets endpoint.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AssetReq {
  /// The symbol of the asset in question.
  // TODO: It is not quite clear if what is wanted here is really only a
  //       symbol. Somewhere it was stated that asset class and exchange
  //       may optionally also be part of the specification.
  symbol: String,
}


/// The representation of a GET request to the /v1/assets/<symbol> endpoint.
#[derive(Debug)]
struct Get {}

EndpointDef! {
  Get,
  Ok => Asset, GetOk, [
    /* 200 */ OK,
  ],
  Err => GetError, [
    /* 404 */ NOT_FOUND => NotFound,
  ]
}

impl Endpoint for Get {
  type Input = AssetReq;
  type Output = GetOk;
  type Error = GetError;

  fn path(input: &Self::Input) -> Str {
    format!("/v1/assets/{}", input.symbol).into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_str as from_json;

  use test_env_log::test;

  use tokio::runtime::current_thread::block_on_all;

  use crate::api::v1::asset::Id;
  use crate::Client;
  use crate::Error;


  #[test]
  fn parse_reference_asset() {
    let response = r#"{
  "id": "904837e3-3b76-47ec-b432-046db621571b",
  "asset_class": "us_equity",
  "exchange": "NASDAQ",
  "symbol": "AAPL",
  "status": "active",
  "tradable": true
}"#;

    let id = Id(Uuid::parse_str("904837e3-3b76-47ec-b432-046db621571b").unwrap());
    let asset = from_json::<Asset>(&response).unwrap();
    assert_eq!(asset.id, id);
    assert_eq!(asset.class, Class::UsEquity);
    assert_eq!(asset.exchange, Exchange::Nasdaq);
    assert_eq!(asset.symbol, "AAPL");
    assert_eq!(asset.status, Status::Active);
    assert_eq!(asset.tradable, true);
  }

  #[test]
  fn retrieve_asset() -> Result<(), Error> {
    let client = Client::from_env()?;
    let request = AssetReq {
      symbol: "AAPL".to_string(),
    };
    let future = client.issue::<Get>(request)?;
    let asset = block_on_all(future)?;

    // The AAPL asset ID, retrieved out-of-band.
    let id = Id(Uuid::parse_str("b0b6dd9d-8b9b-48a9-ba46-b9d54906e415").unwrap());
    assert_eq!(asset.id, id);
    assert_eq!(asset.class, Class::UsEquity);
    assert_eq!(asset.exchange, Exchange::Nasdaq);
    assert_eq!(asset.symbol, "AAPL");
    assert_eq!(asset.status, Status::Active);
    assert_eq!(asset.tradable, true);
    Ok(())
  }
}
