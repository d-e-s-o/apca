// Copyright (C) 2020-2021 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use apca::api::v2::order;
use apca::ApiInfo;
use apca::Client;

use num_decimal::Num;

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

  // Create request for a limit order for AAPL with a limit price of USD
  // 100.
  let request = order::OrderReqInit {
    type_: order::Type::Limit,
    limit_price: Some(Num::from(100)),
    ..Default::default()
  }
  // We want to go long on AAPL, buying a single share.
  .init("AAPL", order::Side::Buy, order::Amount::quantity(1));

  let order = client.issue::<order::Post>(&request).await.unwrap();
  println!("Created order {}", order.id.to_hyphenated_ref());
}
