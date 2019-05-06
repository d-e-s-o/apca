// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use futures::future::err;
use futures::future::Future;
use futures::future::ok;
use futures::sink::Sink;
use futures::stream::Stream;
use futures::stream::unfold;

use log::debug;
use log::trace;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::from_slice as from_json;

use url::Url;

use websocket::ClientBuilder;
use websocket::OwnedMessage;
use websocket::WebSocketError;

use crate::Error;
use crate::events::handshake::auth;
use crate::events::handshake::check_auth;
use crate::events::handshake::check_subscribe;
use crate::events::handshake::StreamType;
use crate::events::handshake::subscribe;


mod stream {
  use super::*;

  #[derive(Clone, Debug, Deserialize)]
  pub struct Data<T>(pub T);

  #[derive(Deserialize)]
  pub struct Event<T> {
    #[serde(rename = "stream")]
    pub stream: StreamType,
    #[serde(rename = "data")]
    pub data: Data<T>,
  }
}


fn handle_only_data_msg<F>(msg: OwnedMessage, f: F) -> Result<(), Error>
where
  F: FnOnce(&[u8]) -> Result<(), Error>,
{
  match msg {
    OwnedMessage::Text(text) => f(text.as_bytes()),
    OwnedMessage::Binary(data) => f(data.as_slice()),
    m => {
      let e = format!("received unexpected message: {:?}", m);
      Err(Error::Str(e.into()))
    },
  }
}


#[derive(Debug)]
enum Operation<T> {
  /// A value was decoded.
  Decode(T),
  /// We received a control message that we just ignore.
  Nop,
  /// The connection is supposed to be close.
  Close,
}

impl<T> Operation<T> {
  fn into_decoded(self) -> Option<T> {
    match self {
      Operation::Decode(dat) => Some(dat),
      _ => None,
    }
  }

  fn is_close(&self) -> bool {
    match self {
      Operation::Close => true,
      _ => false,
    }
  }
}


/// Convert a message into an `Operation`.
fn decode_msg<I>(msg: Option<OwnedMessage>) -> Result<Operation<I>, Error>
where
  I: DeserializeOwned,
{
  match msg {
    None => Err(Error::Str("connection lost unexpectedly".into())),
    Some(msg) => match msg {
      OwnedMessage::Close(_) => Ok(Operation::Close),
      OwnedMessage::Text(txt) => {
        // TODO: Strictly speaking we would need to check that the
        //       stream is the expected one.
        let resp = from_json::<stream::Event<I>>(txt.as_bytes())?;
        Ok(Operation::Decode(resp.data.0))
      },
      OwnedMessage::Binary(dat) => {
        let resp = from_json::<stream::Event<I>>(dat.as_slice())?;
        Ok(Operation::Decode(resp.data.0))
      },
      OwnedMessage::Ping(_dat) => {
        // TODO: Send back a pong.
        Ok(Operation::Nop)
      },
      OwnedMessage::Pong(_) => Ok(Operation::Nop),
    },
  }
}

/// Decode a single value from the client.
fn handle_msg<C, I>(client: C) -> impl Future<Item = (Operation<I>, C), Error = Error>
where
  C: Stream<Item = OwnedMessage, Error = WebSocketError>,
  C: Sink<SinkItem = OwnedMessage, SinkError = WebSocketError>,
  I: DeserializeOwned,
{
  client.into_future()
    // TODO: It is unclear whether a WebSocketError received at this
    //       point could potentially be due to a transient issue.
    .map_err(|(err, _c)| Error::from(err))
    .and_then(|(msg, c)| {
      trace!("received message: {:?}", msg);
      decode_msg(msg)
        .map(|op| (op, c))
    })
}

/// Create a stream of higher level primitives out of a client, honoring
/// and filtering websocket control messages such as `Ping` and `Close`.
fn do_stream<C, I>(client: C) -> impl Stream<Item = I, Error = Error>
where
  C: Stream<Item = OwnedMessage, Error = WebSocketError>,
  C: Sink<SinkItem = OwnedMessage, SinkError = WebSocketError>,
  I: DeserializeOwned,
{
  // TODO: It is an open question as to whether errors are handled
  //       gracefully or whether they actually terminate the stream as
  //       well. It appears to be the latter, which would be wrong.
  unfold((false, client), |(closed, c)| {
    if closed {
      None
    } else {
      let fut = handle_msg(c).map(|(op, c)| {
        let closed = op.is_close();
        (op, (closed, c))
      });
      Some(fut)
    }
  })
  .filter_map(|op| op.into_decoded())
}

#[allow(unused)]
fn stream<I>(
  api_base: Url,
  key_id: Vec<u8>,
  secret: Vec<u8>,
  stream: StreamType,
) -> impl Future<Item = impl Stream<Item = I, Error = Error>, Error = Error>
where
  I: DeserializeOwned,
{
  let mut url = api_base;
  // At some point we adjusted the scheme from http(s) to ws(s), but
  // that seems to be unnecessary. The main problem is that it
  // introduces an additional error path because that step can fail.
  url.set_path("stream");

  debug!("connecting to {}", &url);

  ClientBuilder::from_url(&url)
    .async_connect_secure(None)
    // We just ignore the headers that are sent along after the
    // connection is made. Alpaca does not seem to be using them,
    // really.
    .map(|(c, _)| c)
    .and_then(|c| auth(c, key_id, secret))
    .and_then(|c| c.into_future().map_err(|e| e.0))
    .map_err(Error::from)
    .and_then(|(m, c)| {
      match m {
        Some(msg) => {
          handle_only_data_msg(msg, check_auth)
            .map(|_| c)
            .into()
        },
        None => {
          err(Error::Str("no response to authentication request".into()))
        },
      }
    })
    .and_then(move |c| subscribe(c, stream).map_err(Error::from))
    .and_then(|c| c.into_future().map_err(|e| Error::from(e.0)))
    .and_then(move |(m, c)| {
      match m {
        Some(msg) => {
          handle_only_data_msg(msg, |dat| check_subscribe(dat, stream))
            .map(|_| c)
            .into()
        },
        None => {
          err(Error::Str("no response to subscription request".into()))
        },
      }
    })
		.and_then(|c| ok(do_stream::<_, I>(c)))
}
