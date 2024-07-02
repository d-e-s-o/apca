// Copyright (C) 2019-2024 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;
use serde::Serialize;
use serde_urlencoded::to_string as to_query;

use crate::api::v2::asset::Asset;
use crate::api::v2::asset::Class;
use crate::api::v2::asset::Status;
use crate::Str;


/// A helper for initializing `ListReq` objects.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ListReqInit {
  /// See `ListReq::status`.
  pub status: Status,
  /// See `ListReq::class`.
  pub class: Class,
  #[doc(hidden)]
  pub _non_exhaustive: (),
}

impl ListReqInit {
  /// Create an `ListReq` from an `ListReqInit`.
  #[inline]
  pub fn init(self) -> ListReq {
    ListReq {
      status: self.status,
      class: self.class,
    }
  }
}


/// A GET request to be made to the /v2/assets endpoint.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ListReq {
  /// The status of assets to include in the response.
  #[serde(rename = "status")]
  pub status: Status,
  /// The asset class of which to include assets in the response.
  #[serde(rename = "asset_class")]
  pub class: Class,
}


Endpoint! {
  /// The representation of a GET request to the /v2/assets endpoint.
  pub List(ListReq),
  Ok => Vec<Asset>, [
    /// The list of assets was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => ListError, []

  #[inline]
  fn path(_input: &Self::Input) -> Str {
    "/v2/assets".into()
  }

  fn query(input: &Self::Input) -> Result<Option<Str>, Self::ConversionError> {
    Ok(Some(to_query(input)?.into()))
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_slice as from_json;
  use serde_json::to_vec as to_json;

  use test_log::test;

  use crate::api::v2::asset::Exchange;
  use crate::api_info::ApiInfo;
  use crate::Client;


  /// Check that we can serialize and deserialize a [`ListReq`].
  #[test]
  fn serialize_deserialize_list_request() {
    let request = ListReqInit {
      status: Status::Active,
      class: Class::UsEquity,
      ..Default::default()
    }
    .init();

    let json = to_json(&request).unwrap();
    assert_eq!(from_json::<ListReq>(&json).unwrap(), request);
  }


  /// Make sure that we can list available US stock assets.
  #[test(tokio::test)]
  async fn list_us_stock_assets() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let request = ListReqInit::default().init();
    let assets = client.issue::<List>(&request).await.unwrap();

    let asset = assets.iter().find(|x| x.symbol == "AAPL").unwrap();
    assert_eq!(asset.class, Class::UsEquity);
    assert_eq!(asset.exchange, Exchange::Nasdaq);
    assert_eq!(asset.status, Status::Active);
  }


  /// Make sure that we can list available crypto currency assets.
  #[test(tokio::test)]
  async fn list_crypto_assets() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let request = ListReqInit {
      class: Class::Crypto,
      ..Default::default()
    }
    .init();

    let assets = client.issue::<List>(&request).await.unwrap();

    let asset = assets.iter().find(|x| x.symbol == "BTC/USD").unwrap();
    assert_eq!(asset.class, Class::Crypto);
  }
}
