// Copyright (C) 2022-2023 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

//! Functionality for retrieving historical stock data news.

use chrono::DateTime;
use chrono::Utc;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde_urlencoded::to_string as to_query;

use crate::data::DATA_BASE_URL;
use crate::util::slice_to_str;
use crate::util::vec_from_str;
use crate::Str;


fn symbols_slice_to_str<S>(slice: &[String], serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  fn name_fn(_: &String) -> Str {
    "symbols".into()
  }

  slice_to_str(slice, name_fn, serializer)
}


/// Deserialize a string that may be empty, sanitizing it somewhat in
/// the process.
fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
  D: Deserializer<'de>,
{
  let string = String::deserialize(deserializer)?.trim().to_string();
  let result = if string.is_empty() {
    None
  } else {
    Some(string)
  };
  Ok(result)
}


/// A GET request to be issued to the /v1beta1/news endpoint.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct NewsReq {
  /// The symbols for which to retrieve news. An empty vector retrieves
  /// news for any symbol (including those for crypto currencies).
  #[serde(rename = "symbols", serialize_with = "symbols_slice_to_str")]
  pub symbols: Vec<String>,
  /// The maximum number of news items to be returned for a given page.
  ///
  /// It can be between 1 and 50. Defaults to 10 if the provided value
  /// is None.
  #[serde(rename = "limit")]
  pub limit: Option<usize>,
  /// Report news items on or after this time. Defaults to 2015-01-01 if
  /// not set.
  #[serde(rename = "start")]
  pub start: Option<DateTime<Utc>>,
  /// Report news items on or before this time. Defaults to the current
  /// time if not set.
  #[serde(rename = "end")]
  pub end: Option<DateTime<Utc>>,
  /// If provided we will pass a page token to continue where we left off.
  #[serde(rename = "page_token", skip_serializing_if = "Option::is_none")]
  pub page_token: Option<String>,
}


/// A news item as returned by the /v1beta1/news endpoint.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct NewsItem {
  /// A list of related or mentioned symbols.
  #[serde(rename = "symbols")]
  pub symbols: Vec<String>,
  /// Source where the news originated (e.g., Benzinga).
  #[serde(rename = "source")]
  pub source: String,
  /// The time when this news item was created.
  #[serde(rename = "created_at")]
  pub created_at: DateTime<Utc>,
  /// The time when this news item was last updated.
  #[serde(rename = "updated_at")]
  pub updated_at: DateTime<Utc>,
  /// The news item's headline.
  #[serde(rename = "headline")]
  pub headline: String,
  /// A summary of the news item.
  #[serde(rename = "summary", deserialize_with = "deserialize_optional_string")]
  pub summary: Option<String>,
  /// A URL of the news item.
  #[serde(rename = "url", deserialize_with = "deserialize_optional_string")]
  pub url: Option<String>,
}


/// A collection of news items as returned by the API. This is one page
/// of items.
#[derive(Debug, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct News {
  /// The list of returned news items.
  #[serde(rename = "news", deserialize_with = "vec_from_str")]
  pub items: Vec<NewsItem>,
  /// The token to provide to a request to get the next page of news
  /// items for this request.
  pub next_page_token: Option<String>,
}


Endpoint! {
  /// The representation of a GET request to the /v1beta1/news endpoint.
  pub Get(NewsReq),
  Ok => News, [
    /// The list of news items was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// A query parameter was invalid.
    /* 422 */ UNPROCESSABLE_ENTITY => InvalidInput,
  ]

  fn base_url() -> Option<Str> {
    Some(DATA_BASE_URL.into())
  }

  fn path(_input: &Self::Input) -> Str {
    "/v1beta1/news".into()
  }

  fn query(input: &Self::Input) -> Result<Option<Str>, Self::ConversionError> {
    Ok(Some(to_query(input)?.into()))
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use test_log::test;

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::RequestError;


  /// Check that we can properly retrieve news items.
  #[test(tokio::test)]
  async fn request_news_items() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let request = NewsReq::default();
    let news = client.issue::<Get>(&request).await.unwrap();
    assert!(news.items.len() > 1);

    for item in news.items {
      assert!(item.created_at <= item.updated_at, "{:?}", item);
      assert!(!item.headline.is_empty());
      assert!(!item.source.is_empty());
      assert!(
        item.summary.is_none() || !item.summary.as_ref().unwrap().is_empty(),
        "{:?}",
        item
      );
    }
  }

  /// Verify that we can request news items via the provided page token.
  #[test(tokio::test)]
  async fn pagination() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let mut request = NewsReq {
      limit: Some(1),
      ..Default::default()
    };
    let news = client.issue::<Get>(&request).await.unwrap();
    assert_eq!(news.items.len(), 1);

    request.page_token = news.next_page_token;

    let new_news = client.issue::<Get>(&request).await.unwrap();

    assert_eq!(new_news.items.len(), 1);
    assert!(new_news.items[0].created_at < news.items[0].created_at);
  }

  /// Check that we fail as expected when an invalid page token is
  /// specified.
  #[test(tokio::test)]
  async fn invalid_page_token() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let request = NewsReq {
      page_token: Some("123456789abcdefghi".to_string()),
      ..Default::default()
    };

    let err = client.issue::<Get>(&request).await.unwrap_err();
    match err {
      RequestError::Endpoint(GetError::InvalidInput(_)) => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
  }
}
