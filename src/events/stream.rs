// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use futures::FutureExt;
use futures::stream::Stream;
use futures::StreamExt;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Error as JsonError;

use tracing::debug;
use tracing::info;
use tracing::info_span;

use tungstenite::tokio::connect_async_with_tls_connector;
use tungstenite::tungstenite::Error as WebSocketError;

use websocket_util::stream as do_stream;

use crate::api_info::ApiInfo;
use crate::Error;
use crate::events::handshake::handshake;
use crate::events::handshake::StreamType;


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


async fn stream_impl<I>(
  api_info: ApiInfo,
  stream_type: StreamType,
) -> Result<impl Stream<Item = Result<Result<I, JsonError>, WebSocketError>>, Error>
where
  I: DeserializeOwned,
{
  let ApiInfo {
    base_url: url,
    key_id,
    secret,
  } = api_info;

  let span = info_span!("stream", events = debug(&stream_type));
  let _guard = span.enter();

  info!(message = "connecting", url = display(&url));

  // We just ignore the response & headers that are sent along after
  // the connection is made. Alpaca does not seem to be using them,
  // really.
  let (mut stream, response) = connect_async_with_tls_connector(url.clone(), None).await?;
  info!("connection successful");
  debug!(response = debug(&response));

  handshake(&mut stream, key_id, secret, stream_type).await?;
  info!("subscription successful");

  let stream = do_stream::<_, stream::Event<I>>(stream)
    .map(|stream| {
      stream.map(|result| {
        result.map(|result| {
          result.map(|event| event.data.0)
        })
      })
    }).await;

  Ok(stream)
}

/// Create a stream for decoded event data.
pub async fn stream<S>(
  api_info: ApiInfo,
) -> Result<impl Stream<Item = Result<Result<S::Event, JsonError>, WebSocketError>>, Error>
where
  S: EventStream,
{
  stream_impl(api_info, S::stream()).await
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::future::Future;

  use futures::future::ready;
  use futures::SinkExt;
  use futures::TryStreamExt;

  use serde_json::from_str as from_json;

  use test_env_log::test;

  use tungstenite::tungstenite::Message;

  use url::Url;

  use websocket_util::test::mock_server;
  use websocket_util::test::WebSocketStream;

  use crate::api::v2::events;
  use crate::api::v2::order;

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

    stream::<S>(api_info).await
  }

  #[test]
  fn parse_trade_event() {
    let response = r#"{
  "stream":"trade_updates",
  "data":{
    "event":"canceled",
    "order":{
      "asset_class":"us_equity",
      "asset_id":"3ece3182-5903-4902-b963-f875a0f416e7",
      "canceled_at":"2020-01-19T06:19:40.137087268Z",
      "client_order_id":"be7d3030-a53e-47ee-9dd3-d9ff3460a174",
      "created_at":"2020-01-19T06:19:34.344561Z",
      "expired_at":null,
      "extended_hours":false,
      "failed_at":null,
      "filled_at":null,
      "filled_avg_price":null,
      "filled_qty":"0",
      "id":"7bb4a536-d59b-4e65-aacf-a8b118d815f4",
      "legs":null,
      "limit_price":"1",
      "order_type":"limit",
      "qty":"1",
      "replaced_at":null,
      "replaced_by":null,
      "replaces":null,
      "side":"buy",
      "status":"canceled",
      "stop_price":null,
      "submitted_at":"2020-01-19T06:19:34.32909Z",
      "symbol":"VMW",
      "time_in_force":"opg",
      "type":"limit",
      "updated_at":"2020-01-19T06:19:40.147946209Z"
    },
    "timestamp":"2020-01-19T06:19:40.137087268Z"
  }
}"#;

    let event = from_json::<stream::Event<events::TradeUpdate>>(&response).unwrap();
    assert_eq!(event.stream, StreamType::TradeUpdates);
    assert_eq!(event.data.0.event, events::TradeStatus::Canceled);
    assert_eq!(event.data.0.order.status, order::Status::Canceled);
    assert_eq!(
      event.data.0.order.time_in_force,
      order::TimeInForce::UntilMarketOpen
    );
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
      Err(Error::WebSocket(WebSocketError::Protocol(ref e)))
        if e == "Connection reset without closing handshake" => (),
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
      Error::WebSocket(WebSocketError::Protocol(ref e))
        if e == "Connection reset without closing handshake" => (),
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
