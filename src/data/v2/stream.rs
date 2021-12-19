// Copyright (C) 2021 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::borrow::Borrow as _;
use std::borrow::Cow;
use std::cmp::Ordering;

use serde::ser::Serializer;
use serde::Serialize;

use crate::Str;


/// Serialize a `Symbol::Symbol` variant.
fn symbol_to_str<S>(symbol: &Str, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  serializer.serialize_str(symbol)
}


/// Serialize a `Symbol::All` variant.
fn symbol_all<S>(serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  serializer.serialize_str("*")
}


/// A symbol for which market data can be received.
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize)]
#[serde(untagged)]
pub enum Symbol {
  /// A symbol for a specific equity.
  #[serde(serialize_with = "symbol_to_str")]
  Symbol(Str),
  /// A "wildcard" symbol, representing all available equities.
  #[serde(serialize_with = "symbol_all")]
  All,
}

impl From<&'static str> for Symbol {
  fn from(symbol: &'static str) -> Self {
    if symbol == "*" {
      Symbol::All
    } else {
      Symbol::Symbol(Cow::from(symbol))
    }
  }
}


/// A slice/vector of [`Symbol`] objects.
pub type Symbols = Cow<'static, [Symbol]>;


/// Check whether a slice of `Symbol` objects is normalized.
///
/// Such a slice is normalized if:
/// - it is empty or
/// - it contains a single element `Symbol::All` or
/// - it does not contain `Symbol::All` and all symbols are lexically
///   ordered
fn is_normalized(symbols: &[Symbol]) -> bool {
  // The body here is effectively a copy of `Iterator::is_sorted_by`. We
  // should use that once it's stable.

  #[inline]
  fn check<'a>(last: &'a mut &'a Symbol) -> impl FnMut(&'a Symbol) -> bool + 'a {
    move |curr| {
      if let Some(Ordering::Greater) | None = PartialOrd::partial_cmp(last, &curr) {
        return false
      }
      *last = curr;
      true
    }
  }

  if symbols.len() > 1 && symbols.contains(&Symbol::All) {
    return false
  }

  let mut it = symbols.iter();
  let mut last = match it.next() {
    Some(e) => e,
    None => return true,
  };

  it.all(check(&mut last))
}


/// Normalize a list of symbols.
fn normalize(symbols: Symbols) -> Symbols {
  fn normalize_now(symbols: Symbols) -> Symbols {
    if symbols.contains(&Symbol::All) {
      Cow::from([Symbol::All].as_ref())
    } else {
      let mut symbols = symbols.into_owned();
      // Unwrapping here is fine, as we know that there is no
      // `Symbol::All` variant in the list and so we cannot encounter
      // variants that are not comparable.
      symbols.sort_by(|x, y| x.partial_cmp(y).unwrap());
      symbols.dedup();
      Cow::from(symbols)
    }
  }

  if !is_normalized((*symbols).borrow()) {
    let symbols = normalize_now(symbols);
    debug_assert!(is_normalized(&symbols));
    symbols
  } else {
    symbols
  }
}


#[cfg(test)]
mod tests {
  use super::*;


  /// Check that we can normalize `Symbol` slices.
  #[test]
  fn normalize_subscriptions() {
    let subscriptions = [Symbol::All];
    assert!(is_normalized(&subscriptions));

    let subscriptions = [Symbol::Symbol("MSFT".into()), Symbol::Symbol("SPY".into())];
    assert!(is_normalized(&subscriptions));

    let mut subscriptions = Cow::from(vec![
      Symbol::Symbol("SPY".into()),
      Symbol::Symbol("MSFT".into()),
    ]);
    assert!(!is_normalized(&subscriptions));
    subscriptions = normalize(subscriptions);
    assert!(is_normalized(&subscriptions));

    let expected = [Symbol::Symbol("MSFT".into()), Symbol::Symbol("SPY".into())];
    assert_eq!(subscriptions.as_ref(), expected.as_ref());

    let mut subscriptions = Cow::from(vec![
      Symbol::Symbol("SPY".into()),
      Symbol::All,
      Symbol::Symbol("MSFT".into()),
    ]);
    assert!(!is_normalized(&subscriptions));
    subscriptions = normalize(subscriptions);
    assert!(is_normalized(&subscriptions));

    let expected = [Symbol::All];
    assert_eq!(subscriptions.as_ref(), expected.as_ref());
  }
}
