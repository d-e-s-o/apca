// Copyright (C) 2021-2024 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::DateTime;
use chrono::Utc;

use serde::Deserialize;

use crate::api::v2::account;
use crate::api::v2::watchlist;
use crate::Str;


/// A watchlist item.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WatchlistItem {
  /// The watchlist's ID.
  #[serde(rename = "id")]
  pub id: watchlist::Id,
  /// The watchlist's user-defined name.
  #[serde(rename = "name")]
  pub name: String,
  /// The account's ID.
  #[serde(rename = "account_id")]
  pub account_id: account::Id,
  /// Timestamp this watchlist was created at.
  #[serde(rename = "created_at")]
  pub created_at: DateTime<Utc>,
  /// Timestamp this watchlist was last updated at.
  #[serde(rename = "updated_at")]
  pub updated_at: DateTime<Utc>,
}


Endpoint! {
  /// The representation of a GET request to the /v2/watchlists endpoint.
  pub Get(()),
  Ok => Vec<WatchlistItem>, [
    /// The list of watchlist items was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  #[inline]
  fn path(_input: &Self::Input) -> Str {
    "/v2/watchlists".into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use uuid::Uuid;

  use crate::api::v2::watchlist;
  use crate::api::v2::watchlist::CreateReq;
  use crate::api_info::ApiInfo;
  use crate::Client;
  use test_log::test;


  /// Check that we can list existing watchlists.
  #[test(tokio::test)]
  async fn list_watchlists() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let id = Uuid::new_v4().to_string();
    let created = client
      .issue::<watchlist::Post>(&CreateReq {
        name: id.clone(),
        symbols: vec!["AAPL".to_string()],
      })
      .await
      .unwrap();

    let result = client.issue::<Get>(&()).await;
    client
      .issue::<watchlist::Delete>(&created.id)
      .await
      .unwrap();

    let watchlists = result.unwrap();
    assert!(
      watchlists
        .iter()
        .any(|l| l.id == created.id && l.name == id),
      "{watchlists:#?} : {id}"
    )
  }
}
