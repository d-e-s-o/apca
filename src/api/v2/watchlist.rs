// Copyright (C) 2021 The apca Developers
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


/// A watchlist.
#[derive(Deserialize, PartialEq, Debug)]
pub struct Watchlist {
  /// The watchlist's ID.
  #[serde(rename = "id")]
  pub id: Id,
  /// The account's ID.
  #[serde(rename = "account_id")]
  pub account_id: account::Id,
  /// Timestamp this watchlist was created at.
  #[serde(rename = "created_at")]
  pub created_at: DateTime<Utc>,
  /// Timestamp this watchlist was last updated at.
  #[serde(rename = "updated_at")]
  pub updated_at: DateTime<Utc>,
  /// The list of watched assets.
  #[serde(rename = "assets")]
  pub assets: Vec<asset::Asset>,
}


/// A create watchlist request item
#[derive(Serialize, PartialEq, Debug, Clone)]
pub struct CreateReq {
  /// The watchlist's name.
  #[serde(rename = "name")]
  pub name: String,
  /// The symbols to watch.
  #[serde(rename = "symbols")]
  pub symbols: Vec<String>,
}


Endpoint! {
  /// The representation of a POST request to the /v2/watchlists endpoint.
  pub Post(CreateReq),
  Ok => Watchlist, [
      /// The watchlist was created successfully.
      /* 200 */ OK,
  ],
  Err => CreateError, [
    /// The watchlist name was not unique or other parts of the input
    /// are not valid.
    /* 422 */ UNPROCESSABLE_ENTITY => InvalidInput,
  ]

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
  /// The representation of a GET request to the
  /// /v2/watchlists/<watchlist-id> endpoint.
  pub Get(Id),
  Ok => Watchlist, [
    /// The watchlist object with the given ID was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// No watchlist was found with the given ID.
    /* 404 */ NOT_FOUND => NotFound,
  ]

  fn path(input: &Self::Input) -> Str {
    format!("/v2/watchlists/{}", input.to_simple()).into()
  }
}


EndpointNoParse! {
  /// The representation of a DELETE request to the
  /// /v2/watchlists/<watchlist-id> endpoint.
  pub Delete(Id),
  Ok => (), [
    /// The watchlist was deleted successfully.
    /* 204 */ NO_CONTENT,
  ],
  Err => DeleteError, [
    /// No watchlist was found with the given ID.
    /* 404 */ NOT_FOUND => NotFound,
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

  use crate::api::v2::account;
  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::RequestError;

  use test_env_log::test;


  /// Check that we can create, retrieve, and delete a watchlist.
  #[test(tokio::test)]
  async fn create_get_delete() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let expected_symbols = vec!["AAPL".to_string(), "AMZN".to_string()];
    let created = client
      .issue::<Post>(&CreateReq {
        name: Uuid::new_v4().to_string(),
        symbols: expected_symbols.clone(),
      })
      .await
      .unwrap();
    let result = client.issue::<Get>(&created.id).await;
    client.issue::<Delete>(&created.id).await.unwrap();

    let watchlist = result.unwrap();
    let tracked_symbols = watchlist
      .assets
      .into_iter()
      .map(|a| a.symbol)
      .collect::<Vec<_>>();
    assert_eq!(tracked_symbols, expected_symbols);

    // Also check that the reported account ID matches our account.
    let account = client.issue::<account::Get>(&()).await.unwrap();
    assert_eq!(watchlist.account_id, account.id);
  }

  /// Check that we get back the expected error when attempting to
  /// create a watchlist with a name that is already taken.
  #[test(tokio::test)]
  async fn create_duplicate_name() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let name = "the-name";
    let created = client
      .issue::<Post>(&CreateReq {
        name: name.to_string(),
        symbols: vec!["SPY".to_string()],
      })
      .await
      .unwrap();

    let result = client
      .issue::<Post>(&CreateReq {
        name: name.to_string(),
        symbols: vec!["SPY".to_string()],
      })
      .await;

    client.issue::<Delete>(&created.id).await.unwrap();

    let err = result.unwrap_err();
    match err {
      RequestError::Endpoint(CreateError::InvalidInput(_)) => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
  }

  /// Verify that we report the appropriate error when attempting to
  /// retrieve a watchlist that does not exist.
  #[test(tokio::test)]
  async fn get_non_existent() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let created = client
      .issue::<Post>(&CreateReq {
        name: Uuid::new_v4().to_string(),
        symbols: vec!["AAPL".to_string()],
      })
      .await
      .unwrap();
    client.issue::<Delete>(&created.id).await.unwrap();

    let err = client.issue::<Get>(&created.id).await.unwrap_err();
    match err {
      RequestError::Endpoint(GetError::NotFound(_)) => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
  }

  /// Verify that we report the appropriate error when attempting to
  /// delete a watchlist that does not exist.
  #[test(tokio::test)]
  async fn delete_non_existent() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let id = Id(Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap());
    let err = client.issue::<Delete>(&id).await.unwrap_err();
    match err {
      RequestError::Endpoint(DeleteError::NotFound(_)) => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
  }
}
