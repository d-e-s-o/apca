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
