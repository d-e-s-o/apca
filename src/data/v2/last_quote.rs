// Copyright (C) 2021-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::DateTime;
use chrono::Utc;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;
use serde_json::from_slice as from_json;
use serde_urlencoded::to_string as to_query;

use crate::data::v2::Feed;
use crate::data::DATA_BASE_URL;
use crate::Str;


/// A GET request to be made to the /v2/stocks/{symbol}/quotes/latest endpoint.
#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct LastQuoteReq {
  /// The symbol to retrieve the last quote for.
  #[serde(skip)]
  pub symbol: String,
  /// The data feed to use.
  #[serde(rename = "feed")]
  pub feed: Option<Feed>,
}


/// A helper for initializing [`LastQuoteReq`] objects.
#[derive(Clone, Debug, Default, PartialEq)]
#[allow(missing_copy_implementations)]
pub struct LastQuoteReqInit {
  /// See `LastQuoteReq::feed`.
  pub feed: Option<Feed>,
  #[doc(hidden)]
  pub _non_exhaustive: (),
}

impl LastQuoteReqInit {
  /// Create a [`LastQuoteReq`] from a `LastQuoteReqInit`.
  #[inline]
  pub fn init<S>(self, symbol: S) -> LastQuoteReq
  where
    S: Into<String>,
  {
    LastQuoteReq {
      symbol: symbol.into(),
      feed: self.feed,
    }
  }
}


/// A quote bar as returned by the /v2/stocks/<symbol>/quotes/latest endpoint.
// TODO: Not all fields are hooked up.
#[derive(Clone, Debug, Deserialize, PartialEq)]
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
  /// /v2/stocks/<symbol>/quotes/latest endpoint.
  pub Get(LastQuoteReq),
  Ok => Quote, [
    /// The last quote was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// The provided symbol was invalid or not found or the data feed is
    /// not supported.
    /* 422 */ UNPROCESSABLE_ENTITY => InvalidInput,
  ]

  fn base_url() -> Option<Str> {
    Some(DATA_BASE_URL.into())
  }

  fn path(input: &Self::Input) -> Str {
    format!("/v2/stocks/{}/quotes/latest", input.symbol).into()
  }

  fn query(input: &Self::Input) -> Result<Option<Str>, Self::ConversionError> {
    Ok(Some(to_query(input)?.into()))
  }

  fn parse(body: &[u8]) -> Result<Self::Output, Self::ConversionError> {
    /// A helper object for parsing the response to a `Get` request.
    #[derive(Deserialize)]
    struct Response {
      /// The symbol for which the quote was reported.
      #[allow(unused)]
      symbol: String,
      /// The quote belonging to the provided symbol.
      quote: Quote,
    }

    // We are not interested in the actual `Response` object. Clients
    // can keep track of what symbol they requested a quote for.
    from_json::<Response>(body)
      .map(|response| response.quote)
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

  use test_log::test;

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::RequestError;


  /// Check that we can parse the reference quote from the
  /// documentation.
  #[test]
  fn parse_reference_quote() {
    let response = br#"{
      "t": "2021-02-06T13:35:08.946977536Z",
      "ax": "C",
      "ap": 387.7,
      "as": 1,
      "bx": "N",
      "bp": 387.67,
      "bs": 1,
      "c": [
        "R"
      ]
}"#;

    let quote = from_json::<Quote>(response).unwrap();
    assert_eq!(
      quote.time,
      DateTime::parse_from_rfc3339("2021-02-06T13:35:08.946977536Z").unwrap()
    );
    assert_eq!(quote.ask_price, Num::new(3877, 10));
    assert_eq!(quote.ask_size, 1);
    assert_eq!(quote.bid_price, Num::new(38767, 100));
    assert_eq!(quote.bid_size, 1);
  }

  /// Verify that we can properly parse a reference bar response.
  #[test(tokio::test)]
  async fn request_last_quote() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let req = LastQuoteReqInit::default().init("SPY");
    let quote = client.issue::<Get>(&req).await.unwrap();
    // Just as a rough sanity check, we require that the reported time
    // is some time after two weeks before today. That should safely
    // account for any combination of holidays, weekends, etc.
    assert!(quote.time >= Utc::now() - Duration::weeks(2));
    assert!(quote.ask_price >= quote.bid_price);
    assert_ne!(quote.ask_price, Num::from(0));
    assert_ne!(quote.bid_price, Num::from(0));
    assert_ne!(quote.ask_size, 0);
    assert_ne!(quote.bid_size, 0);
  }

  /// Verify that we can specify the SIP feed as the data source to use.
  #[test(tokio::test)]
  async fn sip_feed() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let req = LastQuoteReq {
      symbol: "SPY".to_string(),
      feed: Some(Feed::SIP),
    };

    let result = client.issue::<Get>(&req).await;
    // Unfortunately we can't really know whether the user has the
    // unlimited plan and can access the SIP feed. So really all we can
    // do here is accept both possible outcomes.
    match result {
      Ok(_) | Err(RequestError::Endpoint(GetError::InvalidInput(_))) => (),
      err => panic!("Received unexpected error: {:?}", err),
    }
  }

  /// Verify that we can properly parse a reference bar response.
  #[test(tokio::test)]
  async fn nonexistent_symbol() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let req = LastQuoteReqInit::default().init("ABC123");
    let err = client.issue::<Get>(&req).await.unwrap_err();
    match err {
      RequestError::Endpoint(GetError::InvalidInput(_)) => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
  }
}
