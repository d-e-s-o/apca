// Copyright (C) 2022-2023 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ops::Range;

use chrono::NaiveDate;
use chrono::NaiveTime;

use serde::de::Error;
use serde::de::Unexpected;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde_urlencoded::to_string as to_query;

use crate::Str;


/// Deserialize a `NaiveTime` from a string.
fn deserialize_naive_time<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
where
  D: Deserializer<'de>,
{
  let string = String::deserialize(deserializer)?;
  NaiveTime::parse_from_str(&string, "%H:%M").map_err(|_| {
    Error::invalid_value(
      Unexpected::Str(&string),
      &"a time stamp string in format %H:%M",
    )
  })
}

/// Deserialize a `NaiveTime` from a string.
fn serialize_naive_time<S>(time: &NaiveTime, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  serializer.serialize_str(&time.format("%H:%M").to_string())
}


/// The market open and close times for a specific date.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpenClose {
  /// The date to which the below open a close times apply.
  #[serde(rename = "date")]
  pub date: NaiveDate,
  /// The time the market opens at.
  #[serde(
    rename = "open",
    deserialize_with = "deserialize_naive_time",
    serialize_with = "serialize_naive_time"
  )]
  pub open: NaiveTime,
  /// The time the market closes at.
  #[serde(
    rename = "close",
    deserialize_with = "deserialize_naive_time",
    serialize_with = "serialize_naive_time"
  )]
  pub close: NaiveTime,
}


/// A GET request to be made to the /v2/calendar endpoint.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CalendarReq {
  /// The (inclusive) start date of the range for which to retrieve
  /// calendar data.
  #[serde(rename = "start")]
  pub start: NaiveDate,
  /// The (exclusive) end date of the range for which to retrieve
  /// calendar data.
  // Note that Alpaca claims that the end date is inclusive as well. It
  // is not.
  #[serde(rename = "end")]
  pub end: NaiveDate,
}

impl From<Range<NaiveDate>> for CalendarReq {
  fn from(range: Range<NaiveDate>) -> Self {
    Self {
      start: range.start,
      end: range.end,
    }
  }
}


Endpoint! {
  /// The representation of a GET request to the /v2/calendar endpoint.
  pub Get(CalendarReq),
  Ok => Vec<OpenClose>, [
    /// The market open and close times were retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  fn path(_input: &Self::Input) -> Str {
    "/v2/calendar".into()
  }

  fn query(input: &Self::Input) -> Result<Option<Str>, Self::ConversionError> {
    Ok(Some(to_query(input)?.into()))
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use crate::api_info::ApiInfo;
  use crate::Client;

  use serde_json::from_slice as from_json;
  use serde_json::to_vec as to_json;

  use test_log::test;


  /// Check that we can serialize and deserialize an `OpenClose` object.
  #[test]
  fn serialize_deserialize_open_close() {
    let open_close = OpenClose {
      date: NaiveDate::from_ymd_opt(2020, 4, 9).unwrap(),
      open: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
      close: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
    };

    let json = to_json(&open_close).unwrap();
    assert_eq!(from_json::<OpenClose>(&json).unwrap(), open_close);
  }

  /// Check that we error out as expected when failing to parse an
  /// `OpenClose` object because the time format is unexpected.
  #[test]
  fn parse_open_close_unexpected_time() {
    let serialized = br#"{"date":"2020-04-09","open":"09:30:00","close":"16:00"}"#;
    let err = from_json::<OpenClose>(serialized).unwrap_err();
    assert!(err
      .to_string()
      .starts_with("invalid value: string \"09:30:00\""));
  }

  /// Check that we can serialize and deserialize a [`CalendarReq`].
  #[test]
  fn serialize_deserialize_calendar_request() {
    let request = CalendarReq {
      start: NaiveDate::from_ymd_opt(2020, 4, 6).unwrap(),
      end: NaiveDate::from_ymd_opt(2020, 4, 10).unwrap(),
    };

    let json = to_json(&request).unwrap();
    assert_eq!(from_json::<CalendarReq>(&json).unwrap(), request);
  }

  /// Check that we can retrieve the market calendar for a specific time
  /// frame.
  #[test(tokio::test)]
  async fn get() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = NaiveDate::from_ymd_opt(2020, 4, 6).unwrap();
    let end = NaiveDate::from_ymd_opt(2020, 4, 10).unwrap();
    let calendar = client
      .issue::<Get>(&CalendarReq::from(start..end))
      .await
      .unwrap();

    let expected = (6..10)
      .map(|day| OpenClose {
        date: NaiveDate::from_ymd_opt(2020, 4, day).unwrap(),
        open: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
        close: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
      })
      .collect::<Vec<_>>();

    assert_eq!(calendar, expected);
  }
}
