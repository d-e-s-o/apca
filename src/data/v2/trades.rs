// Copyright (C) 2022-2023 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::DateTime;
use chrono::Utc;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;
use serde_urlencoded::to_string as to_query;

use crate::data::v2::Feed;
use crate::data::DATA_BASE_URL;
use crate::util::vec_from_str;
use crate::Str;


/// A GET request to be issued to the /v2/stocks/{symbol}/trades endpoint.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TradesReq {
  /// The symbol for which to retrieve market data.
  #[serde(skip)]
  pub symbol: String,
  /// The maximum number of trades to be returned for each symbol.
  ///
  /// It can be between 1 and 10000. Defaults to 1000 if the provided
  /// value is None.
  #[serde(rename = "limit")]
  pub limit: Option<usize>,
  /// Filter trades equal to or after this time.
  #[serde(rename = "start")]
  pub start: DateTime<Utc>,
  /// Filter trades equal to or before this time.
  #[serde(rename = "end")]
  pub end: DateTime<Utc>,
  /// The data feed to use.
  ///
  /// Defaults to [`IEX`][Feed::IEX] for free users and
  /// [`SIP`][Feed::SIP] for users with an unlimited subscription.
  #[serde(rename = "feed")]
  pub feed: Option<Feed>,
  /// If provided we will pass a page token to continue where we left off.
  #[serde(rename = "page_token", skip_serializing_if = "Option::is_none")]
  pub page_token: Option<String>,
}


/// A helper for initializing [`TradesReq`] objects.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TradesReqInit {
  /// See `TradesReq::limit`.
  pub limit: Option<usize>,
  /// See `TradesReq::feed`.
  pub feed: Option<Feed>,
  /// See `TradesReq::page_token`.
  pub page_token: Option<String>,
  #[doc(hidden)]
  pub _non_exhaustive: (),
}

impl TradesReqInit {
  /// Create a [`TradesReq`] from a `TradesReqInit`.
  #[inline]
  pub fn init<S>(self, symbol: S, start: DateTime<Utc>, end: DateTime<Utc>) -> TradesReq
  where
    S: Into<String>,
  {
    TradesReq {
      symbol: symbol.into(),
      start,
      end,
      limit: self.limit,
      feed: self.feed,
      page_token: self.page_token,
    }
  }
}


/// A market data trade as returned by the /v2/stocks/{symbol}/trades endpoint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[non_exhaustive]
pub struct Trade {
  /// Time of the trade.
  #[serde(rename = "t")]
  pub timestamp: DateTime<Utc>,
  /// The price of the trade.
  #[serde(rename = "p")]
  pub price: Num,
  /// The size of the trade.
  #[serde(rename = "s")]
  pub size: usize,
}


/// A collection of trades as returned by the API. This is one page of trades.
#[derive(Debug, Deserialize, Eq, PartialEq)]
#[non_exhaustive]
pub struct Trades {
  /// The list of returned trades.
  #[serde(deserialize_with = "vec_from_str")]
  pub trades: Vec<Trade>,
  /// The symbol the trades correspond to.
  pub symbol: String,
  /// The token to provide to a request to get the next page of trades for this request.
  pub next_page_token: Option<String>,
}


Endpoint! {
  /// The representation of a GET request to the /v2/stocks/{symbol}/trades endpoint.
  pub Get(TradesReq),
  Ok => Trades, [
    /// The market data was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// A query parameter was invalid.
    /* 422 */ UNPROCESSABLE_ENTITY => InvalidInput,
  ]

  fn base_url() -> Option<Str> {
    Some(DATA_BASE_URL.into())
  }

  fn path(input: &Self::Input) -> Str {
    format!("/v2/stocks/{}/trades", input.symbol).into()
  }

  fn query(input: &Self::Input) -> Result<Option<Str>, Self::ConversionError> {
    Ok(Some(to_query(input)?.into()))
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::str::FromStr as _;

  use http_endpoint::Endpoint;

  use serde_json::from_str as from_json;

  use test_log::test;

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::RequestError;


  /// Verify that we can properly parse a reference trades response.
  #[test]
  fn parse_reference_trades() {
    let response = r#"{
    "trades": [
      {
        "t": "2021-02-06T13:04:56.334320128Z",
        "x": "C",
        "p": 387.62,
        "s": 100,
        "c": [
            " ",
            "T"
        ],
        "i": 52983525029461,
        "z": "B"
      },
      {
        "t": "2021-02-06T13:09:42.325484032Z",
        "x": "C",
        "p": 387.69,
        "s": 100,
        "c": [
            " ",
            "T"
        ],
        "i": 52983525033813,
        "z": "B"
      }
    ],
    "symbol": "SPY",
    "next_page_token": "MjAyMS0wMi0wNlQxMzowOTo0Mlo7MQ=="
}"#;

    let res = from_json::<<Get as Endpoint>::Output>(response).unwrap();
    let trades = res.trades;
    let expected_time = DateTime::<Utc>::from_str("2021-02-06T13:04:56.334320128Z").unwrap();
    assert_eq!(trades.len(), 2);
    assert_eq!(trades[0].timestamp, expected_time);
    assert_eq!(trades[0].price, Num::new(38762, 100));
    assert_eq!(trades[0].size, 100);
    assert_eq!(res.symbol, "SPY".to_string());
    assert!(res.next_page_token.is_some())
  }

  /// Check that we can decode a response containing no trades correctly.
  #[test(tokio::test)]
  async fn no_trades() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let start = DateTime::from_str("2021-11-05T00:00:00Z").unwrap();
    let end = DateTime::from_str("2021-11-05T00:00:00Z").unwrap();
    let request = TradesReqInit::default().init("AAPL", start, end);

    let res = client.issue::<Get>(&request).await.unwrap();
    assert_eq!(res.trades, Vec::new())
  }

  /// Check that we can request historic trade data for a stock.
  #[test(tokio::test)]
  async fn request_trades() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let start = DateTime::from_str("2018-12-03T21:47:00Z").unwrap();
    let end = DateTime::from_str("2018-12-06T21:47:00Z").unwrap();
    let request = TradesReqInit {
      limit: Some(2),
      ..Default::default()
    }
    .init("AAPL", start, end);

    let res = client.issue::<Get>(&request).await.unwrap();
    let trades = res.trades;

    let expected_time = DateTime::<Utc>::from_str("2018-12-03T21:47:01.324200Z").unwrap();
    assert_eq!(trades.len(), 2);
    assert_eq!(trades[0].timestamp, expected_time);
    assert_eq!(trades[0].price, Num::new(4608, 25));
    assert_eq!(trades[0].size, 6);
    assert_eq!(res.symbol, "AAPL".to_string());
    assert!(res.next_page_token.is_some())
  }

  /// Verify that we can request data through a provided page token.
  #[test(tokio::test)]
  async fn can_follow_pagination() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let start = DateTime::from_str("2020-12-03T21:47:00Z").unwrap();
    let end = DateTime::from_str("2020-12-07T21:47:00Z").unwrap();
    let mut request = TradesReqInit {
      limit: Some(2),
      ..Default::default()
    }
    .init("AAPL", start, end);

    let mut res = client.issue::<Get>(&request).await.unwrap();
    let trades = res.trades;

    assert_eq!(trades.len(), 2);
    request.page_token = res.next_page_token;

    res = client.issue::<Get>(&request).await.unwrap();
    let new_trades = res.trades;

    assert_eq!(new_trades.len(), 2);
    assert!(new_trades[0].timestamp > trades[1].timestamp);
    assert!(res.next_page_token.is_some())
  }

  /// Check that we fail as expected when an invalid page token is
  /// specified.
  #[test(tokio::test)]
  async fn invalid_page_token() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = DateTime::from_str("2018-12-03T21:47:00Z").unwrap();
    let end = DateTime::from_str("2018-12-07T21:47:00Z").unwrap();
    let request = TradesReqInit {
      page_token: Some("123456789abcdefghi".to_string()),
      ..Default::default()
    }
    .init("SPY", start, end);

    let err = client.issue::<Get>(&request).await.unwrap_err();
    match err {
      RequestError::Endpoint(GetError::InvalidInput(_)) => (),
      _ => panic!("Received unexpected error: {err:?}"),
    };
  }

  /// Verify that we error out as expected when attempting to retrieve
  /// aggregate data trades for an invalid symbol.
  #[test(tokio::test)]
  async fn invalid_symbol() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = DateTime::from_str("2022-02-01T00:00:00Z").unwrap();
    let end = DateTime::from_str("2022-02-20T00:00:00Z").unwrap();
    let request = TradesReqInit::default().init("ABC123", start, end);

    let err = client.issue::<Get>(&request).await.unwrap_err();
    match err {
      // 42210000 is the error code reported for "invalid symbol".
      RequestError::Endpoint(GetError::InvalidInput(Ok(message))) if message.code == 42210000 => (),
      _ => panic!("Received unexpected error: {err:?}"),
    };
  }
}
