// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::ops::Deref;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use serde::Serializer;

use uuid::Error as UuidError;
use uuid::Uuid;

use crate::Str;


/// An ID uniquely identifying an asset.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct Id(pub Uuid);

impl Deref for Id {
  type Target = Uuid;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}


/// An enumeration of the various asset classes available.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum Class {
  /// US equities.
  #[serde(rename = "us_equity")]
  UsEquity,
}

impl AsRef<str> for Class {
  fn as_ref(&self) -> &'static str {
    match *self {
      Class::UsEquity => "us_equity",
    }
  }
}

impl FromStr for Class {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if s == Class::UsEquity.as_ref() {
      Ok(Class::UsEquity)
    } else {
      Err(())
    }
  }
}


/// The status an asset can have.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum Status {
  /// The asset is active.
  #[serde(rename = "active")]
  Active,
  /// The asset is inactive.
  #[serde(rename = "inactive")]
  Inactive,
}

impl AsRef<str> for Status {
  fn as_ref(&self) -> &'static str {
    match *self {
      Status::Active => "active",
      Status::Inactive => "inactive",
    }
  }
}


/// An enumeration of all possible symbol parsing errors.
#[derive(Clone, Debug, PartialEq)]
pub enum ParseSymbolError {
  /// The symbol contains an invalid character.
  InvalidSymbol(char),
  /// The exchange is unknown.
  UnknownExchange,
  /// The asset class is unknown.
  UnknownClass,
  /// The ID could not be parsed.
  InvalidId(UuidError),
  /// The symbol has an invalid/unrecognized format.
  InvalidFormat,
}

impl Display for ParseSymbolError {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      Self::InvalidSymbol(c) => write!(fmt, "the symbol contains an invalid character ('{}')", c),
      Self::UnknownExchange => fmt.write_str("the exchange is unknown"),
      Self::UnknownClass => fmt.write_str("the asset class is unknown"),
      Self::InvalidId(err) => write!(fmt, "failed to parse asset ID: {}", err),
      Self::InvalidFormat => fmt.write_str("the symbol is of an invalid format"),
    }
  }
}


/// A symbol, and the various ways to represent it.
#[derive(Clone, Debug, PartialEq)]
pub enum Symbol {
  /// The symbol. Note that this is not a unique way to identify an
  /// asset (the same symbol may be used in different exchanges or asset
  /// classes).
  Sym(String),
  /// A symbol at a specific exchange.
  SymExchg(String, Exchange),
  /// A symbol for a given asset class at a specific exchange.
  SymExchgCls(String, Exchange, Class),
  /// An asset as described by an ID.
  Id(Id),
}

impl FromStr for Symbol {
  type Err = ParseSymbolError;

  fn from_str(sym: &str) -> Result<Self, Self::Err> {
    let sym = match sym.split(':').collect::<Vec<_>>().as_slice() {
      [sym] => {
        if let Ok(id) = Uuid::parse_str(sym) {
          Self::Id(Id(id))
        } else {
          let invalid = sym.as_bytes().iter().try_fold((), |(), c| {
            if !c.is_ascii_alphabetic() || !c.is_ascii_uppercase() {
              Err(*c as char)
            } else {
              Ok(())
            }
          });

          if let Err(c) = invalid {
            return Err(ParseSymbolError::InvalidSymbol(c))
          }
          Self::Sym(sym.to_string())
        }
      },
      [sym, exchg] => {
        let exchg = Exchange::from_str(exchg).map_err(|_| ParseSymbolError::UnknownExchange)?;

        Self::SymExchg(sym.to_string(), exchg)
      },
      [sym, exchg, cls] => {
        let exchg = Exchange::from_str(exchg).map_err(|_| ParseSymbolError::UnknownExchange)?;
        let cls = Class::from_str(cls).map_err(|_| ParseSymbolError::UnknownClass)?;

        Self::SymExchgCls(sym.to_string(), exchg, cls)
      },
      _ => return Err(ParseSymbolError::InvalidFormat),
    };
    Ok(sym)
  }
}

impl Display for Symbol {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      Self::Sym(sym) => fmt.write_str(&sym),
      Self::SymExchg(sym, exchg) => write!(fmt, "{}:{}", sym, exchg.as_ref()),
      Self::SymExchgCls(sym, exchg, cls) => {
        write!(fmt, "{}:{}:{}", sym, exchg.as_ref(), cls.as_ref())
      },
      Self::Id(id) => write!(fmt, "{}", id.to_hyphenated_ref()),
    }
  }
}

impl Serialize for Symbol {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&self.to_string())
  }
}


/// An enumeration of the various supported exchanges.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum Exchange {
  /// American Stock Exchange.
  #[serde(rename = "AMEX")]
  Amex,
  /// XXX
  // TODO: Not quite clear.
  #[serde(rename = "ARCA")]
  Arca,
  /// BATS Global Markets.
  #[serde(rename = "BATS")]
  Bats,
  /// Nasdaq Stock Market.
  #[serde(rename = "NASDAQ")]
  Nasdaq,
  /// New York Stock Exchange.
  #[serde(rename = "NYSE")]
  Nyse,
  /// NYSE Arca.
  #[serde(rename = "NYSEARCA")]
  Nysearca,
}

impl AsRef<str> for Exchange {
  fn as_ref(&self) -> &'static str {
    match *self {
      Exchange::Amex => "AMEX",
      Exchange::Arca => "ARCA",
      Exchange::Bats => "BATS",
      Exchange::Nasdaq => "NASDAQ",
      Exchange::Nyse => "NYSE",
      Exchange::Nysearca => "NYSEARCA",
    }
  }
}

impl FromStr for Exchange {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if s == Exchange::Amex.as_ref() {
      Ok(Exchange::Amex)
    } else if s == Exchange::Arca.as_ref() {
      Ok(Exchange::Arca)
    } else if s == Exchange::Bats.as_ref() {
      Ok(Exchange::Bats)
    } else if s == Exchange::Nasdaq.as_ref() {
      Ok(Exchange::Nasdaq)
    } else if s == Exchange::Nyse.as_ref() {
      Ok(Exchange::Nyse)
    } else if s == Exchange::Nysearca.as_ref() {
      Ok(Exchange::Nysearca)
    } else {
      Err(())
    }
  }
}


/// The representation of an asset as used by Alpaca.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Asset {
  /// The asset's ID.
  #[serde(rename = "id")]
  pub id: Id,
  /// The asset's class.
  #[serde(rename = "class")]
  pub class: Class,
  /// The exchange the asset is traded at.
  #[serde(rename = "exchange")]
  pub exchange: Exchange,
  /// The asset's symbol.
  #[serde(rename = "symbol")]
  pub symbol: String,
  /// The asset's status.
  #[serde(rename = "status")]
  pub status: Status,
  /// Whether the asset is tradable on Alpaca or not.
  #[serde(rename = "tradable")]
  pub tradable: bool,
  /// Whether the asset is marginable or not.
  #[serde(rename = "marginable")]
  pub marginable: bool,
  /// Whether the asset is shortable or not.
  #[serde(rename = "shortable")]
  pub shortable: bool,
  /// Whether the asset is considered easy-to-borrow or not.
  ///
  /// A value of `true` is a prerequisite for being able to short it.
  #[serde(rename = "easy_to_borrow")]
  pub easy_to_borrow: bool,
}


Endpoint! {
  /// The representation of a GET request to the /v2/assets/<symbol> endpoint.
  pub Get(Symbol),
  Ok => Asset, [
    /// The asset object for the given symbol was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// No asset was found for the given symbol.
    /* 404 */ NOT_FOUND => NotFound,
  ]

  fn path(input: &Self::Input) -> Str {
    format!("/v2/assets/{}", input).into()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use http_endpoint::Error as EndpointError;

  use serde_json::from_str as from_json;
  use serde_json::to_string as to_json;

  use test_env_log::test;

  use uuid::Uuid;

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::Error;


  #[test]
  fn parse_symbol() {
    let id = "b0b6dd9d-8b9b-48a9-ba46-b9d54906e415";
    assert_eq!(
      Symbol::from_str(id).unwrap(),
      Symbol::Id(Id(Uuid::parse_str(id).unwrap())),
    );

    assert_eq!(Symbol::from_str("SPY").unwrap(), Symbol::Sym("SPY".into()));

    assert_eq!(
      Symbol::from_str("SPY:NYSE").unwrap(),
      Symbol::SymExchg("SPY".into(), Exchange::Nyse),
    );

    assert_eq!(
      Symbol::from_str("AAPL:NASDAQ:us_equity").unwrap(),
      Symbol::SymExchgCls("AAPL".into(), Exchange::Nasdaq, Class::UsEquity),
    );

    assert_eq!(
      Symbol::from_str("AAPL:HIHI"),
      Err(ParseSymbolError::UnknownExchange),
    );
    assert_eq!(
      Symbol::from_str("AAPL:NASDAQ:blah"),
      Err(ParseSymbolError::UnknownClass),
    );
    assert_eq!(
      Symbol::from_str("Z%&Y"),
      Err(ParseSymbolError::InvalidSymbol('%')),
    );
    assert_eq!(
      Symbol::from_str("A:B:C:"),
      Err(ParseSymbolError::InvalidFormat),
    );
  }

  #[test]
  fn serialize_symbol() {
    let symbol = Symbol::Sym("AAPL".to_string());
    assert_eq!(to_json(&symbol).unwrap(), r#""AAPL""#);

    let symbol = Symbol::SymExchg("AAPL".to_string(), Exchange::Nasdaq);
    assert_eq!(to_json(&symbol).unwrap(), r#""AAPL:NASDAQ""#);

    let symbol = Symbol::SymExchgCls("AAPL".to_string(), Exchange::Nasdaq, Class::UsEquity);
    assert_eq!(to_json(&symbol).unwrap(), r#""AAPL:NASDAQ:us_equity""#);

    let id = Id(Uuid::parse_str("b0b6dd9d-8b9b-48a9-ba46-b9d54906e415").unwrap());
    let symbol = Symbol::Id(id);
    assert_eq!(to_json(&symbol).unwrap(), r#""b0b6dd9d-8b9b-48a9-ba46-b9d54906e415""#);
  }

  #[test]
  fn parse_reference_asset() {
    let response = r#"{
  "id": "904837e3-3b76-47ec-b432-046db621571b",
  "class": "us_equity",
  "exchange": "NASDAQ",
  "symbol": "AAPL",
  "status": "active",
  "tradable": true,
  "marginable": true,
  "shortable": true,
  "easy_to_borrow": true
}"#;

    let id = Id(Uuid::parse_str("904837e3-3b76-47ec-b432-046db621571b").unwrap());
    let asset = from_json::<Asset>(&response).unwrap();
    assert_eq!(asset.id, id);
    assert_eq!(asset.class, Class::UsEquity);
    assert_eq!(asset.exchange, Exchange::Nasdaq);
    assert_eq!(asset.symbol, "AAPL");
    assert_eq!(asset.status, Status::Active);
    assert_eq!(asset.tradable, true);
    assert_eq!(asset.marginable, true);
    assert_eq!(asset.shortable, true);
    assert_eq!(asset.easy_to_borrow, true);
  }

  #[test(tokio::test)]
  async fn retrieve_asset() -> Result<(), Error> {
    async fn test(symbol: Symbol) -> Result<(), Error> {
      let api_info = ApiInfo::from_env()?;
      let client = Client::new(api_info);
      let asset = client
        .issue::<Get>(symbol)
        .await
        .map_err(EndpointError::from)?;

      // The AAPL asset ID, retrieved out-of-band.
      let id = Id(Uuid::parse_str("b0b6dd9d-8b9b-48a9-ba46-b9d54906e415").unwrap());
      assert_eq!(asset.id, id);
      assert_eq!(asset.class, Class::UsEquity);
      assert_eq!(asset.exchange, Exchange::Nasdaq);
      assert_eq!(asset.symbol, "AAPL");
      assert_eq!(asset.status, Status::Active);
      assert_eq!(asset.tradable, true);
      Ok(())
    }

    let symbols = [
      Symbol::Sym("AAPL".to_string()),
      Symbol::SymExchg("AAPL".to_string(), Exchange::Nasdaq),
      Symbol::SymExchgCls("AAPL".to_string(), Exchange::Nasdaq, Class::UsEquity),
      Symbol::Id(Id(Uuid::parse_str("b0b6dd9d-8b9b-48a9-ba46-b9d54906e415").unwrap())),
    ];

    for symbol in symbols.into_iter().cloned() {
      test(symbol).await?;
    }
    Ok(())
  }
}
