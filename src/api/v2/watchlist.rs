// Copyright (C) 2019-2021 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ops::Deref;

use chrono::DateTime;
use chrono::Utc;

use http::Method;
use http_endpoint::Bytes;

use serde::Deserialize;
use serde::Serialize;

use serde_json::from_slice as from_json;
use serde_json::to_vec as to_json;

use uuid::Uuid;

use crate::api::v2::account;
use crate::api::v2::asset;
use crate::Str;

/// An ID uniquely identifying a watchlist.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct Id(pub Uuid);

impl Deref for Id {
  type Target = Uuid;

  #[inline]
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

/// A watchlist item
#[derive(Deserialize, PartialEq, Debug)]
pub struct Watchlist {
  /// The watchlist's id
  #[serde(rename = "id")]
  pub id: Id,
  /// The account's id
  #[serde(rename = "account_id")]
  pub account_id: account::Id,
  /// Creation datetime
  #[serde(rename = "created_at")]
  pub created_at: DateTime<Utc>,
  /// Last update datetime
  #[serde(rename = "updated_at")]
  pub updated_at: DateTime<Utc>,
  /// The list of watched assets
  #[serde(rename = "assets")]
  pub assets: Vec<asset::Asset>,
}

/// A create watchlist request item
#[derive(Serialize, PartialEq, Debug, Clone)]
pub struct CreateWatchlistReq {
  /// The watchlist's name
  #[serde(rename = "name")]
  pub name: String,
  /// The symbols to watch
  #[serde(rename = "symbols")]
  pub symbols: Vec<String>,
}

Endpoint! {
    /// The representation of a POST request to the /v2/watchlists endpoint.
    pub Post(CreateWatchlistReq),
    Ok => Watchlist, [
        /// The list of orders was retrieved successfully.
        /* 200 */ OK,
    ],
    Err => PostError, []

    #[inline]
    fn path(_input: &Self::Input) -> Str {
        "/v2/watchlists".into()
    }

    #[inline]
    fn method() -> Method {
        Method::POST
    }

    fn body(input: &Self::Input) -> Result<Option<Bytes>, Self::ConversionError> {
        let json = to_json(input)?;
        let bytes = Bytes::from(json);
        Ok(Some(bytes))
    }
}

Endpoint! {
  /// The representation of a GET request to the /v2/watchlists/<watchlist-id>
  /// endpoint.
  pub Get(Id),
  Ok => Watchlist, [
    /// The order object for the given ID was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// No order was found with the given ID.
    /* 404 */ NOT_FOUND => NotFound,
  ]

  fn path(input: &Self::Input) -> Str {
    format!("/v2/watchlists/{}", input.to_simple()).into()
  }
}

EndpointNoParse! {
  /// The representation of a DELETE request to the /v2/orders/<order-id>
  /// endpoint.
  pub Delete(Id),
  Ok => (), [
    /// The order was canceled successfully.
    /* 204 */ NO_CONTENT,
  ],
  Err => DeleteError, [
    /// No order was found with the given ID.
    /* 404 */ NOT_FOUND => NotFound,
    /// The order can no longer be canceled.
    /* 422 */ UNPROCESSABLE_ENTITY => NotCancelable,
  ]

  #[inline]
  fn method() -> Method {
    Method::DELETE
  }

  fn path(input: &Self::Input) -> Str {
    format!("/v2/watchlists/{}", input.to_simple()).into()
  }

  #[inline]
  fn parse(body: &[u8]) -> Result<Self::Output, Self::ConversionError> {
    debug_assert_eq!(body, b"");
    Ok(())
  }

  fn parse_err(body: &[u8]) -> Result<Self::ApiError, Vec<u8>> {
    from_json::<Self::ApiError>(body).map_err(|_| body.to_vec())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use crate::api_info::ApiInfo;
  use crate::Client;

  use test_env_log::test;

  #[test(tokio::test)]
  async fn get_watchlist() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let expected_symbols = vec!["AAPL".to_string(), "AMZN".to_string()];
    let created = client
      .issue::<Post>(&CreateWatchlistReq {
        name: Uuid::new_v4().to_string(),
        symbols: expected_symbols.clone(),
      })
      .await
      .unwrap();
    let result = client.issue::<Get>(&created.id).await;
    client.issue::<Delete>(&created.id).await.unwrap();
    let watchlist = result.unwrap();
    let tracked_symbols: Vec<String> = watchlist.assets.iter().map(|a| a.symbol.clone()).collect();
    assert_eq!(tracked_symbols, expected_symbols)
  }

  #[test(tokio::test)]
  async fn delete_watchlist() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let created = client
      .issue::<Post>(&CreateWatchlistReq {
        name: Uuid::new_v4().to_string(),
        symbols: vec!["AAPL".to_string()],
      })
      .await
      .unwrap();
    client.issue::<Delete>(&created.id).await.unwrap();
    let result = client.issue::<Get>(&created.id).await;
    assert!(result.is_err())
  }
}
