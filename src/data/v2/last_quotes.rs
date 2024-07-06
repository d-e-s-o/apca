// Copyright (C) 2021-2024 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeMap;

use chrono::DateTime;
use chrono::Utc;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;
use serde_json::from_slice as from_json;
use serde_urlencoded::to_string as to_query;

use crate::data::v2::Feed;
use crate::data::DATA_BASE_URL;
use crate::util::string_slice_to_str;
use crate::Str;


/// A GET request to be made to the /v2/stocks/quotes/latest endpoint.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GetReq {
  /// The symbols to retrieve the last quote for.
  #[serde(rename = "symbols", serialize_with = "string_slice_to_str")]
  pub symbols: Vec<String>,
  /// The data feed to use.
  #[serde(rename = "feed")]
  pub feed: Option<Feed>,
}


/// A helper for initializing [`GetReq`] objects.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[allow(missing_copy_implementations)]
pub struct GetReqInit {
  /// See `GetReq::feed`.
  pub feed: Option<Feed>,
  #[doc(hidden)]
  pub _non_exhaustive: (),
}

impl GetReqInit {
  /// Create a [`GetReq`] from a `GetReqInit`.
  #[inline]
  pub fn init<I, S>(self, symbols: I) -> GetReq
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    GetReq {
      symbols: symbols.into_iter().map(S::into).collect(),
      feed: self.feed,
    }
  }
}


/// A quote as returned by the /v2/stocks/quotes/latest endpoint.
// TODO: Not all fields are hooked up.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[non_exhaustive]
pub struct Quote {
  /// The time stamp of this quote.
  #[serde(rename = "t")]
  pub time: DateTime<Utc>,
  /// The ask price.
  #[serde(rename = "ap")]
  pub ask_price: Num,
  /// The ask size.
  #[serde(rename = "as")]
  pub ask_size: u64,
  /// The bid price.
  #[serde(rename = "bp")]
  pub bid_price: Num,
  /// The bid size.
  #[serde(rename = "bs")]
  pub bid_size: u64,
}


EndpointNoParse! {
  /// The representation of a GET request to the
  /// /v2/stocks/quotes/latest endpoint.
  pub Get(GetReq),
  Ok => Vec<(String, Quote)>, [
    /// The last quotes were retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// The provided symbol was invalid or not found or the data feed is
    /// not supported.
    /* 400 */ BAD_REQUEST => InvalidInput,
  ]

  fn base_url() -> Option<Str> {
    Some(DATA_BASE_URL.into())
  }

  fn path(_input: &Self::Input) -> Str {
    "/v2/stocks/quotes/latest".into()
  }

  fn query(input: &Self::Input) -> Result<Option<Str>, Self::ConversionError> {
    Ok(Some(to_query(input)?.into()))
  }

  fn parse(body: &[u8]) -> Result<Self::Output, Self::ConversionError> {
    // TODO: Ideally we'd write our own deserialize implementation here
    //       to create a vector right away instead of going through a
    //       BTreeMap.

    /// A helper object for parsing the response to a `Get` request.
    #[derive(Deserialize)]
    struct Response {
      /// A mapping from symbols to quote objects.
      // We use a `BTreeMap` here to have a consistent ordering of
      // quotes.
      quotes: BTreeMap<String, Quote>,
    }

    // We are not interested in the actual `Response` object. Clients
    // can keep track of what symbol they requested a quote for.
    from_json::<Response>(body)
      .map(|response| {
        response
          .quotes
          .into_iter()
          .collect()
      })
      .map_err(Self::ConversionError::from)
  }

  fn parse_err(body: &[u8]) -> Result<Self::ApiError, Vec<u8>> {
    from_json::<Self::ApiError>(body).map_err(|_| body.to_vec())
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use chrono::Duration;

  use http_endpoint::Endpoint as _;

  use test_log::test;

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::RequestError;


  /// Check that we can parse the reference quotes from the
  /// documentation.
  #[test]
  fn parse_reference_quotes() {
    let response = br#"{
      "quotes": {
        "TSLA": {
          "t": "2022-04-12T17:26:45.009288296Z",
          "ax": "V",
          "ap": 1020,
          "as": 3,
          "bx": "V",
          "bp": 990,
          "bs": 5,
          "c": ["R"],
          "z": "C"
        },
        "AAPL": {
          "t": "2022-04-12T17:26:44.962998616Z",
          "ax": "V",
          "ap": 170,
          "as": 1,
          "bx": "V",
          "bp": 168.03,
          "bs": 1,
          "c": ["R"],
          "z": "C"
        }
      }
    }"#;

    let quotes = Get::parse(response).unwrap();
    assert_eq!(quotes.len(), 2);

    assert_eq!(quotes[0].0, "AAPL");
    let aapl = &quotes[0].1;
    assert_eq!(
      aapl.time,
      DateTime::parse_from_rfc3339("2022-04-12T17:26:44.962998616Z").unwrap()
    );
    assert_eq!(aapl.ask_price, Num::new(170, 1));
    assert_eq!(aapl.ask_size, 1);
    assert_eq!(aapl.bid_price, Num::new(16803, 100));
    assert_eq!(aapl.bid_size, 1);

    assert_eq!(quotes[1].0, "TSLA");
    let tsla = &quotes[1].1;
    assert_eq!(
      tsla.time,
      DateTime::parse_from_rfc3339("2022-04-12T17:26:45.009288296Z").unwrap()
    );
    assert_eq!(tsla.ask_price, Num::new(1020, 1));
    assert_eq!(tsla.ask_size, 3);
    assert_eq!(tsla.bid_price, Num::new(990, 1));
    assert_eq!(tsla.bid_size, 5);
  }

  /// Verify that we can retrieve the last quote for an asset.
  #[test(tokio::test)]
  async fn request_last_quotes() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let req = GetReqInit::default().init(["SPY"]);
    let quotes = client.issue::<Get>(&req).await.unwrap();
    assert_eq!(quotes.len(), 1);
    assert_eq!(quotes[0].0, "SPY");
    // Just as a rough sanity check, we require that the reported time
    // is some time after two weeks before today. That should safely
    // account for any combination of holidays, weekends, etc.
    assert!(quotes[0].1.time >= Utc::now() - Duration::weeks(2));
  }

  /// Retrieve multiple symbols at once.
  #[test(tokio::test)]
  async fn request_last_quotes_multi() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let req = GetReqInit::default().init(["MSFT", "SPY", "AAPL"]);
    let quotes = client.issue::<Get>(&req).await.unwrap();
    assert_eq!(quotes.len(), 3);

    // We always guarantee lexical order of quotes by symbol.
    assert_eq!(quotes[0].0, "AAPL");
    assert!(quotes[0].1.time >= Utc::now() - Duration::weeks(2));
    assert_eq!(quotes[1].0, "MSFT");
    assert!(quotes[1].1.time >= Utc::now() - Duration::weeks(2));
    assert_eq!(quotes[2].0, "SPY");
    assert!(quotes[2].1.time >= Utc::now() - Duration::weeks(2));
  }

  /// Verify that we can specify the SIP feed as the data source to use.
  #[test(tokio::test)]
  async fn sip_feed() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let req = GetReqInit {
      feed: Some(Feed::SIP),
      ..Default::default()
    }
    .init(["SPY"]);

    let result = client.issue::<Get>(&req).await;
    // Unfortunately we can't really know whether the user has the
    // unlimited plan and can access the SIP feed. So really all we can
    // do here is accept both possible outcomes.
    match result {
      Ok(_) | Err(RequestError::Endpoint(GetError::NotPermitted(_))) => (),
      err => panic!("Received unexpected error: {err:?}"),
    }
  }

  /// Verify that we error out as expected when attempting to retrieve
  /// the last quote for an invalid symbol.
  #[test(tokio::test)]
  async fn invalid_symbol() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let req = GetReqInit::default().init(["ABC123"]);
    let err = client.issue::<Get>(&req).await.unwrap_err();
    match err {
      RequestError::Endpoint(GetError::InvalidInput(_)) => (),
      _ => panic!("Received unexpected error: {err:?}"),
    };
  }

  /// Check that a non-existent symbol is simply ignored in a request
  /// for multiple symbols.
  #[test(tokio::test)]
  async fn nonexistent_symbol() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let req = GetReqInit::default().init(["SPY", "NOSUCHSYMBOL"]);
    let quotes = client.issue::<Get>(&req).await.unwrap();
    assert_eq!(quotes.len(), 1);
  }
}
