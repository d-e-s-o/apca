// Copyright (C) 2023 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use num_decimal::Num;


pub struct History {
  /// Basis in dollar of the profit loss calculation.
  pub base_value: Num,

  // - name: timestamp
  //   type: array of epoch int (in seconds)
  //   desc: time of each data element, left-labeled (the beginning of time window)
  // - name: equity
  //   type: array of number
  //   desc: equity value of the account in dollar amount as of the end of each time window
  // - name: profit_loss
  //   type: array of number
  //   desc: profit/loss in dollar from the base value
  // - name: profit_loss_pct
  //   type: array of number
  //   desc: profit/loss in percentage from the base value
  // - name: base_value
  //   type: number
  //   desc: basis in dollar of the profit loss calculation
  // - name: timeframe
  //   desc: time window size of each data element
}


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_slice as from_json;


  /// Make sure that we can parse the reference portfolio history
  /// object.
  #[test]
  fn parse_reference_portfolio_history() {
    let serialized = r#"{
  "timestamp": [1580826600000, 1580827500000, 1580828400000],
  "equity": [27423.73, 27408.19, 27515.97],
  "profit_loss": [11.8, -3.74, 104.04],
  "profit_loss_pct": [0.000430469507254688, -0.0001364369455197062, 0.0037954277571845543],
  "base_value": 27411.93,
  "timeframe": "15Min"
}"#;

    let history = from_json::<History>(serialized).unwrap();
    todo!()
    //assert_eq!(amount, Amount::notional(Num::from_str("15.12").unwrap()));
  }
}
