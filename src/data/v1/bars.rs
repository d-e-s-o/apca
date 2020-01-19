// Copyright (C) 2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::time::SystemTime;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;
use serde_urlencoded::to_string as to_query;

use time_util::optional_system_time_to_rfc3339;
use time_util::system_time_from_secs;

use crate::data::DATA_BASE_URL;
use crate::Str;


/// An enumeration of the various supported time frames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeFrame {
  /// A time frame of one minute.
  OneMinute,
  /// A time frame of five minutes.
  FiveMinutes,
  /// A time frame of fifteen minutes.
  FifteenMinutes,
  /// A time frame of one day.
  OneDay,
}

impl AsRef<str> for TimeFrame {
  fn as_ref(&self) -> &'static str {
    match *self {
      TimeFrame::OneMinute => "1Min",
      TimeFrame::FiveMinutes => "5Min",
      TimeFrame::FifteenMinutes => "15Min",
      TimeFrame::OneDay => "1D",
    }
  }
}


/// A response as returned by the /v2/account endpoint.
// TODO: Not all fields are hooked up.
// TODO: Strictly speaking the `symbols` member should be an array of
//       symbols separated by comma. However, because of the
//       braindeadnesses of Alpaca of not making that a true array, we
//       only support a single symbol right now.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BarReq {
  /// The symbol for which to retrieve market data.
  // TODO: It is not clear whether an `asset::Symbol` is what is
  //       supported here. That would be weird because both are
  //       independent APIs with differing versions, but who knows.
  #[serde(rename = "symbols")]
  pub symbol: String,
  /// The maximum number of bars to be returned for each symbol.
  ///
  /// It can be between 1 and 1000. Defaults to 100 if the provided
  /// value is 0.
  #[serde(rename = "limit")]
  pub limit: usize,
  /// Filter bars equal to or before this time.
  #[serde(rename = "end", serialize_with = "optional_system_time_to_rfc3339")]
  pub end: Option<SystemTime>,
}


/// A market data bar as returned by the /v1/bars/<timeframe> endpoint.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Bar {
  /// The beginning time of this bar.
  #[serde(rename = "t", deserialize_with = "system_time_from_secs")]
  pub time: SystemTime,
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


Endpoint! {
  /// The representation of a GET request to the /v1/bars/<timeframe> endpoint.
  pub Get((TimeFrame, BarReq)),
  Ok => HashMap<String, Vec<Bar>>, [
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
    let (timeframe, _) = input;
    format!("/v1/bars/{}", timeframe.as_ref()).into()
  }

  fn query(input: &Self::Input) -> Option<Str> {
    let (_, request) = input;
    // TODO: Realistically there should be no way for this unwrap to
    //       ever panic because our conversion to strings should not be
    //       fallible. But still, ideally we would not have to unwrap.
    Some(to_query(request).unwrap().into())
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::time::Duration;
  use std::time::UNIX_EPOCH;

  use http_endpoint::Endpoint;
  use http_endpoint::Error as EndpointError;

  use serde_json::from_str as from_json;

  use test_env_log::test;

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::Error;


  #[test]
  fn parse_reference_bars() {
    let response = r#"{
  "AAPL": [
    {
      "t": 1544129220,
      "o": 172.26,
      "h": 172.3,
      "l": 172.16,
      "c": 172.18,
      "v": 3892
    }
  ]
}"#;

    let bars = from_json::<<Get as Endpoint>::Output>(&response).unwrap();
    let aapl = bars.get("AAPL").unwrap();
    assert_eq!(aapl.len(), 1);
    assert_eq!(aapl[0].time, UNIX_EPOCH + Duration::from_secs(1544129220));
    assert_eq!(aapl[0].open, Num::new(17226, 100));
    assert_eq!(aapl[0].close, Num::new(17218, 100));
    assert_eq!(aapl[0].high, Num::new(1723, 10));
    assert_eq!(aapl[0].low, Num::new(17216, 100));
    assert_eq!(aapl[0].volume, 3892);
  }

  #[test(tokio::test)]
  async fn request_bars() -> Result<(), Error> {
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let end = UNIX_EPOCH + Duration::from_secs(1544132820);
    let request = BarReq {
      symbol: "AAPL".to_string(),
      limit: 2,
      end: Some(end),
    };
    let bars = client
      .issue::<Get>((TimeFrame::OneDay, request))
      .await
      .map_err(EndpointError::from)?;

    let aapl = bars.get("AAPL").unwrap();
    assert_eq!(aapl.len(), 2);
    assert_eq!(aapl[0].time, UNIX_EPOCH + Duration::from_secs(1543899600));
    assert_eq!(aapl[0].open, Num::new(18095, 100));
    assert_eq!(aapl[0].close, Num::new(17667, 100));
    assert_eq!(aapl[0].high, Num::new(1823899, 10000));
    assert_eq!(aapl[0].low, Num::new(17627, 100));
    assert_eq!(aapl[0].volume, 35659368);
    assert_eq!(aapl[1].time, UNIX_EPOCH + Duration::from_secs(1544072400));
    assert_eq!(aapl[1].open, Num::new(17176, 100));
    assert_eq!(aapl[1].close, Num::new(17477, 100));
    assert_eq!(aapl[1].high, Num::new(17478, 100));
    assert_eq!(aapl[1].low, Num::new(17042, 100));
    assert_eq!(aapl[1].volume, 38911135);
    Ok(())
  }

  #[test(tokio::test)]
  async fn request_bars_without_end() -> Result<(), Error> {
    let api_info = ApiInfo::from_env()?;
    let client = Client::new(api_info);
    let request = BarReq {
      symbol: "AAPL".to_string(),
      limit: 1,
      end: None,
    };
    let bars = client
      .issue::<Get>((TimeFrame::OneDay, request))
      .await
      .map_err(EndpointError::from)?;

    let now = SystemTime::now();
    // There may not be a time stamp available due to weekends and
    // holidays etc. But we assume to never have a gap larger than seven
    // days.
    let earlier = now - 7 * Duration::from_secs(86400);

    let aapl = bars.get("AAPL").unwrap();
    assert_eq!(aapl.len(), 1);
    assert!(aapl[0].time <= now, aapl[0].time);
    assert!(aapl[0].time >= earlier, aapl[0].time);
    Ok(())
  }
}
