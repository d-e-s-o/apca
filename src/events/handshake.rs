// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;
use serde::Serialize;


/// An enumeration of the different event streams.
#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum StreamType {
  /// A stream for account updates.
  #[serde(rename = "account_updates")]
  AccountUpdates,
  /// A stream for trade updates.
  #[serde(rename = "trade_updates")]
  TradeUpdates,
}


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Streams {
  pub streams: Vec<StreamType>,
}

impl From<&[StreamType]> for Streams {
  fn from(src: &[StreamType]) -> Self {
    Self {
      streams: src.to_vec(),
    }
  }
}


/// Definitions for requests in the initial handshake.
mod req {
  use super::*;


  #[derive(Clone, Copy, Debug, Serialize)]
  pub struct Auth(&'static str);

  impl Default for Auth {
    fn default() -> Self {
      Self("authenticate")
    }
  }

  #[derive(Clone, Copy, Debug, Serialize)]
  pub struct Listen(&'static str);

  impl Default for Listen {
    fn default() -> Self {
      Self("listen")
    }
  }


  #[derive(Clone, Debug, Serialize)]
  struct Data<T>(T)
  where
    T: Serialize;


  #[derive(Clone, Debug, Serialize)]
  pub struct Request<A, T>
  where
    A: Default + Serialize,
    T: Serialize,
  {
    action: A,
    data: Data<T>,
  }

  impl<A, T> Request<A, T>
  where
    A: Default + Serialize,
    T: Serialize,
  {
    #[allow(unused)]
    pub fn new(data: T) -> Self {
      Self {
        action: Default::default(),
        data: Data(data),
      }
    }
  }


  #[derive(Clone, Debug, Serialize)]
  pub struct AuthData {
    key_id: String,
    secret_key: String,
  }

  impl AuthData {
    #[allow(unused)]
    pub fn new(key_id: String, secret_key: String) -> Self {
      Self { key_id, secret_key }
    }
  }
}


/// Definitions for responses in the initial handshake.
mod resp {
  use super::*;

  /// The current operation as used in a response.
  #[derive(Copy, Clone, Debug, Deserialize, PartialEq)]
  pub enum Operation {
    #[serde(rename = "listening")]
    Listening,
    #[serde(rename = "authorization")]
    Authorization,
  }

  #[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
  pub enum Status {
    #[serde(rename = "authorized")]
    Authorized,
    #[serde(rename = "unauthorized")]
    Unauthorized,
  }

  #[derive(Clone, Copy, Debug, Deserialize)]
  pub struct Result {
    pub status: Status,
    /*
     * TODO: Right now we just ignore the `action` field, as we would
     *       not react on it anyway.
     */
  }


  #[derive(Clone, Debug, Deserialize)]
  pub struct Data<T>(pub T);

  #[derive(Deserialize)]
  pub struct Response<T> {
    #[serde(rename = "stream")]
    pub op: Operation,
    #[serde(rename = "data")]
    pub data: Data<T>,
  }
}


#[allow(unused)]
type AuthRequest = req::Request<req::Auth, req::AuthData>;
#[allow(unused)]
type AuthResponse = resp::Response<resp::Result>;
#[allow(unused)]
type StreamRequest = req::Request<req::Listen, Streams>;
#[allow(unused)]
type StreamResponse = resp::Response<Streams>;


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_str as from_json;
  use serde_json::to_string as to_json;


  #[test]
  fn encode_auth_request() {
    let key_id = "some-key".to_string();
    let secret = "super-secret-secret".to_string();
    let expected = {
      r#"{"action":"authenticate","data":{"key_id":"some-key","secret_key":"super-secret-secret"}}"#
    };

    let auth = req::AuthData::new(key_id, secret);
    let request = AuthRequest::new(auth);
    let json = to_json(&request).unwrap();

    assert_eq!(json, expected)
  }

  #[test]
  fn encode_stream_request() {
    let expected = r#"{"action":"listen","data":{"streams":["trade_updates"]}}"#;
    let streams = [StreamType::TradeUpdates].as_ref().into();
    let request = StreamRequest::new(streams);
    let json = to_json(&request).unwrap();

    assert_eq!(json, expected)
  }

  #[test]
  fn decode_auth_response() {
    let json = {
      r#"{"stream":"authorization","data":{"status":"authorized","action":"authenticate"}}"#
    };
    let resp = from_json::<AuthResponse>(json).unwrap();
    assert_eq!(resp.op, resp::Operation::Authorization);
    assert_eq!(resp.data.0.status, resp::Status::Authorized);
  }

  #[test]
  fn decode_auth_response_unauthorized() {
    let json = {
      r#"{"stream":"authorization","data":{"status":"unauthorized","action":"listen"}}"#
    };
    let resp = from_json::<AuthResponse>(json).unwrap();
    assert_eq!(resp.op, resp::Operation::Authorization);
    assert_eq!(resp.data.0.status, resp::Status::Unauthorized);
  }

  #[test]
  fn decode_stream_response() {
    let json = r#"{"stream":"listening","data":{"streams":["trade_updates"]}}"#;

    let resp = from_json::<StreamResponse>(json).unwrap();
    assert_eq!(resp.op, resp::Operation::Listening);
    assert_eq!(resp.data.0.streams, vec![StreamType::TradeUpdates]);
  }
}