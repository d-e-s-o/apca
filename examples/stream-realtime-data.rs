// Copyright (C) 2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use apca::data::v2::stream::drive;
use apca::data::v2::stream::MarketData;
use apca::data::v2::stream::RealtimeData;
use apca::data::v2::stream::IEX;
use apca::ApiInfo;
use apca::Client;
use apca::Error;

use futures::FutureExt as _;
use futures::StreamExt as _;
use futures::TryStreamExt as _;

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

  let (mut stream, mut subscription) = client.subscribe::<RealtimeData<IEX>>().await.unwrap();

  let mut data = MarketData::default();
  // Subscribe to minute aggregate bars for SPY and XLK...
  data.set_bars(["SPY", "XLK"]);
  // ... and realtime quotes for AAPL.
  data.set_quotes(["AAPL"]);
  // ... and realtime trades for MSFT.
  data.set_trades(["MSFT"]);

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
    .try_for_each(|result| async {
      result
        .map(|data| println!("{:?}", data))
        .map_err(Error::Json)
    })
    .await
    .unwrap();

  // Using the provided `subscription` object we could change the
  // symbols for which to receive bars or quotes at any point.
}
