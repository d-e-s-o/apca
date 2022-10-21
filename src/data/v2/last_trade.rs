// Copyright (C) 2021-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::DateTime;
use chrono::Utc;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;
use serde_json::from_slice as from_json;
use serde_urlencoded::to_string as to_query;
use std::collections::HashMap;

use crate::data::v2::Feed;
use crate::data::DATA_BASE_URL;
use crate::Str;

/// A GET request to be made to the /v2/stocks/{symbol}/trades/latest endpoint.
#[derive(Clone, Serialize, Eq, PartialEq, Debug)]
pub struct LastTradeRequest {
  /// Symbols to retrieve the last trade for, comma separated.
  pub symbols: String,
  /// The data feed to use.
  pub feed: Option<Feed>,
}

impl LastTradeRequest {
  /// Create a new LastTradeRequest.
  pub fn new(symbols: Vec<String>) -> Self {
    Self {
      symbols: symbols.join(",").into(),
      feed: None,
    }
  }
  /// Set the data feed to use.
  pub fn with_feed(mut self, feed: Feed) -> Self {
    self.feed = Some(feed);
    self
  }
}

/// A trade data point as returned by the /v2/stocks/{symbol}/trades/latest endpoint.
/// See
/// https://alpaca.markets/docs/api-references/market-data-api/stock-pricing-data/historical/#trade
// TODO: Not all fields are hooked up.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[non_exhaustive]
pub struct Trade {
  /// The time stamp of this trade.
  pub time: DateTime<Utc>,
  /// Trade price
  pub price: Num,
  /// Trade size
  pub size: u64,
  /// Symbol
  pub symbol: String,
}

impl Trade {
  fn from(symbol: &str, point: TradeDataPoint) -> Self {
    Self {
      time: point.t,
      price: point.p,
      size: point.s,
      symbol: symbol.to_string(),
    }
  }

  fn parse(body: &[u8]) -> Result<Vec<Trade>, serde_json::Error> {
    from_json::<LastTradeResponse>(body).map(|response| {
      response
        .trades
        .into_iter()
        .map(|(sym, point)| Trade::from(&sym, point))
        .collect()
    })
  }
}

/// fields for individual data points in the response JSON
#[derive(Clone, Debug, Deserialize)]
struct TradeDataPoint {
  t: DateTime<Utc>,
  p: Num,
  s: u64,
}

/// A representation of the JSON data in the response
#[derive(Deserialize)]
struct LastTradeResponse {
  trades: HashMap<String, TradeDataPoint>,
}

EndpointNoParse! {
  /// The representation of a GET request to the
  /// /v2/stocks/trades/latest endpoint.
  pub Get(LastTradeRequest),
  Ok => Vec<Trade>, [
    /// The last Trade was retrieved successfully.
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

  fn path(_input: &Self::Input) -> Str {
    "/v2/stocks/trades/latest".into()
  }

  fn query(input: &Self::Input) -> Result<Option<Str>, Self::ConversionError> {
    Ok(Some(to_query(input)?.into()))
  }

  fn parse(body: &[u8]) -> Result<Self::Output, Self::ConversionError> {
    Trade::parse(body).map_err(Self::ConversionError::from)
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

  /// Check that we can parse the reference trade from the
  /// documentation.
  #[test]
  fn parse_reference_trade() {
    let response = br#"{
			"trades": {
				"TSLA": {
					"t": "2022-04-12T17:05:06.936423531Z",
					"x": "V",
					"p": 995,
					"s": 100,
					"c": ["@"],
					"i": 10741,
					"z": "C"
				},
				"AAPL": {
					"t": "2022-04-12T17:05:17.428334819Z",
					"x": "V",
					"p": 167.86,
					"s": 99,
					"c": ["@"],
					"i": 7980,
					"z": "C"
				}
			}
		}"#;

    let mut result = Trade::parse(response).unwrap();
    result.sort_by_key(|t| t.time);
    assert_eq!(result.len(), 2);
    assert_eq!(result[1].price, Num::new(16786, 100));
    assert_eq!(result[1].size, 99);
    assert_eq!(result[1].symbol, "AAPL".to_string());
    assert_eq!(
      result[1].time,
      DateTime::parse_from_rfc3339("2022-04-12T17:05:17.428334819Z").unwrap()
    );
  }

  /// Verify that we can retrieve the last trade for an asset.
  #[test(tokio::test)]
  async fn request_last_trade() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let req = LastTradeRequest::new(vec!["SPY".to_string()]);
    let trades = client.issue::<Get>(&req).await.unwrap();
    // Just as a rough sanity check, we require that the reported time
    // is some time after two weeks before today. That should safely
    // account for any combination of holidays, weekends, etc.
    assert!(trades[0].time >= Utc::now() - Duration::weeks(2));
    // This test will fail if SPY goes below $1, but in that case a lot else is wrong with the world.
    assert!(trades[0].price >= Num::new(1, 1));
  }

  /// Retrieve multiple symbols at once.
  #[test(tokio::test)]
  async fn request_last_trades_multi() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let req = LastTradeRequest::new(vec![
      "SPY".to_string(),
      "QQQ".to_string(),
      "MSFT".to_string(),
    ]);
    let trades = client.issue::<Get>(&req).await.unwrap();
    assert_eq!(trades.len(), 3);
    assert!(trades[0].time >= Utc::now() - Duration::weeks(2));
    assert!(trades[0].price >= Num::new(1, 1));
  }

  /// Verify that we can specify the SIP feed as the data source to use.
  #[test(tokio::test)]
  async fn sip_feed() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let req = LastTradeRequest::new(vec!["SPY".to_string()]).with_feed(Feed::SIP);

    let result = client.issue::<Get>(&req).await;
    // Unfortunately we can't really know whether the user has the
    // unlimited plan and can access the SIP feed. So really all we can
    // do here is accept both possible outcomes.
    match result {
      Ok(_) | Err(RequestError::Endpoint(GetError::InvalidInput(_))) => (),
      err => panic!("Received unexpected error: {:?}", err),
    }
  }

  /// A bad symbol should not result in an error, but skips it in the result
  #[test(tokio::test)]
  async fn nonexistent_symbol() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let req = LastTradeRequest::new(vec!["BZZZZZZT".to_string(), "AAPL".to_string()]);
    let trades = client.issue::<Get>(&req).await.unwrap();
    assert_eq!(trades.len(), 1);
  }
}
