// Copyright (C) 2019-2021 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::DateTime;
use chrono::Utc;

use serde::Deserialize;

use crate::api::v2::account;
use crate::api::v2::watchlist;
use crate::Str;

/// A watchlist item
#[derive(Deserialize, PartialEq, Debug, Clone, Copy)]
pub struct WatchlistListItem {
  /// The watchlist's id
  #[serde(rename = "id")]
  pub id: watchlist::Id,
  /// The account's id
  #[serde(rename = "account_id")]
  pub account_id: account::Id,
  /// Creation datetime
  #[serde(rename = "created_at")]
  pub created_at: DateTime<Utc>,
  /// Last update datetime
  #[serde(rename = "updated_at")]
  pub updated_at: DateTime<Utc>,
}

Endpoint! {
  /// The representation of a GET request to the /v2/watchlists endpoint.
  pub Get(()),
  Ok => Vec<WatchlistListItem>, [
    /// The list of orders was retrieved successfully.
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
  use crate::api::v2::watchlist::CreateWatchlistReq;
  use crate::api_info::ApiInfo;
  use crate::Client;
  use test_env_log::test;

  #[test(tokio::test)]
  async fn list_watchlists() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let created = client
      .issue::<watchlist::Post>(&CreateWatchlistReq {
        name: Uuid::new_v4().to_string(),
        symbols: vec!["AAPL".to_string()],
      })
      .await
      .unwrap();
    let result = client.issue::<Get>(&()).await;

    client.issue::<watchlist::Delete>(&created.id).await.unwrap();
    let watchlists = result.unwrap();
    let ids: Vec<_> = watchlists.iter().map(|w| w.id).collect();
    assert!(ids.contains(&created.id))
  }
}
