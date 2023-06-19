// Copyright (C) 2019-2023 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::borrow::Cow;

use async_trait::async_trait;

use futures::stream::Fuse;
use futures::stream::Map;
use futures::stream::SplitSink;
use futures::stream::SplitStream;
use futures::FutureExt as _;
use futures::Sink;
use futures::StreamExt as _;

use serde::Deserialize;
use serde::Serialize;
use serde_json::from_slice as json_from_slice;
use serde_json::from_str as json_from_str;
use serde_json::to_string as to_json;
use serde_json::Error as JsonError;

use tokio::net::TcpStream;

use tungstenite::MaybeTlsStream;
use tungstenite::WebSocketStream;

use websocket_util::subscribe;
use websocket_util::subscribe::MessageStream;
use websocket_util::tungstenite::Error as WebSocketError;
use websocket_util::wrap;
use websocket_util::wrap::Wrapper;

use crate::api::v2::order;
use crate::api_info::ApiInfo;
use crate::subscribable::Subscribable;
use crate::websocket::connect;
use crate::websocket::MessageResult;
use crate::Error;


/// The status of an order, as reported as part of a `OrderUpdate`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OrderStatus {
  /// The order has been received by Alpaca, and routed to exchanges for
  /// execution.
  #[serde(rename = "new")]
  New,
  /// The order has changed.
  #[serde(rename = "replaced")]
  Replaced,
  /// The order replacement has been rejected.
  #[serde(rename = "order_replace_rejected")]
  ReplaceRejected,
  /// The order has been partially filled.
  #[serde(rename = "partial_fill")]
  PartialFill,
  /// The order has been filled, and no further updates will occur for
  /// the order.
  #[serde(rename = "fill")]
  Filled,
  /// The order is done executing for the day, and will not receive
  /// further updates until the next trading day.
  #[serde(rename = "done_for_day")]
  DoneForDay,
  /// The order has been canceled, and no further updates will occur for
  /// the order.
  #[serde(rename = "canceled")]
  Canceled,
  /// The order cancellation has been rejected.
  #[serde(rename = "order_cancel_rejected")]
  CancelRejected,
  /// The order has expired, and no further updates will occur.
  #[serde(rename = "expired")]
  Expired,
  /// The order is waiting to be canceled.
  #[serde(rename = "pending_cancel")]
  PendingCancel,
  /// The order has been stopped, and a trade is guaranteed for the
  /// order, usually at a stated price or better, but has not yet
  /// occurred.
  #[serde(rename = "stopped")]
  Stopped,
  /// The order has been rejected, and no further updates will occur for
  /// the order.
  #[serde(rename = "rejected")]
  Rejected,
  /// The order has been suspended, and is not eligible for trading.
  /// This state only occurs on rare occasions.
  #[serde(rename = "suspended")]
  Suspended,
  /// The order has been received by Alpaca, and routed to the
  /// exchanges, but has not yet been accepted for execution.
  #[serde(rename = "pending_new")]
  PendingNew,
  /// The order is awaiting replacement.
  #[serde(rename = "pending_replace")]
  PendingReplace,
  /// The order has been completed for the day (either filled or done
  /// for day), but remaining settlement calculations are still pending.
  #[serde(rename = "calculated")]
  Calculated,
  /// Any other status that we have not accounted for.
  ///
  /// Note that having any such status should be considered a bug.
  #[serde(other, rename(serialize = "unknown"))]
  Unknown,
}


/// An enumeration of the different event streams.
#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[doc(hidden)]
pub enum StreamType {
  /// A stream for order updates.
  #[serde(rename = "trade_updates")]
  OrderUpdates,
}


/// A type capturing the stream types we may subscribe to.
#[derive(Debug, Deserialize, Serialize)]
#[doc(hidden)]
pub struct Streams<'d> {
  /// A list of stream types.
  pub streams: Cow<'d, [StreamType]>,
}

impl<'d> From<&'d [StreamType]> for Streams<'d> {
  #[inline]
  fn from(src: &'d [StreamType]) -> Self {
    Self {
      streams: Cow::from(src),
    }
  }
}


/// The status reported in authentication control messages.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[doc(hidden)]
#[allow(missing_copy_implementations)]
pub enum AuthenticationStatus {
  /// The client has been authorized.
  #[serde(rename = "authorized")]
  Authorized,
  /// The client has not been authorized.
  #[serde(rename = "unauthorized")]
  Unauthorized,
}


/// The authentication related data provided in authentication control
/// messages.
#[derive(Debug, Deserialize, Serialize)]
#[doc(hidden)]
pub struct Authentication {
  /// The status of an operation.
  #[serde(rename = "status")]
  pub status: AuthenticationStatus,
  /*
   * TODO: Right now we just ignore the `action` field, as we would
   *       not react on it anyway.
   */
}


/// A control message authentication request sent over a websocket
/// channel.
#[derive(Debug, Deserialize, Serialize)]
// Part of unofficial unstable API.
#[doc(hidden)]
#[serde(tag = "action")]
pub enum Authenticate<'d> {
  /// A request to authenticate with the server after a websocket
  /// connection was established.
  #[serde(rename = "auth")]
  Request {
    #[serde(rename = "key")]
    key_id: Cow<'d, str>,
    #[serde(rename = "secret")]
    secret: Cow<'d, str>,
  },
}


/// A control message listen request sent over a websocket channel.
#[derive(Debug, Deserialize, Serialize)]
// Part of unofficial unstable API.
#[doc(hidden)]
#[serde(tag = "action", content = "data")]
pub enum Listen<'d> {
  /// A request to listen to a particular stream.
  #[serde(rename = "listen")]
  Request(Streams<'d>),
}


/// An enumeration of the supported control messages.
#[derive(Debug)]
#[doc(hidden)]
pub enum ControlMessage {
  /// A control message indicating whether or not we were authenticated.
  AuthenticationMessage(Authentication),
  /// A control message indicating which streams we are
  /// subscribed/listening to now.
  ListeningMessage(Streams<'static>),
}


/// An enum representing the different messages we may receive over our
/// websocket channel.
#[derive(Debug, Deserialize, Serialize)]
#[doc(hidden)]
#[serde(tag = "stream", content = "data")]
#[allow(clippy::large_enum_variant)]
pub enum OrderMessage {
  /// An order update.
  #[serde(rename = "trade_updates")]
  OrderUpdate(OrderUpdate),
  /// A control message indicating whether or not we were authenticated
  /// successfully.
  #[serde(rename = "authorization")]
  AuthenticationMessage(Authentication),
  /// A control message detailing the streams we are subscribed to.
  #[serde(rename = "listening")]
  ListeningMessage(Streams<'static>),
}


/// A representation of an order update that we receive through the
/// "trade_updates" stream.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OrderUpdate {
  /// The event that occurred.
  #[serde(rename = "event")]
  pub event: OrderStatus,
  /// The order that received an update.
  #[serde(rename = "order")]
  pub order: order::Order,
}


/// A websocket message that we tried to parse.
type ParsedMessage = MessageResult<Result<OrderMessage, JsonError>, WebSocketError>;

impl subscribe::Message for ParsedMessage {
  type UserMessage = Result<Result<OrderUpdate, JsonError>, WebSocketError>;
  type ControlMessage = ControlMessage;

  fn classify(self) -> subscribe::Classification<Self::UserMessage, Self::ControlMessage> {
    match self {
      MessageResult::Ok(Ok(message)) => match message {
        OrderMessage::OrderUpdate(update) => subscribe::Classification::UserMessage(Ok(Ok(update))),
        OrderMessage::AuthenticationMessage(authentication) => {
          subscribe::Classification::ControlMessage(ControlMessage::AuthenticationMessage(
            authentication,
          ))
        },
        OrderMessage::ListeningMessage(streams) => {
          subscribe::Classification::ControlMessage(ControlMessage::ListeningMessage(streams))
        },
      },
      // JSON errors are directly passed through.
      MessageResult::Ok(Err(err)) => subscribe::Classification::UserMessage(Ok(Err(err))),
      // WebSocket errors are also directly pushed through.
      MessageResult::Err(err) => subscribe::Classification::UserMessage(Err(err)),
    }
  }

  #[inline]
  fn is_error(user_message: &Self::UserMessage) -> bool {
    // Both outer `WebSocketError` and inner `JsonError` errors
    // constitute errors in our sense.
    user_message
      .as_ref()
      .map(|result| result.is_err())
      .unwrap_or(true)
  }
}


/// A subscription allowing certain control operations pertaining order
/// update retrieval.
#[derive(Debug)]
pub struct Subscription<S>(subscribe::Subscription<S, ParsedMessage, wrap::Message>);

impl<S> Subscription<S>
where
  S: Sink<wrap::Message> + Unpin,
{
  /// Authenticate the connection using Alpaca credentials.
  async fn authenticate(
    &mut self,
    key_id: &str,
    secret: &str,
  ) -> Result<Result<(), Error>, S::Error> {
    let request = Authenticate::Request {
      key_id: key_id.into(),
      secret: secret.into(),
    };
    let json = match to_json(&request) {
      Ok(json) => json,
      Err(err) => return Ok(Err(Error::Json(err))),
    };
    let message = wrap::Message::Text(json);
    let response = self.0.send(message).await?;

    match response {
      Some(response) => match response {
        Ok(ControlMessage::AuthenticationMessage(authentication)) => {
          if authentication.status != AuthenticationStatus::Authorized {
            return Ok(Err(Error::Str("authentication not successful".into())))
          }
          Ok(Ok(()))
        },
        Ok(_) => Ok(Err(Error::Str(
          "server responded with an unexpected message".into(),
        ))),
        Err(()) => Ok(Err(Error::Str("failed to authenticate with server".into()))),
      },
      None => Ok(Err(Error::Str(
        "stream was closed before authorization message was received".into(),
      ))),
    }
  }

  /// Subscribe and listen to order updates.
  async fn listen(&mut self) -> Result<Result<(), Error>, S::Error> {
    let streams = Streams::from([StreamType::OrderUpdates].as_ref());
    let request = Listen::Request(streams);
    let json = match to_json(&request) {
      Ok(json) => json,
      Err(err) => return Ok(Err(Error::Json(err))),
    };
    let message = wrap::Message::Text(json);
    let response = self.0.send(message).await?;

    match response {
      Some(response) => match response {
        Ok(ControlMessage::ListeningMessage(streams)) => {
          if !streams.streams.contains(&StreamType::OrderUpdates) {
            return Ok(Err(Error::Str(
              "server did not subscribe us to order update stream".into(),
            )))
          }
          Ok(Ok(()))
        },
        Ok(_) => Ok(Err(Error::Str(
          "server responded with an unexpected message".into(),
        ))),
        Err(()) => Ok(Err(Error::Str(
          "failed to listen to order update stream".into(),
        ))),
      },
      None => Ok(Err(Error::Str(
        "stream was closed before listen message was received".into(),
      ))),
    }
  }
}


type Stream = Map<Wrapper<WebSocketStream<MaybeTlsStream<TcpStream>>>, MapFn>;
type MapFn = fn(Result<wrap::Message, WebSocketError>) -> ParsedMessage;


/// A type used for requesting a subscription to the "trade_updates"
/// event stream.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderUpdates {}

#[async_trait]
impl Subscribable for OrderUpdates {
  type Input = ApiInfo;
  type Subscription = Subscription<SplitSink<Stream, wrap::Message>>;
  type Stream = Fuse<MessageStream<SplitStream<Stream>, ParsedMessage>>;

  async fn connect(api_info: &Self::Input) -> Result<(Self::Stream, Self::Subscription), Error> {
    fn map(result: Result<wrap::Message, WebSocketError>) -> ParsedMessage {
      MessageResult::from(result.map(|message| match message {
        wrap::Message::Text(string) => json_from_str::<OrderMessage>(&string),
        wrap::Message::Binary(data) => json_from_slice::<OrderMessage>(&data),
      }))
    }

    let ApiInfo {
      api_stream_url: url,
      key_id,
      secret,
      ..
    } = api_info;

    let stream = connect(url).await?.map(map as MapFn);
    let (send, recv) = stream.split();
    let (stream, subscription) = subscribe::subscribe(recv, send);
    let mut stream = stream.fuse();

    let mut subscription = Subscription(subscription);
    let authenticate = subscription.authenticate(key_id, secret).boxed();
    let () = subscribe::drive::<ParsedMessage, _, _>(authenticate, &mut stream)
      .await
      .map_err(|result| {
        result
          .map(|result| Error::Json(result.unwrap_err()))
          .map_err(Error::WebSocket)
          .unwrap_or_else(|err| err)
      })???;

    let listen = subscription.listen().boxed();
    let () = subscribe::drive::<ParsedMessage, _, _>(listen, &mut stream)
      .await
      .map_err(|result| {
        result
          .map(|result| Error::Json(result.unwrap_err()))
          .map_err(Error::WebSocket)
          .unwrap_or_else(|err| err)
      })???;

    Ok((stream, subscription))
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use futures::channel::oneshot::channel;
  use futures::future::ok;
  use futures::future::ready;
  use futures::SinkExt;
  use futures::TryStreamExt;

  use serde_json::from_str as json_from_str;

  use test_log::test;

  use websocket_util::test::WebSocketStream;
  use websocket_util::tungstenite::error::ProtocolError;
  use websocket_util::tungstenite::Message;

  use crate::api::v2::order;
  use crate::api::v2::order_util::order_aapl;
  use crate::api::API_BASE_URL;
  use crate::websocket::test::mock_stream;
  use crate::Client;
  use crate::Error;


  // TODO: Until we can interpolate more complex expressions using
  //       `std::format` in a const context we have to hard code the
  //       values of `crate::websocket::test::KEY_ID` and
  //       `crate::websocket::test::SECRET` here.
  const AUTH_REQ: &str = r#"{"action":"auth","key":"USER12345678","secret":"justletmein"}"#;
  const AUTH_RESP: &str =
    r#"{"stream":"authorization","data":{"action":"authenticate","status":"authorized"}}"#;
  const STREAM_REQ: &str = r#"{"action":"listen","data":{"streams":["trade_updates"]}}"#;
  const STREAM_RESP: &str = r#"{"stream":"listening","data":{"streams":["trade_updates"]}}"#;


  /// Check that we can encode an authentication request correctly.
  #[test]
  fn encode_authentication_request() {
    let key_id = "some-key".into();
    let secret = "super-secret-secret".into();
    let expected = r#"{"action":"auth","key":"some-key","secret":"super-secret-secret"}"#;

    let request = Authenticate::Request { key_id, secret };
    let json = to_json(&request).unwrap();
    assert_eq!(json, expected)
  }

  /// Check that we can encode a listen request properly.
  #[test]
  fn encode_listen_request() {
    let expected = r#"{"action":"listen","data":{"streams":["trade_updates"]}}"#;
    let streams = Streams::from([StreamType::OrderUpdates].as_ref());
    let request = Listen::Request(streams);
    let json = to_json(&request).unwrap();
    assert_eq!(json, expected)
  }

  /// Verify that we can decode an order update.
  #[test]
  fn decode_order_update() {
    let json = r#"{
  "stream":"trade_updates","data":{
    "event":"new","execution_id":"11111111-2222-3333-4444-555555555555","order":{
      "asset_class":"us_equity","asset_id":"11111111-2222-3333-4444-555555555555",
      "canceled_at":null,"client_order_id":"11111111-2222-3333-4444-555555555555",
      "created_at":"2021-12-09T19:48:46.176628398Z","expired_at":null,
      "extended_hours":false,"failed_at":null,"filled_at":null,
      "filled_avg_price":null,"filled_qty":"0","hwm":null,
      "id":"11111111-2222-3333-4444-555555555555","legs":null,"limit_price":"1",
      "notional":null,"order_class":"simple","order_type":"limit","qty":"1",
      "replaced_at":null,"replaced_by":null,"replaces":null,"side":"buy",
      "status":"new","stop_price":null,"submitted_at":"2021-12-09T19:48:46.175261379Z",
      "symbol":"AAPL","time_in_force":"day","trail_percent":null,"trail_price":null,
      "type":"limit","updated_at":"2021-12-09T19:48:46.185346448Z"
    },"timestamp":"2021-12-09T19:48:46.182987144Z"
  }
}"#;
    let message = json_from_str::<OrderMessage>(json).unwrap();
    match message {
      OrderMessage::OrderUpdate(update) => {
        assert_eq!(update.event, OrderStatus::New);
        assert_eq!(update.order.side, order::Side::Buy);
      },
      _ => panic!("Decoded unexpected message variant: {message:?}"),
    }
  }

  /// Verify that we can decode a authentication control message.
  #[test]
  fn decode_authentication() {
    let json =
      { r#"{"stream":"authorization","data":{"status":"authorized","action":"authenticate"}}"# };
    let message = json_from_str::<OrderMessage>(json).unwrap();
    match message {
      OrderMessage::AuthenticationMessage(authentication) => {
        assert_eq!(authentication.status, AuthenticationStatus::Authorized);
      },
      _ => panic!("Decoded unexpected message variant: {message:?}"),
    }
  }

  /// Check that we can decode an authentication control message
  /// indicating an unsuccessful authorization.
  #[test]
  fn decode_unauthorized_authentication() {
    let json =
      { r#"{"stream":"authorization","data":{"status":"unauthorized","action":"listen"}}"# };
    let message = json_from_str::<OrderMessage>(json).unwrap();
    match message {
      OrderMessage::AuthenticationMessage(authentication) => {
        assert_eq!(authentication.status, AuthenticationStatus::Unauthorized);
      },
      _ => panic!("Decoded unexpected message variant: {message:?}"),
    }
  }

  /// Verify that we can decode a listening control message.
  #[test]
  fn decode_listening() {
    let json = r#"{"stream":"listening","data":{"streams":["trade_updates"]}}"#;

    let message = json_from_str::<OrderMessage>(json).unwrap();
    match message {
      OrderMessage::ListeningMessage(streams) => {
        assert_eq!(streams.streams, vec![StreamType::OrderUpdates]);
      },
      _ => panic!("Decoded unexpected message variant: {message:?}"),
    }
  }


  /// Check that we report the expected error when the server closes the
  /// connection unexpectedly.
  #[test(tokio::test)]
  async fn broken_stream() {
    async fn test(mut stream: WebSocketStream) -> Result<(), WebSocketError> {
      let msg = stream.next().await.unwrap()?;
      assert_eq!(msg, Message::Text(AUTH_REQ.to_string()));
      Ok(())
    }

    let result = mock_stream::<OrderUpdates, _, _>(test).await;
    match result {
      Ok(..) => panic!("authentication succeeded unexpectedly"),
      Err(Error::WebSocket(WebSocketError::Protocol(e)))
        if e == ProtocolError::ResetWithoutClosingHandshake => {},
      Err(e) => panic!("received unexpected error: {e}"),
    }
  }

  /// Test that we handle an early connection close during subscription
  /// correctly.
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

    let result = mock_stream::<OrderUpdates, _, _>(test).await;
    match result {
      Ok(..) => panic!("operation succeeded unexpectedly"),
      Err(Error::Str(ref e)) if e.starts_with("stream was closed before listen") => (),
      Err(e) => panic!("received unexpected error: {e}"),
    }
  }

  /// Check that we can correctly handle a successful subscription
  /// without order update messages.
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

    let err = mock_stream::<OrderUpdates, _, _>(test).await.unwrap_err();
    match err {
      Error::WebSocket(WebSocketError::Protocol(e))
        if e == ProtocolError::ResetWithoutClosingHandshake => {},
      e => panic!("received unexpected error: {e}"),
    }
  }

  /// Check a JSON decoding error during subscription is reported
  /// correctly.
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

    let result = mock_stream::<OrderUpdates, _, _>(test).await.unwrap_err();
    match result {
      Error::Json(_) => (),
      e => panic!("received unexpected error: {e}"),
    }
  }

  /// Check that JSON errors do not terminate the established stream.
  #[test(tokio::test)]
  async fn decode_error_errors_do_not_terminate() {
    let (sender, receiver) = channel();

    let test = |mut stream: WebSocketStream| {
      async move {
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

        // Wait until the connection was established before sending any
        // additional messages.
        let () = receiver.await.unwrap();

        stream
          .send(Message::Text("{ foobarbaz }".to_string()))
          .await?;
        stream.send(Message::Close(None)).await?;
        Ok(())
      }
    };

    let (stream, _subscription) = mock_stream::<OrderUpdates, _, _>(test).await.unwrap();
    let () = sender.send(()).unwrap();

    stream
      .map_err(Error::from)
      .try_for_each(|_| ready(Ok(())))
      .await
      .unwrap();
  }

  /// Verify that ping websocket messages are responded to with pongs.
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

    let (stream, _subscription) = mock_stream::<OrderUpdates, _, _>(test).await.unwrap();
    stream
      .map_err(Error::from)
      .try_for_each(|_| ready(Ok(())))
      .await
      .unwrap();
  }

  /// Test the end-to-end workflow of streaming an order update for a
  /// newly created order.
  #[test(tokio::test)]
  async fn stream_order_events() {
    // TODO: There may be something amiss here. If we don't cancel the
    //       order we never get an event about a new order. That does
    //       not seem to be in our code, though, as the behavior is the
    //       same when streaming events using Alpaca's Python client.
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let (stream, _subscription) = client.subscribe::<OrderUpdates>().await.unwrap();

    let order = order_aapl(&client).await.unwrap();
    client.issue::<order::Delete>(&order.id).await.unwrap();

    let update = stream
      .try_filter_map(|result| {
        let update = result.unwrap();
        ok(Some(update))
      })
      // There could be other orders happening concurrently but we
      // are only interested in ones belonging to the order we
      // submitted as part of this test.
      .try_skip_while(|update| ok(update.order.id != order.id))
      .next()
      .await
      .unwrap()
      .unwrap();

    assert_eq!(order.id, update.order.id);
    assert_eq!(order.asset_id, update.order.asset_id);
    assert_eq!(order.symbol, update.order.symbol);
    assert_eq!(order.asset_class, update.order.asset_class);
    assert_eq!(order.type_, update.order.type_);
    assert_eq!(order.side, update.order.side);
    assert_eq!(order.time_in_force, update.order.time_in_force);
  }

  /// Test that we fail as expected when attempting to authenticate for
  /// order updates using invalid credentials.
  #[test(tokio::test)]
  async fn stream_with_invalid_credentials() {
    let api_info = ApiInfo::from_parts(API_BASE_URL, "invalid", "invalid-too").unwrap();

    let client = Client::new(api_info);
    let err = client.subscribe::<OrderUpdates>().await.unwrap_err();

    match err {
      Error::Str(ref e) if e == "authentication not successful" => (),
      e => panic!("received unexpected error: {e}"),
    }
  }
}
