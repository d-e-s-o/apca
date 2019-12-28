// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::future::Future;

use async_std::net::TcpStream;
use async_tls::TlsConnector;

use futures::future::Either;
use futures::future::err;
use futures::future::ok;
use futures::future::ready;
use futures::FutureExt;
use futures::sink::Sink;
use futures::SinkExt;
use futures::stream::Stream;
use futures::stream::unfold;
use futures::StreamExt;
use futures::TryFutureExt;
use futures::TryStreamExt;

use log::debug;
use log::trace;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Error as JsonError;
use serde_json::from_slice as from_json;

use url::Url;

#[cfg(test)]
use tungstenite::connect_async;
use tungstenite::connect_async_with_tls_connector;
use tungstenite::MaybeTlsStream;
use tungstenite::tungstenite::Error as WebSocketError;
use tungstenite::tungstenite::handshake::client::Response;
use tungstenite::tungstenite::Message;
use tungstenite::WebSocketStream as TungsteniteStream;

use crate::api_info::ApiInfo;
use crate::Error;
use crate::events::handshake::auth;
use crate::events::handshake::check_auth;
use crate::events::handshake::check_subscribe;
use crate::events::handshake::StreamType;
use crate::events::handshake::subscribe;

pub type WebSocketStream = TungsteniteStream<MaybeTlsStream<TcpStream>>;

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
fn decode_msg<I>(msg: Message) -> Result<Operation<I>, JsonError>
where
  I: DeserializeOwned,
{
  match msg {
    Message::Close(_) => Ok(Operation::Close),
    Message::Text(txt) => {
      // TODO: Strictly speaking we would need to check that the
      //       stream is the expected one.
      let resp = from_json::<stream::Event<I>>(txt.as_bytes())?;
      Ok(Operation::Decode(resp.data.0))
    },
    Message::Binary(dat) => {
      let resp = from_json::<stream::Event<I>>(dat.as_slice())?;
      Ok(Operation::Decode(resp.data.0))
    },
    Message::Ping(dat) => Ok(Operation::Pong(dat)),
    Message::Pong(_) => Ok(Operation::Nop),
  }
}

/// Decode a single value from the client.
fn handle_msg<I>(
  stream: WebSocketStream,
) -> impl Future<Output = (Result<Result<Operation<I>, JsonError>, WebSocketError>, WebSocketStream)>
where
  I: DeserializeOwned,
{
  stream
    .into_future()
    // TODO: It is unclear whether a WebSocketError received at this
    //       point could potentially be due to a transient issue.
    .then(|(option, stream)| {
      let result = option
        .unwrap_or_else(|| Err(WebSocketError::Protocol("connection lost unexpectedly".into())));

      ready((result, stream))
    })
    .then(|(result, stream)| {
      match result {
        Ok(msg) => {
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
          ready((Err(WebSocketError::Protocol("test".into())), stream))
          //let future = match decode_msg::<I>(msg) {
          //  Ok(op) => {
          //    match op {
          //      Operation::Pong(dat) => {
          //        let fut = stream
          //          .send(Message::Pong(dat))
          //          .map(|stream| ready((Ok(Operation::Nop), stream)));

          //        Either::Left(Either::Left(fut))
          //      },
          //      op => {
          //        let fut = ready((Ok(op), stream));
          //        Either::Left(Either::Right(fut))
          //      },
          //    }
          //  },
          //  Err(e) => Either::Right(ready((Err(e), stream))),
          //};
          //future
        },
        Err(e) => ready((Err(e), stream)),
      }
    })
}

///// Handle a single message from the stream.
//async fn handle_msg<I>(
//  mut stream: WebSocketStream,
//) -> Result<(Result<Operation<I>, JsonError>, WebSocketStream), WebSocketError>
//where
//  I: DeserializeOwned,
//{
//  // TODO: It is unclear whether a WebSocketError received at this
//  //       point could potentially be due to a transient issue.
//  let result = stream
//    .next()
//    .await
//    .ok_or_else(|| WebSocketError::Protocol("connection lost unexpectedly".into()))?;
//  let msg = result?;
//
//  trace!("received message: {:?}", msg);
//  let result = decode_msg::<I>(msg);
//  match result {
//    Ok(op) => {
//      if let Operation::Pong(dat) = op {
//        // TODO: We should probably spawn a task here.
//        stream.send(Message::Pong(dat)).await?;
//        Ok((Ok(Operation::Nop), stream))
//      } else {
//        Ok((Ok(op), stream))
//      }
//    },
//    e @ Err(_) => Ok((e, stream)),
//  }
//}

/// Create a stream of higher level primitives out of a client, honoring
/// and filtering websocket control messages such as `Ping` and `Close`.
async fn do_stream<I>(
  stream: WebSocketStream,
) -> impl Stream<Item = Result<Result<I, JsonError>, WebSocketError>> + Unpin
where
  I: DeserializeOwned,
{
  unfold((false, stream), |(closed, mut stream)| {
    if closed {
      Either::Left(ready(None))
    } else {
      let result = handle_msg(stream)
        .then(|(result, stream)| {
          let closed = match result.as_ref() {
            Ok(Ok(op)) => op.is_close(),
            _ => false,
          };
          ready(Some((result, (closed, stream))))
        });
      Either::Right(result)
    }
  })
  .try_filter_map(|res| ready(Ok(res.map(|op| op.into_decoded()).transpose())))
}

async fn stream_impl<F, R, I>(
  connect: F,
  api_info: ApiInfo,
  stream_type: StreamType,
) -> Result<impl Stream<Item = Result<Result<I, JsonError>, WebSocketError>> + Unpin, Error>
where
  F: FnOnce(Url) -> R,
  R: Future<Output = Result<(WebSocketStream, Response), WebSocketError>>,
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

  // We just ignore the response & headers that are sent along after
  // the connection is made. Alpaca does not seem to be using them,
  // really.
  let (mut stream, _response) = connect(url).await?;

  // Authentication.
  // TODO: Can we club the message handling into `auth`? In fact, can we
  //       just have a `handshake` and be done?
  auth(&mut stream, key_id, secret).await?;
  let result = stream
    .next()
    .await
    .ok_or_else(|| Error::Str("no response to authentication request".into()))?;
  let msg = result?;

  handle_only_data_msg(msg, check_auth)?;

  // Subscription.
  subscribe(&mut stream, stream_type).await?;
  let result = stream
    .next()
    .await
    .ok_or_else(|| Error::Str("no response to subscription request".into()))?;
  let msg = result?;

  handle_only_data_msg(msg, |dat| check_subscribe(dat, stream_type))?;

  let stream = do_stream::<I>(stream).await;
  Ok(stream)
}

/// Testing-only function to connect to a websocket stream in an
/// insecure manner.
#[cfg(test)]
async fn stream_insecure<S>(
  api_info: ApiInfo,
) -> Result<impl Stream<Item = Result<Result<S::Event, JsonError>, WebSocketError>> + Unpin, Error>
where
  S: EventStream,
{
  // TODO: Ideally we'd want to establish a TCP connection ourselves and
  //       use `client_async_tls_with_connector`. See implementation of
  //       `connect_async_with_tls_connector_and_config`.
  let connect = |url| connect_async(url);
  stream_impl(connect, api_info, S::stream()).await
}

/// Create a stream for decoded event data.
pub async fn stream<S>(
  api_info: ApiInfo,
) -> Result<impl Stream<Item = Result<Result<S::Event, JsonError>, WebSocketError>> + Unpin, Error>
where
  S: EventStream,
{
  let connect = |url| {
    let connector = TlsConnector::default();
    connect_async_with_tls_connector(url, Some(connector))
  };
  stream_impl(connect, api_info, S::stream()).await
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::future::Future;
  use std::net::SocketAddr;

  use async_std::net::TcpListener;
  use async_std::net::TcpStream;

  use futures::future::ready;
  use futures::FutureExt;
  use futures::SinkExt;
  use futures::StreamExt;
  use futures::TryStreamExt;

  use test_env_log::test;

  use tokio::spawn;

  use tungstenite::accept_async as accept_websocket;
  use tungstenite::WebSocketStream as WsStream;

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

  type WebSocketStream = WsStream<TcpStream>;

  /// Create a websocket server that handles a customizable set of
  /// requests and exits.
  async fn mock_server<F, R>(f: F) -> SocketAddr
  where
    F: Copy + FnOnce(WebSocketStream) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), WebSocketError>> + Send + Sync + 'static,
  {
    // TODO: Does `tungstenite` provide a short hand for this stuff?
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let future = async move {
      listener
        .accept()
        .map(move |result| result.unwrap())
        .then(|(stream, _addr)| accept_websocket(stream))
        .map(move |result| result.unwrap())
        .then(move |ws_stream| f(ws_stream))
        .await
    };

    let _ = spawn(future);
    addr
  }

  async fn mock_stream<S, F, R>(
    f: F,
  ) -> Result<impl Stream<Item = Result<Result<S::Event, JsonError>, WebSocketError>>, Error>
  where
    S: EventStream,
    F: Copy + FnOnce(WebSocketStream) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), WebSocketError>> + Send + Sync + 'static,
  {
    let addr = mock_server(f).await;
    let api_info = ApiInfo {
      base_url: Url::parse(&format!("http://{}", addr.to_string())).unwrap(),
      key_id: KEY_ID.as_bytes().to_vec(),
      secret: SECRET.as_bytes().to_vec(),
    };

    stream_insecure::<S>(api_info).await
  }

  #[test(tokio::test)]
  async fn broken_stream() {
    async fn test(mut stream: WebSocketStream) -> Result<(), WebSocketError> {
      let msg = stream.next().await.unwrap()?;
      assert_eq!(msg, Message::Text(AUTH_REQ.to_string()));
      Ok(())
    }

    let result = mock_stream::<DummyStream, _, _>(test).await;
    match result {
      Ok(_) => panic!("authentication succeeded unexpectedly"),
      Err(Error::Str(ref e)) if e == "no response to authentication request" => (),
      Err(e) => panic!("received unexpected error: {}", e),
    }
  }

  #[test(tokio::test)]
  async fn early_close() {
    async fn test(mut stream: WebSocketStream) -> Result<(), WebSocketError> {
      // Authentication.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(AUTH_REQ.to_string()),
      );
      stream.send(Message::Text(AUTH_RESP.to_string())).await?;

      // Subscription.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(STREAM_REQ.to_string()),
      );
      // Just respond with a Close.
      stream.send(Message::Close(None)).await?;
      Ok(())
    }

    let result = mock_stream::<DummyStream, _, _>(test).await;
    match result {
      Ok(_) => panic!("operation succeeded unexpectedly"),
      Err(Error::Str(ref e)) if e.starts_with("received unexpected message: Close") => (),
      Err(e) => panic!("received unexpected error: {}", e),
    }
  }

  #[test(tokio::test)]
  async fn no_messages() {
    async fn test(mut stream: WebSocketStream) -> Result<(), WebSocketError> {
      // Authentication.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(AUTH_REQ.to_string()),
      );
      stream.send(Message::Text(AUTH_RESP.to_string())).await?;

      // Subscription.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(STREAM_REQ.to_string()),
      );
      stream.send(Message::Text(STREAM_RESP.to_string())).await?;
      Ok(())
    }

    let stream = mock_stream::<DummyStream, _, _>(test).await.unwrap();
    let err = stream
      .map_err(Error::from)
      .try_for_each(|_| ready(Ok(())))
      .await
      .unwrap_err();

    match err {
      Error::WebSocket(e) => match e {
        WebSocketError::Protocol(s) if s.starts_with("connection lost unexpectedly") => (),
        e => panic!("received unexpected error: {}", e),
      },
      e => panic!("received unexpected error: {}", e),
    }
  }

  #[test(tokio::test)]
  async fn decode_error_during_handshake() {
    async fn test(mut stream: WebSocketStream) -> Result<(), WebSocketError> {
      // Authentication.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(AUTH_REQ.to_string()),
      );
      stream.send(Message::Text(AUTH_RESP.to_string())).await?;

      stream
        .send(Message::Text("{ foobarbaz }".to_string()))
        .await?;
      Ok(())
    }

    let result = mock_stream::<DummyStream, _, _>(test).await;
    match result {
      Ok(_) => panic!("operation succeeded unexpectedly"),
      Err(Error::Json(_)) => (),
      Err(e) => panic!("received unexpected error: {}", e),
    }
  }

  #[test(tokio::test)]
  async fn decode_error_errors_do_not_terminate() {
    async fn test(mut stream: WebSocketStream) -> Result<(), WebSocketError> {
      // Authentication.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(AUTH_REQ.to_string()),
      );
      stream.send(Message::Text(AUTH_RESP.to_string())).await?;

      // Subscription.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(STREAM_REQ.to_string()),
      );
      stream.send(Message::Text(STREAM_RESP.to_string())).await?;

      stream
        .send(Message::Text("{ foobarbaz }".to_string()))
        .await?;
      stream.send(Message::Text(UNIT_EVENT.to_string())).await?;
      stream.send(Message::Close(None)).await?;
      Ok(())
    }

    let stream = mock_stream::<DummyStream, _, _>(test).await.unwrap();
    let _ = stream
      .map_err(Error::from)
      .try_for_each(|_| ready(Ok(())))
      .await
      .unwrap();
  }

  #[test(tokio::test)]
  async fn ping_pong() {
    async fn test(mut stream: WebSocketStream) -> Result<(), WebSocketError> {
      // Authentication.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(AUTH_REQ.to_string()),
      );
      stream.send(Message::Text(AUTH_RESP.to_string())).await?;

      // Subscription.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(STREAM_REQ.to_string()),
      );
      stream.send(Message::Text(STREAM_RESP.to_string())).await?;

      // Ping.
      stream.send(Message::Ping(Vec::new())).await?;
      // Expect Pong.
      assert_eq!(stream.next().await.unwrap()?, Message::Pong(Vec::new()),);

      stream.send(Message::Close(None)).await?;
      Ok(())
    }

    let stream = mock_stream::<DummyStream, _, _>(test).await.unwrap();
    let _ = stream
      .map_err(Error::from)
      .try_for_each(|_| ready(Ok(())))
      .await
      .unwrap();
  }
}
