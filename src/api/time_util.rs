// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use chrono::offset::TimeZone;
use chrono::Utc;

use serde::de::Deserializer;
use serde::de::Error;
use serde::de::Unexpected;
use serde::Deserialize;


/// The list of time stamp formats we support.
const FORMATS: [&str; 2] = [
  "%Y-%m-%dT%H:%M:%S%.fZ",
  "%Y-%m-%dT%H:%M:%SZ",
];


/// Parse a `SystemTime` from a string.
fn parse_system_time<'de, D>(time: &str) -> Result<SystemTime, D::Error>
where
  D: Deserializer<'de>,
{
  for format in &FORMATS {
    // Ideally we would want to only continue in case of
    // ParseErrorKind::Invalid. However, that member is private...
    let datetime = match Utc.datetime_from_str(&time, format) {
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
pub fn system_time<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
where
  D: Deserializer<'de>,
{
  let time = String::deserialize(deserializer)?;
  parse_system_time::<D>(&time)
}


/// Deserialize an optional time stamp.
pub fn optional_system_time<'de, D>(deserializer: D) -> Result<Option<SystemTime>, D::Error>
where
  D: Deserializer<'de>,
{
  match Option::<String>::deserialize(deserializer)? {
    Some(time) => Some(parse_system_time::<D>(&time)).transpose(),
    None => Ok(None),
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::time::SystemTime;

  use serde::Deserialize;
  use serde_json::Error as JsonError;
  use serde_json::from_str as from_json;


  #[derive(Debug, Deserialize)]
  struct Time {
    #[serde(deserialize_with = "system_time")]
    time: SystemTime,
  }

  #[test]
  fn deserialize_system_time() -> Result<(), JsonError> {
    let times = [
      r#"{"time": "2018-04-01T12:00:00Z"}"#,
      r#"{"time": "2018-04-01T12:00:00.000Z"}"#,
    ];

    for json in &times {
      let time = from_json::<Time>(json)?;
      assert_eq!(time.time, UNIX_EPOCH + Duration::from_secs(1522584000));
    }
    Ok(())
  }
}
