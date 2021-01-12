// Copyright (C) 2020-2021 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::SystemTime;

use num_decimal::Num;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde_urlencoded::to_string as to_query;
use serde_variant::to_variant_name;

use time_util::system_time_from_date_str;
use time_util::system_time_from_str;

use crate::api::v2::de::ContentDeserializer;
use crate::api::v2::de::TaggedContentVisitor;
use crate::api::v2::order;
use crate::api::v2::util::u64_from_str;
use crate::Str;


/// An enum representing the various non-trade activities.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum ActivityType {
  /// Order fills (both partial and full fills).
  ///
  /// This variant will only ever be set for trade activities.
  #[serde(rename = "FILL")]
  Fill,
  /// Cash transactions (both CSD and CSR).
  #[serde(rename = "TRANS")]
  Transaction,
  ///  Miscellaneous or rarely used activity types (All types except those in TRANS, DIV, or FILL).
  #[serde(rename = "MISC")]
  Miscellaneous,
  /// ACATS IN/OUT (Cash).
  #[serde(rename = "ACATC")]
  AcatsInOutCash,
  /// ACATS IN/OUT (Securities).
  #[serde(rename = "ACATS")]
  AcatsInOutSecurities,
  /// Cash disbursement(+).
  #[serde(rename = "CSD")]
  CashDisbursement,
  /// Cash receipt(-).
  #[serde(rename = "CSR")]
  CashReceipt,
  /// Dividends.
  #[serde(rename = "DIV")]
  Dividend,
  /// Dividend (capital gain long term).
  #[serde(rename = "DIVCGL")]
  CapitalGainLongTerm,
  /// Dividend (capital gain short term).
  #[serde(rename = "DIVCGS")]
  CapitalGainShortTerm,
  /// Dividend fee.
  #[serde(rename = "DIVFEE")]
  DividendFee,
  /// Dividend adjusted (Foreign Tax Withheld).
  #[serde(rename = "DIVFT")]
  DividendAdjusted,
  /// Dividend adjusted (NRA Withheld).
  #[serde(rename = "DIVNRA")]
  DividendAdjustedNraWithheld,
  /// Dividend return of capital.
  #[serde(rename = "DIVROC")]
  DividendReturnOfCapital,
  /// Dividend adjusted (Tefra Withheld).
  #[serde(rename = "DIVTW")]
  DividendAdjustedTefraWithheld,
  /// Dividend (tax exempt).
  #[serde(rename = "DIVTXEX")]
  DividendTaxExtempt,
  /// Interest (credit/margin).
  #[serde(rename = "INT")]
  Interest,
  /// Interest adjusted (NRA Withheld).
  #[serde(rename = "INTNRA")]
  InterestAdjustedNraWithheld,
  /// Interest adjusted (Tefra Withheld).
  #[serde(rename = "INTTW")]
  InterestAdjustedTefraWithheld,
  /// Journal entry.
  #[serde(rename = "JNL")]
  JournalEntry,
  /// Journal entry (cash).
  #[serde(rename = "JNLC")]
  JournalEntryCash,
  /// Journal entry (stock).
  #[serde(rename = "JNLS")]
  JournalEntryStock,
  /// Merger/Acquisition.
  #[serde(rename = "MA")]
  Acquisition,
  /// Name change.
  #[serde(rename = "NC")]
  NameChange,
  /// Option assignment.
  #[serde(rename = "OPASN")]
  OptionAssignment,
  /// Option expiration.
  #[serde(rename = "OPEXP")]
  OptionExpiration,
  /// Option exercise.
  #[serde(rename = "OPXRC")]
  OptionExercise,
  /// Pass Thru Charge.
  #[serde(rename = "PTC")]
  PassThruCharge,
  /// Pass Thru Rebate.
  #[serde(rename = "PTR")]
  PassThruRebate,
  /// Reorg CA.
  #[serde(rename = "REORG")]
  Reorg,
  /// Symbol change.
  #[serde(rename = "SC")]
  SymbolChange,
  /// Stock spinoff.
  #[serde(rename = "SSO")]
  StockSpinoff,
  /// Stock split.
  #[serde(rename = "SSP")]
  StockSplit,
}


/// An enumeration describing the side of a trade activity.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum Side {
  /// A buy of an asset.
  #[serde(rename = "buy")]
  Buy,
  /// A sale of an asset.
  #[serde(rename = "sell")]
  Sell,
  /// A short sale of an asset.
  #[serde(rename = "sell_short")]
  ShortSell,
}


/// A trade related activity.
// TODO: Not all fields are hooked up.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct TradeActivity {
  /// The time at which the execution occurred.
  #[serde(rename = "transaction_time", deserialize_with = "system_time_from_str")]
  pub transaction_time: SystemTime,
  /// The traded symbol.
  #[serde(rename = "symbol")]
  pub symbol: String,
  /// The ID of the order this trade activity belongs to.
  #[serde(rename = "order_id")]
  pub order_id: order::Id,
  /// The side of a trade.
  #[serde(rename = "side")]
  pub side: Side,
  /// The number of shares involved in the trade execution.
  #[serde(rename = "qty", deserialize_with = "u64_from_str")]
  pub quantity: u64,
  /// The cumulative quantity of shares involved in the execution.
  #[serde(rename = "cum_qty", deserialize_with = "u64_from_str")]
  pub cumulative_quantity: u64,
  /// For partially filled orders, the quantity of shares that are left
  /// to be filled.
  #[serde(rename = "leaves_qty", deserialize_with = "u64_from_str")]
  pub unfilled_quantity: u64,
  /// The per-share price that the trade was executed at.
  #[serde(rename = "price")]
  pub price: Num,
}


/// A non-trade related activity.
///
/// This struct is merely an implementation detail aiding in having
/// proper deserialization support for the `Activity` type. It is not
/// meant to be used directly by users. They should use
/// `NonTradeActivity` instead.
// TODO: Not all fields are hooked up.
#[doc(hidden)]
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct NonTradeActivityImpl<T> {
  /// The type of non-trade activity.
  ///
  /// Note that the `Fill` variant will never be used here.
  #[serde(rename = "activity_type")]
  pub type_: T,
  /// The date on which the activity occurred or on which the
  /// transaction associated with the activity settled.
  #[serde(rename = "date", deserialize_with = "system_time_from_date_str")]
  pub date: SystemTime,
  /// The net amount of money (positive or negative) associated with the
  /// activity.
  #[serde(rename = "net_amount")]
  pub net_amount: Num,
  /// The symbol of the security involved with the activity. Not present
  /// for all activity types.
  #[serde(rename = "symbol")]
  pub symbol: Option<String>,
  /// For dividend activities, the number of shares that contributed to
  /// the payment. Not present for other activity types.
  #[serde(rename = "qty")]
  pub quantity: Option<Num>,
  /// For dividend activities, the average amount paid per share. Not
  /// present for other activity types.
  #[serde(rename = "per_share_amount")]
  pub per_share_amount: Option<Num>,
  /// A description of the activity.
  #[serde(rename = "description")]
  pub description: Option<String>,
}

impl<T> NonTradeActivityImpl<T> {
  fn into_other<U>(self, activity_type: U) -> NonTradeActivityImpl<U> {
    let Self {
      date,
      net_amount,
      symbol,
      quantity,
      per_share_amount,
      description,
      ..
    } = self;

    NonTradeActivityImpl::<U> {
      type_: activity_type,
      date,
      net_amount,
      symbol,
      quantity,
      per_share_amount,
      description,
    }
  }
}


/// A non-trade related activity.
///
/// Examples include dividend payments or cash transfers.
pub type NonTradeActivity = NonTradeActivityImpl<ActivityType>;


/// An activity.
#[derive(Clone, Debug, PartialEq)]
pub enum Activity {
  /// A trade activity.
  Trade(TradeActivity),
  /// A non-trade activity (e.g., a dividend payment).
  NonTrade(NonTradeActivity),
}

impl Activity {
  /// Convert this activity into a trade activity, if it is of the
  /// corresponding variant.
  pub fn into_trade(self) -> Result<TradeActivity, Self> {
    match self {
      Activity::Trade(trade) => Ok(trade),
      Activity::NonTrade(..) => Err(self),
    }
  }

  /// Convert this activity into a non-trade activity, if it is of the
  /// corresponding variant.
  pub fn into_non_trade(self) -> Result<NonTradeActivity, Self> {
    match self {
      Activity::Trade(..) => Err(self),
      Activity::NonTrade(non_trade) => Ok(non_trade),
    }
  }
}

impl<'de> Deserialize<'de> for Activity {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let tagged = match Deserializer::deserialize_any(
      deserializer,
      TaggedContentVisitor::<ActivityType>::new("activity_type"),
    ) {
      Ok(val) => val,
      Err(err) => return Err(err),
    };

    let content = ContentDeserializer::new(tagged.content);
    match tagged.tag {
      ActivityType::Fill => TradeActivity::deserialize(content).map(Activity::Trade),
      activity_type => NonTradeActivityImpl::<Option<()>>::deserialize(content)
        .map(|non_trade| non_trade.into_other::<ActivityType>(activity_type))
        .map(Activity::NonTrade),
    }
  }
}


/// Serialize an optional `Vec` into a string of textual representations
/// of the elements separated by comma.
fn optional_vec_to_str<S, T>(vec: &Option<Vec<T>>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
  T: Serialize,
{
  match vec {
    Some(vec) => {
      // `serde_urlencoded` seemingly does not know how to handle a
      // `Vec`. So what we do is we convert each and every element to a
      // string and then concatenate them, separating each by comma.
      let s = vec
        .iter()
        // We know that we are dealing with an enum variant and the
        // function will never return an error for those, so it's fine
        // to unwrap.
        .map(|type_| to_variant_name(type_).unwrap())
        .collect::<Vec<_>>()
        .join(",");
      serializer.serialize_str(&s)
    },
    None => serializer.serialize_none(),
  }
}

/// A GET request to be made to the /v2/account/activities endpoint.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ActivityReq {
  /// The types of activities to retrieve.
  ///
  /// If `None` all activities will be retrieved.
  #[serde(rename = "activity_types", serialize_with = "optional_vec_to_str")]
  pub types: Option<Vec<ActivityType>>,
}


Endpoint! {
  /// The representation of a GET request to the /v2/account/activities
  /// endpoint.
  pub Get(ActivityReq),
  Ok => Vec<Activity>, [
    /// The activity was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  fn path(_input: &Self::Input) -> Str {
    "/v2/account/activities".into()
  }

  fn query(input: &Self::Input) -> Option<Str> {
    // TODO: Realistically there should be no way for this unwrap to
    //       ever panic because our conversion to strings should not be
    //       fallible. But still, ideally we would not have to unwrap.
    Some(to_query(input).unwrap().into())
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use serde_json::from_str as from_json;

  use time_util::parse_system_time_from_date_str;

  use test_env_log::test;

  use uuid::Uuid;

  use crate::api_info::ApiInfo;
  use crate::Client;


  #[test]
  fn parse_reference_trade_activity() {
    let response = r#"{
  "activity_type": "FILL",
  "cum_qty": "1",
  "id": "20190524113406977::8efc7b9a-8b2b-4000-9955-d36e7db0df74",
  "leaves_qty": "0",
  "price": "1.63",
  "qty": "1",
  "side": "buy",
  "symbol": "LPCN",
  "transaction_time": "2019-05-24T15:34:06.977Z",
  "order_id": "904837e3-3b76-47ec-b432-046db621571b",
  "type": "fill"
}"#;

    let trade = from_json::<Activity>(&response)
      .unwrap()
      .into_trade()
      .unwrap();

    let id = order::Id(Uuid::parse_str("904837e3-3b76-47ec-b432-046db621571b").unwrap());
    assert_eq!(trade.symbol, "LPCN");
    assert_eq!(trade.order_id, id);
    assert_eq!(trade.side, Side::Buy);
    assert_eq!(trade.quantity, 1);
    assert_eq!(trade.cumulative_quantity, 1);
    assert_eq!(trade.unfilled_quantity, 0);
    assert_eq!(trade.price, Num::new(163, 100));
  }

  #[test]
  fn parse_reference_non_trade_activity() {
    let response = r#"{
  "activity_type": "DIV",
  "id": "20190801011955195::5f596936-6f23-4cef-bdf1-3806aae57dbf",
  "date": "2019-08-01",
  "net_amount": "1.02",
  "symbol": "T",
  "per_share_amount": "0.51"
}"#;

    let non_trade = from_json::<Activity>(&response)
      .unwrap()
      .into_non_trade()
      .unwrap();

    assert_eq!(non_trade.type_, ActivityType::Dividend);
    assert_eq!(
      non_trade.date,
      parse_system_time_from_date_str("2019-08-01").unwrap()
    );
    assert_eq!(non_trade.symbol, Some("T".into()));
    assert_eq!(non_trade.per_share_amount, Some(Num::new(51, 100)));
  }


  #[test]
  fn parse_dividend() {
    let response = r#"{
      "id":"20200626000000000::e3163618-f82b-4568-af54-b30404484224",
      "activity_type":"DIV",
      "date":"2020-01-01",
      "net_amount":"21.97",
      "description":"DIV",
      "symbol":"SPY",
      "qty":"201.9617035750071243",
      "per_share_amount":"0.108783"
}"#;
    let non_trade = from_json::<Activity>(&response)
      .unwrap()
      .into_non_trade()
      .unwrap();
    assert_eq!(non_trade.type_, ActivityType::Dividend);
    assert_eq!(
      non_trade.date,
      parse_system_time_from_date_str("2020-01-01").unwrap()
    );
    assert_eq!(non_trade.symbol, Some("SPY".into()));
    assert_eq!(
      non_trade.quantity,
      Some(Num::new(2019617035750071243u64, 10000000000000000u64))
    );
    assert_eq!(non_trade.per_share_amount, Some(Num::new(108783, 1000000)));
  }

  #[test(tokio::test)]
  async fn retrieve_some_activities() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let request = ActivityReq {
      types: Some(vec![
        ActivityType::Fill,
        ActivityType::Transaction,
        ActivityType::Dividend,
      ]),
    };
    let activities = client.issue::<Get>(request).await.unwrap();

    assert!(!activities.is_empty());

    for activity in activities {
      match activity {
        // A trade activity maps to the `Fill` type, so that is
        // expected.
        Activity::Trade(..) => (),
        Activity::NonTrade(non_trade) => {
          assert!(
            non_trade.type_ == ActivityType::Transaction
              || non_trade.type_ == ActivityType::Dividend
          );
        },
      }
    }
  }

  #[test(tokio::test)]
  async fn retrieve_trade_activities() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let request = ActivityReq {
      types: Some(vec![ActivityType::Fill]),
    };
    let activities = client.issue::<Get>(request).await.unwrap();

    assert!(!activities.is_empty());

    for activity in activities {
      match activity {
        Activity::Trade(..) => (),
        Activity::NonTrade(non_trade) => {
          panic!("received unexpected non-trade variant {:?}", non_trade)
        },
      }
    }
  }

  #[test(tokio::test)]
  async fn retrieve_all_activities() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let request = ActivityReq::default();
    let activities = client.issue::<Get>(request).await.unwrap();

    // We don't really have a better way to test this than testing that
    // we parsed something. Note that this may not work for newly
    // created accounts, an order may have to be filled first.
    assert!(!activities.is_empty());
  }
}
