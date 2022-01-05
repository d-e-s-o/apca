// Copyright (C) 2019-2021 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::str::from_utf8;

use futures::Sink;
use futures::SinkExt;
use futures::Stream;
use futures::StreamExt;
use futures::TryFutureExt;

use tracing::error;
use tracing::instrument;
use tracing::trace;

use serde::Deserialize;
use serde::Serialize;
use serde_json::from_slice as from_json;
use serde_json::to_string as to_json;

use websocket_util::tungstenite::Error as WebSocketError;
use websocket_util::tungstenite::Message;

use crate::Error;


/// An enumeration of the different event streams.
#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum StreamType {
  /// A stream for trade updates.
  #[serde(rename = "trade_updates")]
  TradeUpdates,
}


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Streams {
  pub streams: Vec<StreamType>,
}

impl From<&[StreamType]> for Streams {
  #[inline]
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
    #[inline]
    fn default() -> Self {
      Self("authenticate")
    }
  }

  #[derive(Clone, Copy, Debug, Serialize)]
  pub struct Listen(&'static str);

  impl Default for Listen {
    #[inline]
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
    #[inline]
    pub fn new(data: T) -> Self {
      Self {
        action: Default::default(),
        data: Data(data),
      }
    }
  }


  #[derive(Clone, Debug, Serialize)]
  pub struct AuthData<'d> {
    key_id: &'d str,
    secret_key: &'d str,
  }

  impl<'d> AuthData<'d> {
    #[inline]
    pub fn new(key_id: &'d str, secret_key: &'d str) -> Self {
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

type AuthRequest<'d> = req::Request<req::Auth, req::AuthData<'d>>;
type AuthResponse = resp::Response<resp::Result>;
type StreamRequest = req::Request<req::Listen, Streams>;
type StreamResponse = resp::Response<Streams>;

/// Authenticate with the streaming service.
async fn auth<S>(stream: &mut S, key_id: &str, secret: &str) -> Result<(), WebSocketError>
where
  S: Sink<Message, Error = WebSocketError> + Unpin,
{
  let auth = req::AuthData::new(key_id, secret);
  let request = AuthRequest::new(auth);
  let json = to_json(&request).unwrap();
  trace!(request = display(&json));

  stream
    .send(Message::text(json))
    .map_err(|e| {
      error!("failed to send stream auth request: {}", e);
      e
    })
    .await
}


/// Check the response to an authentication request.
fn check_auth(msg: &[u8]) -> Result<(), Error> {
  match from_utf8(msg) {
    Ok(s) => trace!(response = display(&s)),
    Err(b) => trace!(response = display(&b)),
  }

  match from_json::<AuthResponse>(msg) {
    Ok(resp) => match resp.op {
      resp::Operation::Authorization => match resp.data.0.status {
        resp::Status::Authorized => Ok(()),
        resp::Status::Unauthorized => Err(Error::Str("authentication not successful".into())),
      },
      op => {
        let e = format!("received unexpected stream operation: {:?}", op);
        Err(Error::Str(e.into()))
      },
    },
    Err(e) => Err(Error::from(e)),
  }
}

/// Subscribe to the given stream.
async fn subscribe_stream<S>(stream: &mut S, stream_type: StreamType) -> Result<(), WebSocketError>
where
  S: Sink<Message, Error = WebSocketError> + Unpin,
{
  let request = StreamRequest::new([stream_type].as_ref().into());
  let json = to_json(&request).unwrap();
  trace!(request = display(&json));

  stream
    .send(Message::text(json))
    .map_err(|e| {
      error!("failed to send stream subscribe request: {}", e);
      e
    })
    .await
}


/// Check the response to a stream subscription request.
fn check_subscribe(msg: &[u8], stream: StreamType) -> Result<(), Error> {
  match from_utf8(msg) {
    Ok(s) => trace!(response = display(&s)),
    Err(b) => trace!(response = display(&b)),
  }

  match from_json::<StreamResponse>(msg) {
    Ok(resp) => match &resp.data.0.streams[..] {
      &[s] if s == stream => Ok(()),
      &[] => {
        let e = format!("failed to subscribe to stream {:?}", stream);
        Err(Error::Str(e.into()))
      },
      s => {
        let s = s
          .iter()
          .map(|s| format!("{:?}", s))
          .collect::<Vec<_>>()
          .as_slice()
          .join(", ");
        let e = format!(
          "got subscription to unexpected stream(s): {} (expected: {:?})",
          s, stream
        );
        Err(Error::Str(e.into()))
      },
    },
    Err(e) => Err(Error::from(e)),
  }
}


fn handle_only_data_msg<F>(msg: Message, f: F) -> Result<(), Error>
where
  F: FnOnce(&[u8]) -> Result<(), Error>,
{
  match msg {
    Message::Text(text) => f(text.as_bytes()),
    Message::Binary(data) => f(data.as_slice()),
    m => {
      let e = format!("received unexpected message: {:?}", m);
      Err(Error::Str(e.into()))
    },
  }
}


#[instrument(level = "trace", skip(stream, key_id, secret))]
async fn authenticate<S>(stream: &mut S, key_id: &str, secret: &str) -> Result<(), Error>
where
  S: Sink<Message, Error = WebSocketError>,
  S: Stream<Item = Result<Message, WebSocketError>> + Unpin,
{
  auth(stream, key_id, secret).await?;
  let result = stream
    .next()
    .await
    .ok_or_else(|| Error::Str("no response to authentication request".into()))?;
  let msg = result?;

  handle_only_data_msg(msg, check_auth)?;
  Ok(())
}


#[instrument(level = "trace", skip(stream, stream_type))]
async fn subscribe<S>(stream: &mut S, stream_type: StreamType) -> Result<(), Error>
where
  S: Sink<Message, Error = WebSocketError>,
  S: Stream<Item = Result<Message, WebSocketError>> + Unpin,
{
  subscribe_stream(stream, stream_type).await?;
  let result = stream
    .next()
    .await
    .ok_or_else(|| Error::Str("no response to subscription request".into()))?;
  let msg = result?;

  handle_only_data_msg(msg, |dat| check_subscribe(dat, stream_type))?;
  Ok(())
}


/// Authenticate with and subscribe to an Alpaca event stream.
pub async fn handshake<S>(
  stream: &mut S,
  key_id: &str,
  secret: &str,
  stream_type: StreamType,
) -> Result<(), Error>
where
  S: Sink<Message, Error = WebSocketError>,
  S: Stream<Item = Result<Message, WebSocketError>> + Unpin,
{
  // Creating spans on the fly was not found to be working:
  // - if we `enter` explicitly we seemingly never exit and two
  //   subsequent spans appear as nested
  // - if we use `in_scope` the output somehow lacks the span's name
  // Because of that we use dedicated, `instrument`ed, functions.
  authenticate(stream, key_id, secret).await?;
  subscribe(stream, stream_type).await?;
  Ok(())
}
