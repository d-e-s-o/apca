// Copyright (C) 2019-2021 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use futures::stream::Stream;
use futures::StreamExt;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::from_slice as json_from_slice;
use serde_json::from_str as json_from_str;
use serde_json::Error as JsonError;

use url::Url;

use tokio::net::TcpStream;

use tracing::debug;
use tracing::span;
use tracing::trace;
use tracing::Level;
use tracing_futures::Instrument;

use tungstenite::connect_async;
use tungstenite::MaybeTlsStream;
use tungstenite::WebSocketStream;

use websocket_util::tungstenite::Error as WebSocketError;
use websocket_util::tungstenite::Message as WebSocketMessage;
use websocket_util::wrap::Wrapper;

use crate::api_info::ApiInfo;
use crate::events::handshake::handshake;
use crate::events::handshake::StreamType;
use crate::Error;


/// A trait representing a particular event stream.
pub trait EventStream {
  /// The events being reported through the stream.
  type Event: DeserializeOwned;

  /// The actual type of stream.
  fn stream() -> StreamType;
}


/// A type representing the outer most event encapsulating type.
#[derive(Clone, Debug, Deserialize)]
pub struct Event<T> {
  /// The stream type reported by the server.
  #[serde(rename = "stream")]
  pub stream: StreamType,
  /// The inner data.
  #[serde(rename = "data")]
  pub data: T,
}


/// Internal function to connect to websocket server.
async fn connect_internal(
  mut url: Url,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, Error> {
  // TODO: We really shouldn't need this conditional logic. Find a
  //       better way.
  match url.scheme() {
    "ws" | "wss" => (),
    _ => {
      url.set_scheme("wss").map_err(|()| {
        Error::Str(format!("unable to change URL scheme for {}: invalid URL?", url).into())
      })?;
    },
  }
  url.set_path("stream");

  let span = span!(Level::DEBUG, "stream");

  async move {
    debug!(message = "connecting", url = display(&url));

    // We just ignore the response & headers that are sent along after
    // the connection is made. Alpaca does not seem to be using them,
    // really.
    let (stream, response) = connect_async(url).await?;
    debug!("connection successful");
    trace!(response = debug(&response));

    Ok(stream)
  }
  .instrument(span)
  .await
}


/// Connect to websocket server.
pub async fn connect(
  url: Url,
) -> Result<Wrapper<WebSocketStream<MaybeTlsStream<TcpStream>>>, Error> {
  connect_internal(url)
    .await
    .map(|stream| Wrapper::builder().build(stream))
}

/// Create a stream for decoded event data.
pub async fn stream<S>(
  api_info: &ApiInfo,
) -> Result<impl Stream<Item = Result<Result<S::Event, JsonError>, WebSocketError>>, Error>
where
  S: EventStream,
{
  let ApiInfo {
    base_url: url,
    key_id,
    secret,
  } = api_info;

  let mut stream = connect_internal(url.clone()).await?;

  handshake(&mut stream, key_id, secret, S::stream()).await?;
  debug!("subscription successful");

  let stream = stream.filter_map(|result| async {
    match result {
      Ok(message) => match message {
        WebSocketMessage::Text(string) => {
          let result = json_from_str::<Event<S::Event>>(&string);
          Some(Ok(result.map(|event| event.data)))
        },
        WebSocketMessage::Binary(data) => {
          let result = json_from_slice::<Event<S::Event>>(&data);
          Some(Ok(result.map(|event| event.data)))
        },
        WebSocketMessage::Ping(_) | WebSocketMessage::Pong(_) | WebSocketMessage::Close(_) => None,
      },
      Err(err) => Some(Err(err)),
    }
  });

  Ok(stream)
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::future::Future;

  use futures::future::ready;
  use futures::SinkExt;
  use futures::TryStreamExt;

  use test_log::test;

  use url::Url;

  use websocket_util::test::mock_server;
  use websocket_util::test::WebSocketStream;
  use websocket_util::tungstenite::error::ProtocolError;
  use websocket_util::tungstenite::Message;

  const KEY_ID: &str = "USER12345678";
  const SECRET: &str = "justletmein";
  const AUTH_REQ: &str =
    r#"{"action":"authenticate","data":{"key_id":"USER12345678","secret_key":"justletmein"}}"#;
  const AUTH_RESP: &str =
    r#"{"stream":"authorization","data":{"action":"authenticate","status":"authorized"}}"#;
  const STREAM_REQ: &str = r#"{"action":"listen","data":{"streams":["trade_updates"]}}"#;
  const STREAM_RESP: &str = r#"{"stream":"listening","data":{"streams":["trade_updates"]}}"#;
  const UNIT_EVENT: &str = r#"{"stream":"trade_updates","data":null}"#;


  /// A stream used solely for testing purposes.
  enum DummyStream {}

  impl EventStream for DummyStream {
    type Event = ();

    fn stream() -> StreamType {
      StreamType::TradeUpdates
    }
  }

  async fn mock_stream<S, F, R>(
    f: F,
  ) -> Result<impl Stream<Item = Result<Result<S::Event, JsonError>, WebSocketError>>, Error>
  where
    S: EventStream,
    F: FnOnce(WebSocketStream) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), WebSocketError>> + Send + Sync + 'static,
  {
    let addr = mock_server(f).await;
    let api_info = ApiInfo {
      base_url: Url::parse(&format!("ws://{}", addr.to_string())).unwrap(),
      key_id: KEY_ID.to_string(),
      secret: SECRET.to_string(),
    };

    stream::<S>(&api_info).await
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
      Err(Error::WebSocket(WebSocketError::Protocol(e)))
        if e == ProtocolError::ResetWithoutClosingHandshake => {},
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
      Error::WebSocket(WebSocketError::Protocol(e))
        if e == ProtocolError::ResetWithoutClosingHandshake => {},
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
    stream
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
    stream
      .map_err(Error::from)
      .try_for_each(|_| ready(Ok(())))
      .await
      .unwrap();
  }
}
