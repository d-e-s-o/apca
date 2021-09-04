// Copyright (C) 2020-2021 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::DateTime;
use chrono::Utc;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;
use serde_urlencoded::to_string as to_query;

use crate::data::DATA_BASE_URL;
use crate::Str;

/// An enumeration of the various supported time frames.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum TimeFrame {
  /// A time frame of one minute.
  #[serde(rename = "1Min")]
  OneMinute,
  /// A time frame of one hour.
  #[serde(rename = "1Hour")]
  OneHour,
  /// A time frame of one day.
  #[serde(rename = "1Day")]
  OneDay,
}

/// An enumeration of the adjustment
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum Adjustment {
  /// Raw adjustment
  #[serde(rename = "raw")]
  Raw,
  /// Split adjustment
  #[serde(rename = "split")]
  Split,
  /// Dividend adjustment
  #[serde(rename = "dividend")]
  Dividend,
  /// All adjustments
  #[serde(rename = "all")]
  All,
}

/// A GET request to be issued to the /v2/stocks/{symbol} endpoint.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BarReq {
  /// The symbol for which to retrieve market data.
  #[serde(skip)]
  pub symbol: String,
  /// The maximum number of bars to be returned for each symbol.
  ///
  /// It can be between 1 and 10000. Defaults to 1000 if the provided
  /// value is None.
  #[serde(rename = "limit")]
  pub limit: Option<usize>,

  /// Filter bars equal to or after this time.
  #[serde(rename = "start")]
  pub start: DateTime<Utc>,

  /// Filter bars equal to or before this time.
  #[serde(rename = "end")]
  pub end: DateTime<Utc>,

  /// If provided we will pass a page token to continue where we left off.
  #[serde(rename = "page_token", skip_serializing_if = "Option::is_none")]
  pub page_token: Option<String>,

  /// The time frame for the bars.
  #[serde(rename = "timeframe")]
  pub timeframe: TimeFrame,

  /// The adjustment to use (defaults to raw)
  #[serde(rename = "adjustment")]
  pub adjustment: Option<Adjustment>,
}

/// A market data bar as returned by the /v2/stocks/{symbol} endpoint.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Bar {
  /// The beginning time of this bar.
  #[serde(rename = "t")]
  pub time: DateTime<Utc>,
  /// The open price.
  #[serde(rename = "o")]
  pub open: Num,
  /// The close price.
  #[serde(rename = "c")]
  pub close: Num,
  /// The highest price.
  #[serde(rename = "h")]
  pub high: Num,
  /// The lowest price.
  #[serde(rename = "l")]
  pub low: Num,
  /// The trading volume.
  #[serde(rename = "v")]
  pub volume: usize,
}

/// A collection of bars as returned by the API. This is one page of bars.
#[derive(Debug, Deserialize, PartialEq)]
pub struct Bars {
  /// The list of returned bars
  pub bars: Vec<Bar>,
  /// The symbol the bars correspond to
  pub symbol: String,
  /// The token to provide to a request to get the next page of bars for this request.
  pub next_page_token: Option<String>,
}

Endpoint! {
  /// The representation of a GET request to the /v2/stocks/{symbol}/bars/ endpoint.
  pub Get(BarReq),
  Ok => Bars, [
    /// The market data was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// One or more of the arguments are not well formed.
    /* 422 */ UNPROCESSABLE_ENTITY => InvalidArgument,
  ]

  fn base_url() -> Option<Str> {
    Some(DATA_BASE_URL.into())
  }

  fn path(input: &Self::Input) -> Str {
    format!("/v2/stocks/{}/bars", input.symbol).into()
  }

  fn query(input: &Self::Input) -> Result<Option<Str>, Self::ConversionError> {
    Ok(Some(to_query(input)?.into()))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use chrono::{NaiveDateTime, TimeZone};

  use http_endpoint::Endpoint;

  use serde_json::from_str as from_json;

  use test_env_log::test;

  use crate::api_info::ApiInfo;
  use crate::Client;

  #[test]
  fn parse_reference_bars() {
    let response = r#"{
  "symbol": "AAPL",
  "bars": [
    {
      "t": "2021-02-01T16:01:00Z",
      "o": 172.26,
      "h": 172.3,
      "l": 172.16,
      "c": 172.18,
      "v": 3892
    }
  ],
  "next_page_token": "123456"
}"#;

    let res = from_json::<<Get as Endpoint>::Output>(response).unwrap();
    let bars = res.bars;
    let expected_time = Utc.ymd(2021, 2, 1).and_hms_milli(16, 1, 0, 0);
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].time, expected_time);
    assert_eq!(bars[0].open, Num::new(17226, 100));
    assert_eq!(bars[0].close, Num::new(17218, 100));
    assert_eq!(bars[0].high, Num::new(1723, 10));
    assert_eq!(bars[0].low, Num::new(17216, 100));
    assert_eq!(bars[0].volume, 3892);
    assert_eq!(res.symbol, "AAPL".to_string());
    assert!(res.next_page_token.is_some())
  }

  #[test(tokio::test)]
  async fn request_bars() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let start = Utc.ymd(2018, 12, 3).and_hms_milli(21, 47, 0, 0);
    let end = Utc.ymd(2018, 12, 6).and_hms_milli(21, 47, 0, 0);
    let request = BarReq {
      symbol: "AAPL".to_string(),
      limit: Some(2),
      start,
      end,
      timeframe: TimeFrame::OneDay,
      page_token: None,
      adjustment: None,
    };
    let res = client.issue::<Get>(&request).await.unwrap();
    let bars = res.bars;

    assert_eq!(bars.len(), 2);
    assert_eq!(
      bars[0].time,
      DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(1543899600, 0), Utc)
    );
    assert_eq!(bars[0].open, Num::new(3619, 20));
    assert_eq!(bars[0].close, Num::new(17669, 100));
    assert_eq!(bars[0].high, Num::new(1823899, 10000));
    assert_eq!(bars[0].low, Num::new(17627, 100));
    assert_eq!(bars[0].volume, 41344313);
    assert_eq!(
      bars[1].time,
      DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(1544072400, 0), Utc)
    );
    assert_eq!(bars[1].open, Num::new(4294, 25));
    assert_eq!(bars[1].close, Num::new(4368, 25));
    assert_eq!(bars[1].high, Num::new(8739, 50));
    assert_eq!(bars[1].low, Num::new(8521, 50));
    assert_eq!(bars[1].volume, 43099506);
  }

  #[test(tokio::test)]
  async fn can_follow_pagination() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let start = Utc.ymd(2018, 12, 3).and_hms_milli(21, 47, 0, 0);
    let end = Utc.ymd(2018, 12, 7).and_hms_milli(21, 47, 0, 0);
    let mut request = BarReq {
      symbol: "AAPL".to_string(),
      limit: Some(2),
      start,
      end,
      timeframe: TimeFrame::OneDay,
      page_token: None,
      adjustment: None,
    };
    let mut res = client.issue::<Get>(&request).await.unwrap();
    let bars = res.bars;

    assert_eq!(bars.len(), 2);
    request.page_token = res.next_page_token;

    res = client.issue::<Get>(&request).await.unwrap();
    let new_bars = res.bars;

    assert_eq!(new_bars.len(), 1);
    assert!(new_bars[0].time > bars[1].time);
    assert!(res.next_page_token.is_none())
  }

  async fn request_with_adjustment(adjustment: Adjustment) -> Bars {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let start = Utc.ymd(2018, 12, 3).and_hms_milli(21, 47, 0, 0);
    let end = Utc.ymd(2018, 12, 4).and_hms_milli(21, 47, 0, 0);
    let request = BarReq {
      symbol: "AAPL".to_string(),
      limit: None,
      start,
      end,
      timeframe: TimeFrame::OneDay,
      page_token: None,
      adjustment: Some(adjustment),
    };
    client.issue::<Get>(&request).await.unwrap()
  }

  #[test(tokio::test)]
  async fn request_with_dividend_adjustment() {
    let res = request_with_adjustment(Adjustment::Dividend);
    let bars = res.await.bars;

    assert_eq!(bars.len(), 1);
    assert_eq!(
      bars[0].time,
      DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(1543899600, 0), Utc)
    );
    assert_eq!(bars[0].open, Num::new(43976877, 250000));
    assert_eq!(bars[0].close, Num::new(171766221, 1000000));
    assert_eq!(bars[0].high, Num::new(177307283, 1000000));
    assert_eq!(bars[0].low, Num::new(6854317, 40000));
    assert_eq!(bars[0].volume, 41344313);
  }

  #[test(tokio::test)]
  async fn request_with_all_adjustment() {
    let res = request_with_adjustment(Adjustment::All);
    let bars = res.await.bars;
    assert_eq!(bars.len(), 1);
    assert_eq!(
      bars[0].time,
      DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(1543899600, 0), Utc)
    );
    assert_eq!(bars[0].open, Num::new(43976877, 1000000));
    assert_eq!(bars[0].close, Num::new(8588311, 200000));
    assert_eq!(bars[0].high, Num::new(44326821, 1000000));
    assert_eq!(bars[0].low, Num::new(42839481, 1000000));
    assert_eq!(bars[0].volume, 165377252);
  }

  #[test(tokio::test)]
  async fn request_with_split_adjustment() {
    let res = request_with_adjustment(Adjustment::Split);
    let bars = res.await.bars;
    assert_eq!(bars.len(), 1);
    assert_eq!(
      bars[0].time,
      DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(1543899600, 0), Utc)
    );
    dbg!(&bars[0]);
    assert_eq!(bars[0].open, Num::new(3619, 80));
    assert_eq!(bars[0].close, Num::new(17669, 400));
    assert_eq!(bars[0].high, Num::new(1823899, 40000));
    assert_eq!(bars[0].low, Num::new(17627, 400));
    assert_eq!(bars[0].volume, 165377252);
  }
}
