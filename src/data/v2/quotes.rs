// Copyright (C) 2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::DateTime;
use chrono::Utc;

use serde::Deserialize;
use serde::Serialize;
use serde_urlencoded::to_string as to_query;

use crate::data::v2::Feed;
use crate::data::DATA_BASE_URL;
use crate::util::vec_from_str;
use crate::Str;

/// A quote as returned by the /v2/stocks/<symbol>/quotes endpoint.
pub use super::last_quote::Quote;


/// A collection of quotes as returned by the API. This is one page of
/// quotes.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct Quotes {
  /// The list of returned quotes.
  #[serde(deserialize_with = "vec_from_str")]
  pub quotes: Vec<Quote>,
  /// The symbol the quotes correspond to.
  pub symbol: String,
  /// The token to provide to a request to get the next page of quotes
  /// for this request.
  pub next_page_token: Option<String>,
}


/// A helper for initializing [`QuotesReq`] objects.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QuotesReqInit {
  /// See `QuotesReq::limit`.
  pub limit: Option<usize>,
  /// See `QuotesReq::feed`.
  pub feed: Option<Feed>,
  /// See `QuotesReq::page_token`.
  pub page_token: Option<String>,
  #[doc(hidden)]
  pub _non_exhaustive: (),
}

impl QuotesReqInit {
  /// Create a [`QuotesReq`] from a `QuotesReqInit`.
  #[inline]
  pub fn init<S>(self, symbol: S, start: DateTime<Utc>, end: DateTime<Utc>) -> QuotesReq
  where
    S: Into<String>,
  {
    QuotesReq {
      symbol: symbol.into(),
      start,
      end,
      limit: self.limit,
      feed: self.feed,
      page_token: self.page_token,
    }
  }
}


/// A GET request to be made to the /v2/stocks/<symbol>/quotes endpoint.
// TODO: Not all fields are hooked up.
#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct QuotesReq {
  /// The symbol to retrieve quotes for.
  #[serde(skip)]
  pub symbol: String,
  /// Filter data equal to or after this time in RFC-3339 format.
  /// Defaults to the current day in CT.
  #[serde(rename = "start")]
  pub start: DateTime<Utc>,
  /// Filter data equal to or before this time in RFC-3339 format.
  /// Default value is now.
  #[serde(rename = "end")]
  pub end: DateTime<Utc>,
  /// Number of quotes to return. Must be in range 1-10000, defaults to
  /// 1000.
  #[serde(rename = "limit")]
  pub limit: Option<usize>,
  /// The data feed to use.
  #[serde(rename = "feed")]
  pub feed: Option<Feed>,
  /// Pagination token to continue from.
  #[serde(rename = "page_token")]
  pub page_token: Option<String>,
}


Endpoint! {
  /// The representation of a GET request to the
  /// /v2/stocks/<symbol>/quotes endpoint.
  pub Get(QuotesReq),
  Ok => Quotes, [
    /// The quote information was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// Some of the provided data was invalid or not found.
    /* 422 */ UNPROCESSABLE_ENTITY => InvalidInput,
  ]

  fn base_url() -> Option<Str> {
    Some(DATA_BASE_URL.into())
  }

  #[inline]
  fn path(input: &Self::Input) -> Str {
    format!("/v2/stocks/{}/quotes", input.symbol).into()
  }

  fn query(input: &Self::Input) -> Result<Option<Str>, Self::ConversionError> {
    Ok(Some(to_query(input)?.into()))
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::str::FromStr as _;

  use num_decimal::Num;

  use test_log::test;

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::RequestError;


  /// Check that we can retrieve quotes for a specific time frame.
  #[test(tokio::test)]
  async fn request_quotes() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = DateTime::from_str("2022-01-04T13:35:59Z").unwrap();
    let end = DateTime::from_str("2022-01-04T13:36:00Z").unwrap();
    let request = QuotesReqInit::default().init("SPY", start, end);
    let quotes = client.issue::<Get>(&request).await.unwrap();

    assert_eq!(&quotes.symbol, "SPY");

    for quote in quotes.quotes {
      assert!(quote.time >= start, "{}", quote.time);
      assert!(quote.time <= end, "{}", quote.time);
      assert_ne!(quote.ask_price, Num::from(0));
      assert_ne!(quote.bid_price, Num::from(0));
      assert_ne!(quote.ask_size, 0);
      assert_ne!(quote.bid_size, 0);
    }
  }

  /// Verify that we error out as expected when attempting to retrieve
  /// the quotes for a non-existent symbol.
  #[test(tokio::test)]
  async fn nonexistent_symbol() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = DateTime::from_str("2022-01-04T13:35:59Z").unwrap();
    let end = DateTime::from_str("2022-01-04T13:36:00Z").unwrap();
    let request = QuotesReqInit::default().init("ABC123", start, end);
    let err = client.issue::<Get>(&request).await.unwrap_err();
    match err {
      RequestError::Endpoint(GetError::InvalidInput(_)) => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
  }

  /// Check that we fail as expected when an invalid page token is
  /// specified.
  #[test(tokio::test)]
  async fn invalid_page_token() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = DateTime::from_str("2022-01-04T13:35:59Z").unwrap();
    let end = DateTime::from_str("2022-01-04T13:36:00Z").unwrap();
    let request = QuotesReqInit {
      page_token: Some("123456789abcdefghi".to_string()),
      ..Default::default()
    }
    .init("SPY", start, end);

    let err = client.issue::<Get>(&request).await.unwrap_err();
    match err {
      RequestError::Endpoint(GetError::InvalidInput(_)) => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
  }

  /// Check that we can page quotes as expected.
  #[test(tokio::test)]
  async fn page_quotes() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = DateTime::from_str("2022-01-04T13:35:00Z").unwrap();
    let end = DateTime::from_str("2022-01-04T13:36:00Z").unwrap();
    let mut request = QuotesReqInit {
      limit: Some(2),
      ..Default::default()
    }
    .init("SPY", start, end);

    let mut last_quotes = None;
    // We assume that there are at least three pages of two quotes.
    for _ in 0..3 {
      let quotes = client.issue::<Get>(&request).await.unwrap();
      assert_ne!(Some(quotes.clone()), last_quotes);

      request.page_token = quotes.next_page_token.clone();
      last_quotes = Some(quotes);
    }
  }
}
