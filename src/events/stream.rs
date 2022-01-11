// Copyright (C) 2019-2021 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::de::DeserializeOwned;
use serde::Deserialize;

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

use websocket_util::wrap::Wrapper;

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
