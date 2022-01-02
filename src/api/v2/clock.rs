// Copyright (C) 2019-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::DateTime;
use chrono::Utc;

use serde::Deserialize;

use crate::Str;


/// A type encapsulating market open/close timing information.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct Clock {
  /// An indication whether the market is currently open or not.
  #[serde(rename = "is_open")]
  pub open: bool,
  /// The current time.
  #[serde(rename = "timestamp")]
  pub current: DateTime<Utc>,
  /// The next market opening time stamp.
  #[serde(rename = "next_open")]
  pub next_open: DateTime<Utc>,
  /// The next market closing time stamp.
  #[serde(rename = "next_close")]
  pub next_close: DateTime<Utc>,
}


Endpoint! {
  /// The representation of a GET request to the /v2/clock endpoint.
  pub Get(()),
  Ok => Clock, [
    /// The clock object for the given symbol was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  #[inline]
  fn path(_input: &Self::Input) -> Str {
    "/v2/clock".into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use chrono::Duration;

  use serde_json::from_str as from_json;

  use test_log::test;

  use crate::api_info::ApiInfo;
  use crate::Client;


  #[test]
  fn parse_reference_clock() {
    let response = r#"{
  "timestamp": "2018-04-01T12:00:00.000Z",
  "is_open": true,
  "next_open": "2018-04-01T12:00:00.000Z",
  "next_close": "2018-04-01T12:00:00.000Z"
}"#;

    let clock = from_json::<Clock>(response).unwrap();
    assert!(clock.open);
  }

  #[test(tokio::test)]
  async fn current_market_clock() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let clock = client.issue::<Get>(&()).await.unwrap();

    // We want to sanitize the current time being reported at least to a
    // certain degree. For that we assume that our local time is
    // somewhat synchronized to "real" time and are asserting that the
    // current time reported by Alpaca is within one hour of our local
    // time (mainly to rule out wrong time zone handling).
    let now = Utc::now();
    assert!(now > clock.current - Duration::hours(1));
    assert!(now < clock.current + Duration::hours(1));

    assert!(clock.current < clock.next_open);
    assert!(clock.current < clock.next_close);

    if clock.open {
      assert!(clock.next_open > clock.next_close);
    } else {
      assert!(clock.next_open < clock.next_close);
    }
  }
}
