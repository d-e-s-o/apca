// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::SystemTime;

use serde::Deserialize;

use crate::api::time_util::system_time;
use crate::endpoint::Endpoint;
use crate::Str;


/// A type encapsulating market open/close timing information.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct Clock {
  /// An indication whether the market is currently open or not.
  #[serde(rename = "is_open")]
  open: bool,
  /// The current time.
  #[serde(rename = "timestamp", deserialize_with = "system_time")]
  current: SystemTime,
  /// The next market opening time stamp.
  #[serde(rename = "next_open", deserialize_with = "system_time")]
  next_open: SystemTime,
  /// The next market closing time stamp.
  #[serde(rename = "next_close", deserialize_with = "system_time")]
  next_close: SystemTime,
}

/// The representation of a GET request to the /v1/assets/<symbol> endpoint.
#[derive(Clone, Copy, Debug)]
pub struct Get {}

EndpointDef! {
  Get,
  Ok => Clock, [
    /// The clock object for the given symbol was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []
}

impl Endpoint for Get {
  type Input = ();
  type Output = Clock;
  type Error = GetError;

  fn path(_input: &Self::Input) -> Str {
    "/v1/clock".into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::time::Duration;

  use serde_json::from_str as from_json;

  use test_env_log::test;

  use tokio::runtime::current_thread::block_on_all;

  use crate::Client;
  use crate::Error;


  #[test]
  fn parse_reference_clock() {
    let response = r#"{
  "timestamp": "2018-04-01T12:00:00.000Z",
  "is_open": true,
  "next_open": "2018-04-01T12:00:00.000Z",
  "next_close": "2018-04-01T12:00:00.000Z"
}"#;

    let clock = from_json::<Clock>(&response).unwrap();
    assert_eq!(clock.open, true);
  }

  #[test]
  fn current_market_clock() -> Result<(), Error> {
    const SECS_IN_HOUR: u64 = 60 * 60;

    let client = Client::from_env()?;
    let future = client.issue::<Get>(())?;
    let clock = block_on_all(future)?;

    // We want to sanitize the current time being reported at least to a
    // certain degree. For that we assume that our local time is
    // somewhat synchronized to "real" time and are asserting that the
    // current time reported by Alpaca is within one hour of our local
    // time (mainly to rule out wrong time zone handling).
    let now = SystemTime::now();
    let hour = Duration::from_secs(SECS_IN_HOUR);
    assert!(now > clock.current - hour, "now: {}, current: {}");
    assert!(now < clock.current + hour, "now: {}, current: {}");

    assert!(clock.current < clock.next_open, "current: {}, open: {}");
    assert!(clock.current < clock.next_close, "current: {}, close: {}");

    if clock.open {
      assert!(clock.next_open > clock.next_close, "open: {}, close: {}");
    } else {
      assert!(clock.next_open < clock.next_close, "open: {}, close: {}");
    }
    Ok(())
  }
}
