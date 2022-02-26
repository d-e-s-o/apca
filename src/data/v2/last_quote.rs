// Copyright (C) 2021-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::DateTime;
use chrono::Utc;

use num_decimal::Num;

use serde::Deserialize;

use serde_json::from_slice as from_json;

use crate::data::DATA_BASE_URL;
use crate::Str;


/// A quote bar as returned by the /v2/stocks/{symbol}/quotes/latest endpoint.
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
  /// The representation of a GET request to the /v2/stocks/{symbol}/quotes/latest endpoint.
  pub Get(String),
  Ok => Quote, [
    /// The last quote was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// The provided symbol was invalid or not found.
    /* 422 */ UNPROCESSABLE_ENTITY => InvalidInput,
  ]

  fn base_url() -> Option<Str> {
    Some(DATA_BASE_URL.into())
  }

  fn path(input: &Self::Input) -> Str {
    format!("/v2/stocks/{}/quotes/latest", input).into()
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

    let quote = client.issue::<Get>(&"SPY".to_string()).await.unwrap();
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

  /// Verify that we can properly parse a reference bar response.
  #[test(tokio::test)]
  async fn nonexistent_symbol() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let err = client
      .issue::<Get>(&"ABC123".to_string())
      .await
      .unwrap_err();
    match err {
      RequestError::Endpoint(GetError::InvalidInput(_)) => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
  }
}
