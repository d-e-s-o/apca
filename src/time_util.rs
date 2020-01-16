// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::convert::TryInto;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use chrono::DateTime;
use chrono::offset::FixedOffset;
use chrono::offset::TimeZone;
use chrono::offset::Utc;
use chrono::ParseError;

use serde::de::Deserializer;
use serde::de::Error;
use serde::de::Unexpected;
use serde::ser::Serializer;
use serde::Deserialize;

type DateFn = fn(&str) -> Result<DateTime<FixedOffset>, ParseError>;

/// The list of time stamp formats we support.
const PARSE_FNS: [DateFn; 3] = [
  |s| FixedOffset::east(0).datetime_from_str(s, "%Y-%m-%dT%H:%M:%S%.fZ"),
  |s| FixedOffset::east(0).datetime_from_str(s, "%Y-%m-%dT%H:%M:%SZ"),
  |s| DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f%z"),
];


/// Parse a `SystemTime` from a string.
fn parse_system_time_from_str<'de, D>(time: &str) -> Result<SystemTime, D::Error>
where
  D: Deserializer<'de>,
{
  for parse_fn in &PARSE_FNS {
    // Ideally we would want to only continue in case of
    // ParseErrorKind::Invalid. However, that member is private...
    let datetime = match parse_fn(&time) {
      Ok(datetime) => datetime,
      Err(_) => continue,
    };

    let sec = datetime.timestamp();
    let nsec = datetime.timestamp_subsec_nanos();
    let systime = if sec < 0 {
      UNIX_EPOCH - Duration::new(-sec as u64, 0) + Duration::new(0, nsec)
    } else {
      UNIX_EPOCH + Duration::new(sec as u64, nsec)
    };
    return Ok(systime)
  }

  // TODO: Ideally we would want to somehow embed the last error we got
  //       into the error we emit.
  Err(Error::invalid_value(
    Unexpected::Str(&time),
    &"a time stamp",
  ))
}


/// Deserialize a time stamp as a `SystemTime`.
pub fn system_time_from_str<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
where
  D: Deserializer<'de>,
{
  let time = String::deserialize(deserializer)?;
  parse_system_time_from_str::<D>(&time)
}


/// Deserialize an optional time stamp.
pub fn optional_system_time_from_str<'de, D>(
  deserializer: D,
) -> Result<Option<SystemTime>, D::Error>
where
  D: Deserializer<'de>,
{
  match Option::<String>::deserialize(deserializer)? {
    Some(time) => Some(parse_system_time_from_str::<D>(&time)).transpose(),
    None => Ok(None),
  }
}


/// Deserialize a `SystemTime` from a UNIX time stamp.
pub fn system_time_from_secs<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
where
  D: Deserializer<'de>,
{
  let seconds = u64::deserialize(deserializer)?;
  let time = UNIX_EPOCH + Duration::new(seconds, 0);
  Ok(time)
}


/// Serialize a `SystemTime` into a RFC3339 time stamp.
pub fn system_time_to_rfc3339<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  let duration = time.duration_since(UNIX_EPOCH).unwrap();
  let secs = duration.as_secs().try_into().unwrap();
  let nanos = duration.subsec_nanos();
  let string = Utc.timestamp(secs, nanos).to_rfc3339();

  serializer.serialize_str(&string)
}

/// Serialize an optional `SystemTime` into a RFC3339 time stamp.
pub fn optional_system_time_to_rfc3339<S>(
  time: &Option<SystemTime>,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  match time {
    Some(time) => system_time_to_rfc3339(time, serializer),
    None => serializer.serialize_none(),
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::time::SystemTime;

  use serde::Deserialize;
  use serde::Serialize;
  use serde_json::Error as JsonError;
  use serde_json::from_str as from_json;
  use serde_json::to_string as to_json;


  #[derive(Debug, Deserialize)]
  struct Time {
    #[serde(deserialize_with = "system_time_from_str")]
    time: SystemTime,
  }

  #[test]
  fn deserialize_system_time_from_str() -> Result<(), JsonError> {
    let times = [
      r#"{"time": "2018-04-01T12:00:00Z"}"#,
      r#"{"time": "2018-04-01T12:00:00.000Z"}"#,
      r#"{"time": "2018-04-01T08:00:00.000-04:00"}"#,
    ];

    for json in &times {
      let time = from_json::<Time>(json)?;
      assert_eq!(time.time, UNIX_EPOCH + Duration::from_secs(1522584000));
    }
    Ok(())
  }


  #[derive(Debug, Deserialize, Serialize)]
  struct OtherTime {
    #[serde(
      deserialize_with = "system_time_from_secs",
      serialize_with = "system_time_to_rfc3339",
    )]
    time: SystemTime,
  }

  #[test]
  fn deserialize_system_time_from_secs() -> Result<(), JsonError> {
    let time = from_json::<OtherTime>(r#"{"time": 1544129220}"#)?;
    assert_eq!(time.time, UNIX_EPOCH + Duration::from_secs(1544129220));
    Ok(())
  }

  #[test]
  fn serialize_system_time_to_rfc3339() -> Result<(), JsonError> {
    let time = OtherTime {
      time: UNIX_EPOCH + Duration::from_secs(1544129220),
    };
    let json = to_json(&time)?;
    assert_eq!(json, r#"{"time":"2018-12-06T20:47:00+00:00"}"#);
    Ok(())
  }
}
