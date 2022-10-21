// Copyright (C) 2020-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use apca::data::v2::{last_quote, last_trade};
use apca::ApiInfo;
use apca::Client;

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

  let quote_req = last_quote::LastQuoteReq::new(vec!["AAPL".to_string(), "MSFT".to_string()]);
  let quotes = client.issue::<last_quote::Get>(&quote_req).await.unwrap();
  quotes.iter().for_each(|q| {
    println!(
      "Latest quote for {}: Ask {}/{} Bid {}/{}",
      q.symbol, q.ask_price, q.ask_size, q.bid_price, q.bid_size
    )
  });

  let trade_req = last_trade::LastTradeRequest::new(vec![
    "SPY".to_string(),
    "QQQ".to_string(),
    "IWM".to_string(),
  ]);
  let trades = client.issue::<last_trade::Get>(&trade_req).await.unwrap();
  trades.iter().for_each(|trade| {
    println!(
      "Latest trade for {}: {} @ {}",
      trade.symbol, trade.size, trade.price
    );
  });
}
