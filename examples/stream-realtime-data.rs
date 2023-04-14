// Copyright (C) 2022-2023 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(clippy::let_unit_value)]
use serde::Deserialize;
use serde::Serialize;

use chrono::{DateTime, Utc};
use apca::data::v2::stream::drive;
use apca::data::v2::stream::Bar;
use apca::data::v2::stream::Quote;
use apca::data::v2::stream::MarketData;
use apca::data::v2::stream::RealtimeData;
use apca::data::v2::stream::IEX;
use apca::ApiInfo;
use apca::Client;
use apca::Error;

use futures::FutureExt as _;
use futures::StreamExt as _;
use futures::TryStreamExt as _;
use num_decimal::Num;

/// A trade for an equity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Trade
{
  /// The trade's symbol.
  #[serde(rename = "S")]
  pub symbol: String,
  /// The trade's ID.
  #[serde(rename = "i")]
  pub trade_id: u64,
  /// The trade's price.
  #[serde(rename = "p")]
  pub trade_price: Num,
  /// The trade's size.
  #[serde(rename = "s")]
  pub trade_size: u64,
  /// The trade's conditions.
  #[serde(rename = "c")]
  pub conditions: Vec<String>,
  /// The trade's time stamp.
  #[serde(rename = "t")]
  pub timestamp: DateTime<Utc>,
  /// The trade's exchange.
  #[serde(rename = "x")]
  pub exchange: String,
  /// The trade's tape.
  #[serde(rename = "z")]
  pub tape: String,
  /// The trade's update. “canceled”, “corrected”, “incorrect”
  #[serde(rename = "u", default)]
  pub update: Option<String>,
}

#[tokio::main]
async fn main() {
  // Requires the following environment variables to be present:
  // - APCA_API_KEY_ID -> your API key
  // - APCA_API_SECRET_KEY -> your secret key
  //
  // Optionally, the following variable is honored:
  // - APCA_API_BASE_URL -> the API base URL to use (set to
  //   https://api.alpaca.markets for live trading)
  let api_info = ApiInfo::from_env().unwrap();
  let client = Client::new(api_info);

  let (mut stream, mut subscription) = client.subscribe::<RealtimeData::<IEX, Bar, Quote, Trade>>().await.unwrap();

  let mut data = MarketData::default();
  // Subscribe to minute aggregate bars for SPY and XLK...
  data.set_bars(["SPY", "XLK"]);
  // ... and realtime quotes for AAPL...
  data.set_quotes(["AAPL"]);
  // ... and realtime trades for TSLA.
  data.set_trades(["TSLA"]);

  let subscribe = subscription.subscribe(&data).boxed();
  // Actually subscribe with the websocket server.
  let () = drive(subscribe, &mut stream)
    .await
    .unwrap()
    .unwrap()
    .unwrap();

  let () = stream
    // Stop after receiving and printing 50 updates.
    .take(50)
    .map_err(Error::WebSocket)
    .try_for_each(|result| async { result.map(|data| println!("{data:?}")).map_err(Error::Json) })
    .await
    .unwrap();

  // Using the provided `subscription` object we could change the
  // symbols for which to receive bars or quotes at any point.
}
