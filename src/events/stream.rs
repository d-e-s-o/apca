// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use futures::future::Either;
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
use serde_json::Error as JsonError;
use serde_json::from_slice as from_json;

use url::Url;

use websocket::client::r#async::ClientNew;
use websocket::ClientBuilder;
use websocket::OwnedMessage;
use websocket::stream::r#async::AsyncRead;
use websocket::stream::r#async::AsyncWrite;
use websocket::WebSocketError;

use crate::api_info::ApiInfo;
use crate::Error;
use crate::events::handshake::auth;
use crate::events::handshake::check_auth;
use crate::events::handshake::check_subscribe;
use crate::events::handshake::StreamType;
use crate::events::handshake::subscribe;


/// A trade representing a particular event stream.
pub trait EventStream {
  /// The events being reported through the stream.
  type Event: DeserializeOwned;

  /// The actual type of stream.
  fn stream() -> StreamType;
}


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
  /// A ping was received and we are about to issue a pong.
  Pong(Vec<u8>),
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
fn decode_msg<I>(msg: OwnedMessage) -> Result<Operation<I>, JsonError>
where
  I: DeserializeOwned,
{
  match msg {
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
    OwnedMessage::Ping(dat) => Ok(Operation::Pong(dat)),
    OwnedMessage::Pong(_) => Ok(Operation::Nop),
  }
}

/// Decode a single value from the client.
fn handle_msg<C, I>(
  client: C,
) -> impl Future<Item = (Result<Operation<I>, JsonError>, C), Error = Error>
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
      match msg {
        Some(msg) => ok((msg, c)),
        None => err(Error::Str("connection lost unexpectedly".into())),
      }
    })
    .and_then(|(msg, c)| {
      trace!("received message: {:?}", msg);
      // We have to jump through a shit ton of hoops just to be able to
      // respond to a ping. In a nutshell, because our code is
      // (supposed to be) independent of the reactor/event loop/whatever
      // you may call it, we can't really just "spawn" a task. There is
      // futures::executor::spawn but really its purpose is unclear,
      // given that one still has to poll the resulting task to drive it
      // to completion.
      // So either way, it appears that we are needlessly blocking the
      // actual request by waiting for the Pong to be sent, but then
      // that's only for Ping events, so that should not matter much.
      ok(decode_msg::<I>(msg))
        .and_then(|res| {
          match res {
            Ok(op) => {
              match op {
                Operation::Pong(dat) => {
                  let fut = c
                    .send(OwnedMessage::Pong(dat))
                    .map_err(Error::from)
                    .map(|c| (Ok(Operation::Nop), c));
                  Either::A(Either::A(fut))
                },
                op => {
                  let fut = ok((Ok(op), c));
                  Either::A(Either::B(fut))
                },
              }
            },
            Err(e) => Either::B(ok((Err(e), c))),
          }
        })
    })
}

/// Create a stream of higher level primitives out of a client, honoring
/// and filtering websocket control messages such as `Ping` and `Close`.
fn do_stream<C, I>(client: C) -> impl Stream<Item = Result<I, JsonError>, Error = Error>
where
  C: Stream<Item = OwnedMessage, Error = WebSocketError>,
  C: Sink<SinkItem = OwnedMessage, SinkError = WebSocketError>,
  I: DeserializeOwned,
{
  unfold((false, client), |(closed, c)| {
    if closed {
      None
    } else {
      let fut = handle_msg(c).map(|(res, c)| {
        let closed = res.as_ref().map(|op| op.is_close()).unwrap_or(false);

        (res, (closed, c))
      });
      Some(fut)
    }
  })
  .filter_map(|res| res.map(|op| op.into_decoded()).transpose())
}

fn stream_impl<F, S, I>(
  connect: F,
  api_info: ApiInfo,
  stream: StreamType,
) -> impl Future<Item = impl Stream<Item = Result<I, JsonError>, Error = Error>, Error = Error>
where
  F: FnOnce(Url) -> ClientNew<S>,
  S: AsyncRead + AsyncWrite,
  I: DeserializeOwned,
{
  let ApiInfo {
    base_url: mut url,
    key_id,
    secret,
  } = api_info;

  // At some point we adjusted the scheme from http(s) to ws(s), but
  // that seems to be unnecessary. The main problem is that it
  // introduces an additional error path because that step can fail.
  url.set_path("stream");

  debug!("connecting to {}", &url);

  connect(url)
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

/// Testing-only function to connect to a websocket stream in an
/// insecure manner.
#[cfg(test)]
fn stream_insecure<S>(
  api_info: ApiInfo,
) -> impl Future<Item = impl Stream<Item = Result<S::Event, JsonError>, Error = Error>, Error = Error>
where
  S: EventStream,
{
  let connect = |url| ClientBuilder::from_url(&url).async_connect_insecure();
  stream_impl(connect, api_info, S::stream())
}

/// Create a stream for decoded event data.
pub fn stream<S>(
  api_info: ApiInfo,
) -> impl Future<Item = impl Stream<Item = Result<S::Event, JsonError>, Error = Error>, Error = Error>
where
  S: EventStream,
{
  let connect = |url| ClientBuilder::from_url(&url).async_connect_secure(None);
  stream_impl(connect, api_info, S::stream())
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::net::SocketAddr;
  use std::net::TcpStream;
  use std::thread::JoinHandle;
  use std::thread::spawn;

  use test_env_log::test;

  use tokio::runtime::current_thread::block_on_all;

  use websocket::receiver::Reader;
  use websocket::sender::Writer;
  use websocket::sync::Server;


  const KEY_ID: &str = "USER12345678";
  const SECRET: &str = "justletmein";
  const AUTH_REQ: &str = {
    r#"{"action":"authenticate","data":{"key_id":"USER12345678","secret_key":"justletmein"}}"#
  };
  const AUTH_RESP: &str = {
    r#"{"stream":"authorization","data":{"action":"authenticate","status":"authorized"}}"#
  };
  const STREAM_REQ: &str = r#"{"action":"listen","data":{"streams":["account_updates"]}}"#;
  const STREAM_RESP: &str = r#"{"stream":"listening","data":{"streams":["account_updates"]}}"#;
  const UNIT_EVENT: &str = r#"{"stream":"account_updates","data":null}"#;


  /// A stream used solely for testing purposes.
  enum DummyStream {}

  impl EventStream for DummyStream {
    type Event = ();

    fn stream() -> StreamType {
      StreamType::AccountUpdates
    }
  }

  /// Create a websocket server that handles a customizable set of
  /// requests and exits.
  fn mock_server<F>(f: F) -> (SocketAddr, JoinHandle<()>)
  where
    F: Copy + FnOnce(Reader<TcpStream>, Writer<TcpStream>) + Send + 'static,
  {
    let mut server = Server::bind("localhost:0").unwrap();
    let addr = server.local_addr().unwrap();

    let handle = spawn(move || match server.accept() {
      Ok(conn) => {
        let client = conn.accept().unwrap();
        let (recv, send) = client.split().unwrap();

        f(recv, send)
      }
      Err(_) => panic!("nobody tried to connect"),
    });

    (addr, handle)
  }

  fn mock_stream<S, F>(
    f: F,
  ) -> (
    JoinHandle<()>,
    impl Future<Item = impl Stream<Item = Result<S::Event, JsonError>, Error = Error>, Error = Error>,
  )
  where
    S: EventStream,
    F: Copy + FnOnce(Reader<TcpStream>, Writer<TcpStream>) + Send + 'static,
  {
    let (addr, thrd) = mock_server(f);
    let api_info = ApiInfo {
      base_url: Url::parse(&format!("http://{}", addr.to_string())).unwrap(),
      key_id: KEY_ID.as_bytes().to_vec(),
      secret: SECRET.as_bytes().to_vec(),
    };

    (thrd, stream_insecure::<S>(api_info))
  }


  #[test]
  fn broken_stream() -> Result<(), Error> {
    let (_srv, fut) = mock_stream::<DummyStream, _>(|mut recv, _| {
      // Authentication. We receive the message but never respond.
      let _ = recv.recv_message().unwrap();
    });
    let fut = fut.and_then(|s| s.for_each(|_| ok(())));

    let err = block_on_all(fut).unwrap_err();
    match err {
      Error::Str(ref e) if e == "no response to authentication request" => (),
      e @ _ => panic!("received unexpected error: {}", e),
    }
    Ok(())
  }

  #[test]
  fn early_close() -> Result<(), Error> {
    let (_srv, fut) = mock_stream::<DummyStream, _>(|mut recv, mut send| {
      // Authentication.
      let msg = recv.recv_message().unwrap();
      assert_eq!(msg, OwnedMessage::Text(AUTH_REQ.to_string()));
      send
        .send_message(&OwnedMessage::Text(AUTH_RESP.to_string()))
        .unwrap();

      // Subscription.
      let msg = recv.recv_message().unwrap();
      assert_eq!(msg, OwnedMessage::Text(STREAM_REQ.to_string()));

      // Just respond with a Close.
      send.send_message(&OwnedMessage::Close(None)).unwrap();
    });
    let fut = fut.and_then(|s| s.for_each(|_| ok(())));

    let err = block_on_all(fut).unwrap_err();
    match err {
      Error::Str(ref e) if e.starts_with("received unexpected message: Close") => (),
      e @ _ => panic!("received unexpected error: {}", e),
    }
    Ok(())
  }

  #[test]
  fn no_messages() -> Result<(), Error> {
    let (_srv, fut) = mock_stream::<DummyStream, _>(|mut recv, mut send| {
      // Authentication.
      let msg = recv.recv_message().unwrap();
      assert_eq!(msg, OwnedMessage::Text(AUTH_REQ.to_string()));
      send
        .send_message(&OwnedMessage::Text(AUTH_RESP.to_string()))
        .unwrap();

      // Subscription.
      let msg = recv.recv_message().unwrap();
      assert_eq!(msg, OwnedMessage::Text(STREAM_REQ.to_string()));
      send
        .send_message(&OwnedMessage::Text(STREAM_RESP.to_string()))
        .unwrap();
    });
    let fut = fut.and_then(|s| s.for_each(|_| ok(())));

    let err = block_on_all(fut).unwrap_err();
    match err {
      Error::Str(ref e) if e.starts_with("connection lost unexpectedly") => (),
      e @ _ => panic!("received unexpected error: {}", e),
    }
    Ok(())
  }

  #[test]
  fn decode_error_during_handshake() -> Result<(), Error> {
    let (_srv, fut) = mock_stream::<DummyStream, _>(|mut recv, mut send| {
      // Authentication.
      let msg = recv.recv_message().unwrap();
      assert_eq!(msg, OwnedMessage::Text(AUTH_REQ.to_string()));
      send
        .send_message(&OwnedMessage::Text(AUTH_RESP.to_string()))
        .unwrap();

      // Some unexpected (and invalid) JSON.
      let update = "{ foobarbaz }";
      send
        .send_message(&OwnedMessage::Text(update.to_string()))
        .unwrap();
    });
    let fut = fut.and_then(|s| s.for_each(|_| ok(())));

    let err = block_on_all(fut).unwrap_err();
    match err {
      Error::Json(_) => (),
      e @ _ => panic!("received unexpected error: {}", e),
    }
    Ok(())
  }

  #[test]
  fn decode_error_errors_do_not_terminate() -> Result<(), Error> {
    let (_srv, fut) = mock_stream::<DummyStream, _>(|mut recv, mut send| {
      // Authentication.
      let msg = recv.recv_message().unwrap();
      assert_eq!(msg, OwnedMessage::Text(AUTH_REQ.to_string()));
      send
        .send_message(&OwnedMessage::Text(AUTH_RESP.to_string()))
        .unwrap();

      // Subscription.
      let msg = recv.recv_message().unwrap();
      assert_eq!(msg, OwnedMessage::Text(STREAM_REQ.to_string()));
      send
        .send_message(&OwnedMessage::Text(STREAM_RESP.to_string()))
        .unwrap();

      let update = "{ foobarbaz }";
      send
        .send_message(&OwnedMessage::Text(update.to_string()))
        .unwrap();
      send
        .send_message(&OwnedMessage::Text(UNIT_EVENT.to_string()))
        .unwrap();
      send.send_message(&OwnedMessage::Close(None)).unwrap();
    });
    let fut = fut.and_then(|s| s.for_each(|_| ok(())));

    let _ = block_on_all(fut).unwrap();
    Ok(())
  }

  #[test]
  fn ping_pong() -> Result<(), Error> {
    let (_srv, fut) = mock_stream::<DummyStream, _>(|mut recv, mut send| {
      // Authentication.
      let msg = recv.recv_message().unwrap();
      assert_eq!(msg, OwnedMessage::Text(AUTH_REQ.to_string()));
      send
        .send_message(&OwnedMessage::Text(AUTH_RESP.to_string()))
        .unwrap();

      // Subscription.
      let msg = recv.recv_message().unwrap();
      assert_eq!(msg, OwnedMessage::Text(STREAM_REQ.to_string()));
      send
        .send_message(&OwnedMessage::Text(STREAM_RESP.to_string()))
        .unwrap();

      // Ping.
      send.send_message(&OwnedMessage::Ping(Vec::new())).unwrap();

      // Expect Pong.
      let msg = recv.recv_message().unwrap();
      assert_eq!(msg, OwnedMessage::Pong(Vec::new()));

      send.send_message(&OwnedMessage::Close(None)).unwrap();
    });
    let fut = fut.and_then(|s| s.for_each(|_| ok(())));

    let _ = block_on_all(fut)?;
    Ok(())
  }
}
