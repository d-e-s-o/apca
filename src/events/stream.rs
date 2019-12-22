// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use futures::compat::Future01CompatExt;
use futures01::future::Either;
use futures01::future::err;
use futures01::future::Future as Future01;
use futures01::future::ok;
use futures01::sink::Sink;
use futures01::stream::Stream;
use futures01::stream::unfold;

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


/// A trait representing a particular event stream.
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
) -> impl Future01<Item = (Result<Operation<I>, JsonError>, C), Error = WebSocketError>
where
  C: Stream<Item = OwnedMessage, Error = WebSocketError>,
  C: Sink<SinkItem = OwnedMessage, SinkError = WebSocketError>,
  I: DeserializeOwned,
{
  client.into_future()
    // TODO: It is unclear whether a WebSocketError received at this
    //       point could potentially be due to a transient issue.
    .map_err(|(err, _c)| err)
    .and_then(|(msg, c)| {
      match msg {
        Some(msg) => ok((msg, c)),
        None => err(WebSocketError::ProtocolError("connection lost unexpectedly")),
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
fn do_stream<C, I>(client: C) -> impl Stream<Item = Result<I, JsonError>, Error = WebSocketError>
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

async fn stream_impl<F, S, I>(
  connect: F,
  api_info: ApiInfo,
  stream: StreamType,
) -> Result<impl Stream<Item = Result<I, JsonError>, Error = WebSocketError>, Error>
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
    .compat()
    .await
}

/// Testing-only function to connect to a websocket stream in an
/// insecure manner.
#[cfg(test)]
async fn stream_insecure<S>(
  api_info: ApiInfo,
) -> Result<impl Stream<Item = Result<S::Event, JsonError>, Error = WebSocketError>, Error>
where
  S: EventStream,
{
  let connect = |url| ClientBuilder::from_url(&url).async_connect_insecure();
  stream_impl(connect, api_info, S::stream()).await
}

/// Create a stream for decoded event data.
pub async fn stream<S>(
  api_info: ApiInfo,
) -> Result<impl Stream<Item = Result<S::Event, JsonError>, Error = WebSocketError>, Error>
where
  S: EventStream,
{
  let connect = |url| ClientBuilder::from_url(&url).async_connect_secure(None);
  stream_impl(connect, api_info, S::stream()).await
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::net::SocketAddr;

  use test_env_log::test;

  use tokio::spawn;
  use tokio01::net::TcpStream;
  use tokio01::reactor::Handle;

  use websocket::client::r#async::Framed;
  use websocket::OwnedMessage;
  use websocket::r#async::MessageCodec;
  use websocket::r#async::Server;


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

  type WebSocketStream = Framed<TcpStream, MessageCodec<OwnedMessage>>;

  /// Create a websocket server that handles a customizable set of
  /// requests and exits.
  fn mock_server<F, R>(f: F) -> (SocketAddr, impl Future01<Item = (), Error = ()>)
  where
    F: Copy + FnOnce(WebSocketStream) -> R + 'static,
    R: Future01<Item = (), Error = WebSocketError> + 'static,
  {
    let server = Server::bind("localhost:0", &Handle::default()).unwrap();
    let addr = server.local_addr().unwrap();

    let future = server
      .incoming()
      .take(1)
      .and_then(move |(upgrade, _addr)| {
        upgrade
          .accept()
          .and_then(move |(stream, _headers)| f(stream))
          .map_err(|e| panic!(e))
      })
      .map_err(|e| panic!(e))
      .for_each(|_| ok(()));

    (addr, future)
  }

  async fn mock_stream<S, F, R>(
    f: F,
  ) -> Result<impl Stream<Item = Result<S::Event, JsonError>, Error = WebSocketError>, Error>
  where
    S: EventStream,
    F: Copy + FnOnce(WebSocketStream) -> R + Send + 'static,
    R: Future01<Item = (), Error = WebSocketError> + Send + 'static,
  {
    let (addr, srv_fut) = mock_server(f);
    let api_info = ApiInfo {
      base_url: Url::parse(&format!("http://{}", addr.to_string())).unwrap(),
      key_id: KEY_ID.as_bytes().to_vec(),
      secret: SECRET.as_bytes().to_vec(),
    };

    let _ = spawn(srv_fut.compat());
    stream_insecure::<S>(api_info).await
  }

  fn expect_msg<C>(
    stream: C,
    expected: OwnedMessage,
  ) -> impl Future01<Item = C, Error = WebSocketError>
  where
    C: Stream<Item = OwnedMessage, Error = WebSocketError>,
  {
    stream
      .into_future()
      .map(move |(msg, stream)| {
        assert_eq!(msg, Some(expected));
        stream
      })
      .map_err(|(e, _)| e)
  }

  #[test(tokio::test)]
  async fn broken_stream() {
    let result = mock_stream::<DummyStream, _, _>(|stream| {
      ok(stream)
        // Authentication. We receive the message but never respond.
        .and_then(|stream| expect_msg(stream, OwnedMessage::Text(AUTH_REQ.to_string())))
        .map(|_| ())
    })
    .await;

    match result {
      Ok(_) => panic!("authentication succeeded unexpectedly"),
      Err(Error::Str(ref e)) if e == "no response to authentication request" => (),
      Err(e) => panic!("received unexpected error: {}", e),
    }
  }

  #[test(tokio::test)]
  async fn early_close() {
    let result = mock_stream::<DummyStream, _, _>(|stream| {
      ok(stream)
        // Authentication.
        .and_then(|stream| expect_msg(stream, OwnedMessage::Text(AUTH_REQ.to_string())))
        .and_then(|stream| stream.send(OwnedMessage::Text(AUTH_RESP.to_string())))
        // Subscription.
        .and_then(|stream| expect_msg(stream, OwnedMessage::Text(STREAM_REQ.to_string())))
        // Just respond with a Close.
        .and_then(|stream| stream.send(OwnedMessage::Close(None)))
        .map(|_| ())
    })
    .await;

    match result {
      Ok(_) => panic!("operation succeeded unexpectedly"),
      Err(Error::Str(ref e)) if e.starts_with("received unexpected message: Close") => (),
      Err(e) => panic!("received unexpected error: {}", e),
    }
  }

  #[test(tokio::test)]
  async fn no_messages() {
    let stream = mock_stream::<DummyStream, _, _>(|stream| {
      ok(stream)
        // Authentication.
        .and_then(|stream| expect_msg(stream, OwnedMessage::Text(AUTH_REQ.to_string())))
        .and_then(|stream| stream.send(OwnedMessage::Text(AUTH_RESP.to_string())))
        // Subscription.
        .and_then(|stream| expect_msg(stream, OwnedMessage::Text(STREAM_REQ.to_string())))
        .and_then(|stream| stream.send(OwnedMessage::Text(STREAM_RESP.to_string())))
        .map(|_| ())
    })
    .await
    .unwrap();

    let err = stream
      .map_err(Error::from)
      .for_each(|_| ok(()))
      .compat()
      .await
      .unwrap_err();
    match err {
      Error::WebSocket(e) => match e {
        WebSocketError::ProtocolError(s) if s.starts_with("connection lost unexpectedly") => (),
        e => panic!("received unexpected error: {}", e),
      },
      e => panic!("received unexpected error: {}", e),
    }
  }

  #[test(tokio::test)]
  async fn decode_error_during_handshake() {
    let result = mock_stream::<DummyStream, _, _>(|stream| {
      ok(stream)
        // Authentication.
        .and_then(|stream| expect_msg(stream, OwnedMessage::Text(AUTH_REQ.to_string())))
        .and_then(|stream| stream.send(OwnedMessage::Text(AUTH_RESP.to_string())))
        .and_then(|stream| stream.send(OwnedMessage::Text("{ foobarbaz }".to_string())))
        .map(|_| ())
    })
    .await;

    match result {
      Ok(_) => panic!("operation succeeded unexpectedly"),
      Err(Error::Json(_)) => (),
      Err(e) => panic!("received unexpected error: {}", e),
    }
  }

  #[test(tokio::test)]
  async fn decode_error_errors_do_not_terminate() {
    let stream = mock_stream::<DummyStream, _, _>(|stream| {
      ok(stream)
        // Authentication.
        .and_then(|stream| expect_msg(stream, OwnedMessage::Text(AUTH_REQ.to_string())))
        .and_then(|stream| stream.send(OwnedMessage::Text(AUTH_RESP.to_string())))
        // Subscription.
        .and_then(|stream| expect_msg(stream, OwnedMessage::Text(STREAM_REQ.to_string())))
        .and_then(|stream| stream.send(OwnedMessage::Text(STREAM_RESP.to_string())))
        .and_then(|stream| stream.send(OwnedMessage::Text("{ foobarbaz }".to_string())))
        .and_then(|stream| stream.send(OwnedMessage::Text(UNIT_EVENT.to_string())))
        .and_then(|stream| stream.send(OwnedMessage::Close(None)))
        .map(|_| ())
    })
    .await
    .unwrap();

    let _ = stream
      .map_err(Error::from)
      .for_each(|_| ok(()))
      .compat()
      .await
      .unwrap();
  }

  #[test(tokio::test)]
  async fn ping_pong() {
    let stream = mock_stream::<DummyStream, _, _>(|stream| {
      ok(stream)
        // Authentication.
        .and_then(|stream| expect_msg(stream, OwnedMessage::Text(AUTH_REQ.to_string())))
        .and_then(|stream| stream.send(OwnedMessage::Text(AUTH_RESP.to_string())))
        // Subscription.
        .and_then(|stream| expect_msg(stream, OwnedMessage::Text(STREAM_REQ.to_string())))
        .and_then(|stream| stream.send(OwnedMessage::Text(STREAM_RESP.to_string())))
        // Ping.
        .and_then(|stream| stream.send(OwnedMessage::Ping(Vec::new())))
        // Expect Pong.
        .and_then(|stream| expect_msg(stream, OwnedMessage::Pong(Vec::new())))
        .and_then(|stream| stream.send(OwnedMessage::Close(None)))
        .map(|_| ())
    })
    .await
    .unwrap();

    let _ = stream
      .map_err(Error::from)
      .for_each(|_| ok(()))
      .compat()
      .await
      .unwrap();
  }
}
