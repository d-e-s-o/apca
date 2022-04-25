// Copyright (C) 2019-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Serialize;
use serde_urlencoded::to_string as to_query;

use crate::api::v2::asset::Asset;
use crate::api::v2::asset::Class;
use crate::api::v2::asset::Status;
use crate::Str;


/// A helper for initializing `AssetsReq` objects.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AssetsReqInit {
  /// See `AssetsReq::status`.
  pub status: Status,
  /// See `AssetsReq::class`.
  pub class: Class,
  #[doc(hidden)]
  pub _non_exhaustive: (),
}

impl AssetsReqInit {
  /// Create an `AssetsReq` from an `AssetsReqInit`.
  #[inline]
  pub fn init(self) -> AssetsReq {
    AssetsReq {
      status: self.status,
      class: self.class,
    }
  }
}


/// A GET request to be made to the /v2/assets endpoint.
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct AssetsReq {
  /// The status of assets to include in the response.
  #[serde(rename = "status")]
  pub status: Status,
  /// The asset class of which to include assets in the response.
  #[serde(rename = "asset_class")]
  pub class: Class,
}


Endpoint! {
  /// The representation of a GET request to the /v2/assets endpoint.
  pub Get(AssetsReq),
  Ok => Vec<Asset>, [
    /// The list of assets was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

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

  use test_log::test;

  use crate::api::v2::asset::Exchange;
  use crate::api_info::ApiInfo;
  use crate::Client;


  /// Make sure that we can list available US stock assets.
  #[test(tokio::test)]
  async fn list_us_stock_assets() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let request = AssetsReqInit::default().init();
    let assets = client.issue::<Get>(&request).await.unwrap();

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
    let request = AssetsReqInit {
      class: Class::Crypto,
      ..Default::default()
    }
    .init();

    let assets = client.issue::<Get>(&request).await.unwrap();

    let asset = assets.iter().find(|x| x.symbol == "BTCUSD").unwrap();
    assert_eq!(asset.class, Class::Crypto);
  }
}
