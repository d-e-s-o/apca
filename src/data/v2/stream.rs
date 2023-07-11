// Copyright (C) 2021-2023 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::borrow::Borrow as _;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Deref;

use async_trait::async_trait;

use chrono::DateTime;
use chrono::Utc;

use futures::stream::Fuse;
use futures::stream::FusedStream;
use futures::stream::Map;
use futures::stream::SplitSink;
use futures::stream::SplitStream;
use futures::Future;
use futures::FutureExt as _;
use futures::Sink;
use futures::StreamExt as _;

use num_decimal::Num;

use serde::de::DeserializeOwned;
use serde::de::Deserializer;
use serde::ser::SerializeSeq as _;
use serde::ser::Serializer;
use serde::Deserialize;
use serde::Serialize;
use serde_json::from_slice as json_from_slice;
use serde_json::from_str as json_from_str;
use serde_json::to_string as to_json;
use serde_json::Error as JsonError;

use thiserror::Error as ThisError;

use tokio::net::TcpStream;

use tungstenite::MaybeTlsStream;
use tungstenite::WebSocketStream;

use url::Url;

use websocket_util::subscribe;
use websocket_util::subscribe::MessageStream;
use websocket_util::tungstenite::Error as WebSocketError;
use websocket_util::wrap;
use websocket_util::wrap::Wrapper;

use super::unfold::Unfold;

use crate::subscribable::Subscribable;
use crate::websocket::connect;
use crate::websocket::MessageResult;
use crate::ApiInfo;
use crate::Error;
use crate::Str;


type UserMessage<B, Q, T> = <ParsedMessage<B, Q, T> as subscribe::Message>::UserMessage;

/// Helper function to drive a [`Subscription`] related future to
/// completion. The function makes sure to poll the provided stream,
/// which is assumed to be associated with the `Subscription` that the
/// future belongs to, so that control messages can be received.
#[inline]
pub async fn drive<F, S, B, Q, T>(
  future: F,
  stream: &mut S,
) -> Result<F::Output, UserMessage<B, Q, T>>
where
  F: Future + Unpin,
  S: FusedStream<Item = UserMessage<B, Q, T>> + Unpin,
{
  subscribe::drive::<ParsedMessage<B, Q, T>, _, _>(future, stream).await
}


mod private {
  pub trait Sealed {}
}


#[doc(hidden)]
#[derive(Clone, Debug)]
pub enum SourceVariant {
  /// The source provided is a path component to be appended to an
  /// already present base URL.
  PathComponent(&'static str),
  /// The source provided is a complete URL.
  Url(String),
}


/// A trait representing the source from which to stream real time data.
// TODO: Once we can use enumerations as const generic parameters we
//       should probably switch over to repurposing `data::v2::Feed`
//       here instead.
pub trait Source: private::Sealed {
  /// Return the source.
  #[doc(hidden)]
  fn source() -> SourceVariant;
}


/// Use the Investors Exchange (IEX) as the data source.
///
/// This source is available unconditionally, i.e., with the free and
/// unlimited plans.
#[derive(Clone, Copy, Debug)]
pub enum IEX {}

impl Source for IEX {
  #[inline]
  fn source() -> SourceVariant {
    SourceVariant::PathComponent("iex")
  }
}

impl private::Sealed for IEX {}


/// Use CTA (administered by NYSE) and UTP (administered by Nasdaq) SIPs
/// as the data source.
///
/// This source is only usable with the unlimited market data plan.
#[derive(Clone, Copy, Debug)]
pub enum SIP {}

impl Source for SIP {
  #[inline]
  fn source() -> SourceVariant {
    SourceVariant::PathComponent("sip")
  }
}

impl private::Sealed for SIP {}


/// A realtime data source that uses a custom URL.
///
/// This type provides a way to stream realtime data from a custom URL.
/// The endpoint at said URL has to follow the `v2` handshake and
/// message protocol. Provided that is the case, usage could be as
/// follows:
/// ```no_run
/// # use apca::ApiInfo;
/// # use apca::Client;
/// # use apca::data::v2::stream::CustomUrl;
/// # use apca::data::v2::stream::RealtimeData;
/// // The v1beta3 crypto API happens to be using the same handshake as
/// // the v2 stock APIs.
/// #[derive(Default)]
/// struct Crypto;
///
/// impl ToString for Crypto {
///   fn to_string(&self) -> String {
///     "wss://stream.data.alpaca.markets/v1beta3/crypto/us".into()
///   }
/// }
///
/// let api_info = ApiInfo::from_env().unwrap();
/// let client = Client::new(api_info);
/// # tokio::runtime::Runtime::new().unwrap().block_on(async move {
/// let (mut stream, mut subscription) = client
///   .subscribe::<RealtimeData<CustomUrl<Crypto>>>()
///   .await
///   .unwrap();
/// # })
///
/// // Use `subscription` to subscribe to quotes, trades, or bars, then
/// // handle `stream` as usual.
/// ```
#[derive(Clone, Copy, Debug)]
pub struct CustomUrl<URL> {
  _phantom: PhantomData<URL>,
}

impl<URL> Source for CustomUrl<URL>
where
  URL: Default + ToString,
{
  #[inline]
  fn source() -> SourceVariant {
    let url = URL::default();
    SourceVariant::Url(url.to_string())
  }
}

impl<URL> private::Sealed for CustomUrl<URL> {}


/// A symbol.
pub type Symbol = Str;


/// Check whether a slice of symbols is normalized.
///
/// Such a slice is normalized if it is sorted lexically and all
/// duplicates are removed.
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

  let mut it = symbols.iter();
  let mut last = match it.next() {
    Some(e) => e,
    None => return true,
  };

  it.all(check(&mut last))
}


/// Normalize a list of symbols.
fn normalize(symbols: Cow<'static, [Symbol]>) -> Cow<'static, [Symbol]> {
  fn normalize_now(symbols: Cow<'static, [Symbol]>) -> Cow<'static, [Symbol]> {
    let mut symbols = symbols.into_owned();
    // Unwrapping here is fine, as we know that there is no
    // `Symbol::All` variant in the list and so we cannot encounter
    // variants that are not comparable.
    symbols.sort_by(|x, y| x.partial_cmp(y).unwrap());
    symbols.dedup();
    Cow::from(symbols)
  }

  if !is_normalized(&symbols) {
    let symbols = normalize_now(symbols);
    debug_assert!(is_normalized(&symbols));
    symbols
  } else {
    symbols
  }
}


/// Aggregate data for an equity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bar {
  /// The bar's symbol.
  #[serde(rename = "S")]
  pub symbol: String,
  /// The bar's open price.
  #[serde(rename = "o")]
  pub open_price: Num,
  /// The bar's high price.
  #[serde(rename = "h")]
  pub high_price: Num,
  /// The bar's low price.
  #[serde(rename = "l")]
  pub low_price: Num,
  /// The bar's close price.
  #[serde(rename = "c")]
  pub close_price: Num,
  /// The bar's volume.
  #[serde(rename = "v")]
  pub volume: Num,
  /// The bar's time stamp.
  #[serde(rename = "t")]
  pub timestamp: DateTime<Utc>,
}


/// A quote for an equity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Quote {
  /// The quote's symbol.
  #[serde(rename = "S")]
  pub symbol: String,
  /// The bid's price.
  #[serde(rename = "bp")]
  pub bid_price: Num,
  /// The bid's size.
  #[serde(rename = "bs")]
  pub bid_size: Num,
  /// The ask's price.
  #[serde(rename = "ap")]
  pub ask_price: Num,
  /// The ask's size.
  #[serde(rename = "as")]
  pub ask_size: Num,
  /// The quote's time stamp.
  #[serde(rename = "t")]
  pub timestamp: DateTime<Utc>,
}


/// A trade for an equity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Trade {
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
  pub trade_size: Num,
  /// The trade's time stamp.
  #[serde(rename = "t")]
  pub timestamp: DateTime<Utc>,
}


/// An error as reported by the Alpaca Stream API.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ThisError)]
#[error("{message} ({code})")]
pub struct StreamApiError {
  /// The error code being reported.
  #[serde(rename = "code")]
  pub code: u64,
  /// A message providing more details about the error.
  #[serde(rename = "msg")]
  pub message: String,
}


/// An enum representing the different messages we may receive over our
/// websocket channel.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[doc(hidden)]
#[serde(tag = "T")]
#[allow(clippy::large_enum_variant)]
pub enum DataMessage<B = Bar, Q = Quote, T = Trade> {
  /// A variant representing aggregate data for a given symbol.
  #[serde(rename = "b")]
  Bar(B),
  /// A variant representing a quote for a given symbol.
  #[serde(rename = "q")]
  Quote(Q),
  /// A variant representing a trade for a given symbol.
  #[serde(rename = "t")]
  Trade(T),
  /// A control message describing the current list of subscriptions.
  #[serde(rename = "subscription")]
  Subscription(MarketData),
  /// A control message indicating that the last operation was
  /// successful.
  #[serde(rename = "success")]
  Success,
  /// An error reported by the Alpaca Stream API.
  #[serde(rename = "error")]
  Error(StreamApiError),
}


/// A data item as received over our websocket channel.
#[derive(Debug)]
#[non_exhaustive]
pub enum Data<B = Bar, Q = Quote, T = Trade> {
  /// A variant representing aggregate data for a given symbol.
  Bar(B),
  /// A variant representing quote data for a given symbol.
  Quote(Q),
  /// A variant representing trade data for a given symbol.
  Trade(T),
}

impl Data {
  /// Check whether this object is of the `Bar` variant.
  #[inline]
  pub fn is_bar(&self) -> bool {
    matches!(self, Self::Bar(..))
  }

  /// Check whether this object is of the `Quote` variant.
  #[inline]
  pub fn is_quote(&self) -> bool {
    matches!(self, Self::Quote(..))
  }

  /// Check whether this object is of the `Trade` variant.
  #[inline]
  pub fn is_trade(&self) -> bool {
    matches!(self, Self::Trade(..))
  }
}


/// An enumeration of the supported control messages.
#[derive(Debug)]
#[doc(hidden)]
pub enum ControlMessage {
  /// A control message describing the current list of subscriptions.
  Subscription(MarketData),
  /// A control message indicating that the last operation was
  /// successful.
  Success,
  /// An error reported by the Alpaca Stream API.
  Error(StreamApiError),
}


/// A websocket message that we tried to parse.
type ParsedMessage<B, Q, T> =
  MessageResult<Result<DataMessage<B, Q, T>, JsonError>, WebSocketError>;

impl<B, Q, T> subscribe::Message for ParsedMessage<B, Q, T> {
  type UserMessage = Result<Result<Data<B, Q, T>, JsonError>, WebSocketError>;
  type ControlMessage = ControlMessage;

  fn classify(self) -> subscribe::Classification<Self::UserMessage, Self::ControlMessage> {
    match self {
      MessageResult::Ok(Ok(message)) => match message {
        DataMessage::Bar(bar) => subscribe::Classification::UserMessage(Ok(Ok(Data::Bar(bar)))),
        DataMessage::Quote(quote) => {
          subscribe::Classification::UserMessage(Ok(Ok(Data::Quote(quote))))
        },
        DataMessage::Trade(trade) => {
          subscribe::Classification::UserMessage(Ok(Ok(Data::Trade(trade))))
        },
        DataMessage::Subscription(data) => {
          subscribe::Classification::ControlMessage(ControlMessage::Subscription(data))
        },
        DataMessage::Success => subscribe::Classification::ControlMessage(ControlMessage::Success),
        DataMessage::Error(error) => {
          subscribe::Classification::ControlMessage(ControlMessage::Error(error))
        },
      },
      // JSON errors are directly passed through.
      MessageResult::Ok(Err(err)) => subscribe::Classification::UserMessage(Ok(Err(err))),
      // WebSocket errors are also directly pushed through.
      MessageResult::Err(err) => subscribe::Classification::UserMessage(Err(err)),
    }
  }

  #[inline]
  fn is_error(user_message: &Self::UserMessage) -> bool {
    // Both outer `WebSocketError` and inner `JsonError` errors
    // constitute errors in our sense. Note, however, that an API error
    // does not. It's just a regular control message from our
    // perspective.
    user_message
      .as_ref()
      .map(|result| result.is_err())
      .unwrap_or(true)
  }
}


/// Deserialize a normalized list of symbols from a string.
#[inline]
fn normalized_from_str<'de, D>(deserializer: D) -> Result<Cow<'static, [Symbol]>, D::Error>
where
  D: Deserializer<'de>,
{
  Cow::<'static, [Symbol]>::deserialize(deserializer).map(normalize)
}


/// A type representing a normalized list of symbols.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SymbolList(#[serde(deserialize_with = "normalized_from_str")] Cow<'static, [Symbol]>);

impl Deref for SymbolList {
  type Target = [Symbol];

  fn deref(&self) -> &Self::Target {
    self.0.borrow()
  }
}

impl From<Cow<'static, [Symbol]>> for SymbolList {
  #[inline]
  fn from(symbols: Cow<'static, [Symbol]>) -> Self {
    Self(normalize(symbols))
  }
}

impl From<Vec<String>> for SymbolList {
  #[inline]
  fn from(symbols: Vec<String>) -> Self {
    Self(normalize(Cow::from(
      IntoIterator::into_iter(symbols)
        .map(Symbol::from)
        .collect::<Vec<_>>(),
    )))
  }
}

impl<const N: usize> From<[&'static str; N]> for SymbolList {
  #[inline]
  fn from(symbols: [&'static str; N]) -> Self {
    Self(normalize(Cow::from(
      IntoIterator::into_iter(symbols)
        .map(Symbol::from)
        .collect::<Vec<_>>(),
    )))
  }
}


mod symbols_all {
  use super::*;

  use serde::de::Error;
  use serde::de::Unexpected;

  /// Deserialize the [`Symbols::All`] variant.
  pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<(), D::Error>
  where
    D: Deserializer<'de>,
  {
    let string = <[&str; 1]>::deserialize(deserializer)?;
    if string == ["*"] {
      Ok(())
    } else {
      Err(Error::invalid_value(
        Unexpected::Str(string[0]),
        &"the string \"*\"",
      ))
    }
  }

  /// Serialize the [`Symbols::All`] variant.
  pub(crate) fn serialize<S>(serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let mut seq = serializer.serialize_seq(Some(1))?;
    seq.serialize_element("*")?;
    seq.end()
  }
}


/// An enumeration of symbols to subscribe to.
// Please note that the order of variants is important for
// deserialization purposes: we first need to check whether we are
// dealing with the `All` variant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Symbols {
  /// A variant representing all symbols.
  #[serde(with = "symbols_all")]
  All,
  /// A list of symbols to work with.
  List(SymbolList),
}

impl Symbols {
  /// Check whether the object represents no symbols.
  #[inline]
  pub fn is_empty(&self) -> bool {
    match self {
      Self::List(list) => list.is_empty(),
      Self::All => false,
    }
  }
}

impl Default for Symbols {
  fn default() -> Self {
    Self::List(SymbolList::from([]))
  }
}


/// A type defining the market data a client intends to subscribe to.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct MarketData {
  /// The aggregate bars to subscribe to.
  #[serde(default)]
  pub bars: Symbols,
  /// The quotes to subscribe to.
  #[serde(default)]
  pub quotes: Symbols,
  /// The trades to subscribe to.
  #[serde(default)]
  pub trades: Symbols,
}

impl MarketData {
  /// A convenience function for setting the [`bars`][MarketData::bars]
  /// member.
  #[inline]
  pub fn set_bars<S>(&mut self, symbols: S)
  where
    S: Into<SymbolList>,
  {
    self.bars = Symbols::List(symbols.into());
  }

  /// A convenience function for setting the [`quotes`][MarketData::quotes]
  /// member.
  #[inline]
  pub fn set_quotes<S>(&mut self, symbols: S)
  where
    S: Into<SymbolList>,
  {
    self.quotes = Symbols::List(symbols.into());
  }

  /// A convenience function for setting the [`trades`][MarketData::trades]
  /// member.
  #[inline]
  pub fn set_trades<S>(&mut self, symbols: S)
  where
    S: Into<SymbolList>,
  {
    self.trades = Symbols::List(symbols.into());
  }
}


/// A control message "request" sent over a websocket channel.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[doc(hidden)]
#[serde(tag = "action")]
pub enum Request<'d> {
  /// A control message indicating whether or not we were authenticated
  /// successfully.
  #[serde(rename = "auth")]
  Authenticate {
    #[serde(rename = "key")]
    key_id: Cow<'d, str>,
    #[serde(rename = "secret")]
    secret: Cow<'d, str>,
  },
  /// A control message subscribing the client to receive updates for
  /// the provided symbols.
  #[serde(rename = "subscribe")]
  Subscribe(Cow<'d, MarketData>),
  /// A control message unsubscribing the client from receiving updates
  /// for the provided symbols.
  #[serde(rename = "unsubscribe")]
  Unsubscribe(Cow<'d, MarketData>),
}


/// A subscription allowing certain control operations pertaining
/// a real time market data stream.
///
/// # Notes
/// - in order for any [`subscribe`][Subscription::subscribe] or
///   [`unsubscribe`][Subscription::unsubscribe] operation to resolve,
///   the associated [`MessageStream`] stream needs to be polled;
///   consider using the [`drive`] function for that purpose
#[derive(Debug)]
pub struct Subscription<S, B, Q, T> {
  /// Our internally used subscription object for sending control
  /// messages.
  subscription: subscribe::Subscription<S, ParsedMessage<B, Q, T>, wrap::Message>,
  /// The currently active individual market data subscriptions.
  subscriptions: MarketData,
}

impl<S, B, Q, T> Subscription<S, B, Q, T> {
  /// Create a `Subscription` object wrapping the `websocket_util` based one.
  #[inline]
  fn new(subscription: subscribe::Subscription<S, ParsedMessage<B, Q, T>, wrap::Message>) -> Self {
    Self {
      subscription,
      subscriptions: MarketData::default(),
    }
  }
}

impl<S, B, Q, T> Subscription<S, B, Q, T>
where
  S: Sink<wrap::Message> + Unpin,
{
  /// Authenticate the connection using Alpaca credentials.
  async fn authenticate(
    &mut self,
    key_id: &str,
    secret: &str,
  ) -> Result<Result<(), Error>, S::Error> {
    let request = Request::Authenticate {
      key_id: key_id.into(),
      secret: secret.into(),
    };
    let json = match to_json(&request) {
      Ok(json) => json,
      Err(err) => return Ok(Err(Error::Json(err))),
    };
    let message = wrap::Message::Text(json);
    let response = self.subscription.send(message).await?;

    match response {
      Some(response) => match response {
        Ok(ControlMessage::Success) => Ok(Ok(())),
        Ok(ControlMessage::Subscription(..)) => Ok(Err(Error::Str(
          "server responded with unexpected subscription message".into(),
        ))),
        Ok(ControlMessage::Error(error)) => Ok(Err(Error::Str(
          format!(
            "failed to authenticate with server: {} ({})",
            error.message, error.code
          )
          .into(),
        ))),
        Err(()) => Ok(Err(Error::Str("failed to authenticate with server".into()))),
      },
      None => Ok(Err(Error::Str(
        "stream was closed before authorization message was received".into(),
      ))),
    }
  }

  /// Handle sending of a subscribe or unsubscribe request.
  async fn subscribe_unsubscribe(
    &mut self,
    request: &Request<'_>,
  ) -> Result<Result<(), Error>, S::Error> {
    let json = match to_json(request) {
      Ok(json) => json,
      Err(err) => return Ok(Err(Error::Json(err))),
    };
    let message = wrap::Message::Text(json);
    let response = self.subscription.send(message).await?;

    match response {
      Some(response) => match response {
        Ok(ControlMessage::Subscription(data)) => {
          self.subscriptions = data;
          Ok(Ok(()))
        },
        Ok(ControlMessage::Error(error)) => Ok(Err(Error::Str(
          format!("failed to subscribe: {error}").into(),
        ))),
        Ok(_) => Ok(Err(Error::Str(
          "server responded with unexpected message".into(),
        ))),
        Err(()) => Ok(Err(Error::Str("failed to adjust subscription".into()))),
      },
      None => Ok(Err(Error::Str(
        "stream was closed before subscription confirmation message was received".into(),
      ))),
    }
  }

  /// Subscribe to the provided market data.
  ///
  /// Contained in `subscribe` are the *additional* symbols to subscribe
  /// to. Use the [`unsubscribe`][Self::unsubscribe] method to
  /// unsubscribe from receiving data for certain symbols.
  #[inline]
  pub async fn subscribe(&mut self, subscribe: &MarketData) -> Result<Result<(), Error>, S::Error> {
    let request = Request::Subscribe(Cow::Borrowed(subscribe));
    self.subscribe_unsubscribe(&request).await
  }

  /// Unsubscribe from receiving market data for the provided symbols.
  ///
  /// Subscriptions of market data for symbols other than the ones
  /// provided to this function are left untouched.
  #[inline]
  pub async fn unsubscribe(
    &mut self,
    unsubscribe: &MarketData,
  ) -> Result<Result<(), Error>, S::Error> {
    let request = Request::Unsubscribe(Cow::Borrowed(unsubscribe));
    self.subscribe_unsubscribe(&request).await
  }

  /// Inquire the currently active individual market data subscriptions.
  #[inline]
  pub fn subscriptions(&self) -> &MarketData {
    &self.subscriptions
  }
}


type ParseFn<B, Q, T> = fn(
  Result<wrap::Message, WebSocketError>,
) -> Result<Result<Vec<DataMessage<B, Q, T>>, JsonError>, WebSocketError>;
type MapFn<B, Q, T> =
  fn(Result<Result<DataMessage<B, Q, T>, JsonError>, WebSocketError>) -> ParsedMessage<B, Q, T>;
type Stream<B, Q, T> = Map<
  Unfold<
    Map<Wrapper<WebSocketStream<MaybeTlsStream<TcpStream>>>, ParseFn<B, Q, T>>,
    DataMessage<B, Q, T>,
    JsonError,
  >,
  MapFn<B, Q, T>,
>;


/// A type used for requesting a subscription to real time market
/// data.
///
/// The bar (`B`), quote (`Q`), and trade (`T`) types used can be
/// overwritten to extend/customize the default types ([`Bar`],
/// [`Quote`], and [`Trade`], respectively) that are provided by the
/// library.
#[derive(Debug)]
pub struct RealtimeData<S, B = Bar, Q = Quote, T = Trade> {
  /// Phantom data to make sure that we "use" `S`.
  _phantom: PhantomData<(S, B, Q, T)>,
}

#[async_trait]
impl<S, B, Q, T> Subscribable for RealtimeData<S, B, Q, T>
where
  S: Source,
  B: Send + Unpin + Debug + DeserializeOwned,
  Q: Send + Unpin + Debug + DeserializeOwned,
  T: Send + Unpin + Debug + DeserializeOwned,
{
  type Input = ApiInfo;
  type Subscription = Subscription<SplitSink<Stream<B, Q, T>, wrap::Message>, B, Q, T>;
  type Stream = Fuse<MessageStream<SplitStream<Stream<B, Q, T>>, ParsedMessage<B, Q, T>>>;

  async fn connect(api_info: &Self::Input) -> Result<(Self::Stream, Self::Subscription), Error> {
    fn parse<B, Q, T>(
      result: Result<wrap::Message, WebSocketError>,
    ) -> Result<Result<Vec<DataMessage<B, Q, T>>, JsonError>, WebSocketError>
    where
      B: DeserializeOwned,
      Q: DeserializeOwned,
      T: DeserializeOwned,
    {
      result.map(|message| match message {
        wrap::Message::Text(string) => json_from_str::<Vec<DataMessage<B, Q, T>>>(&string),
        wrap::Message::Binary(data) => json_from_slice::<Vec<DataMessage<B, Q, T>>>(&data),
      })
    }

    let ApiInfo {
      data_stream_base_url: url,
      key_id,
      secret,
      ..
    } = api_info;

    let url = match S::source() {
      SourceVariant::PathComponent(component) => {
        let mut url = url.clone();
        url.set_path(&format!("v2/{}", component));
        url
      },
      SourceVariant::Url(url) => Url::parse(&url)?,
    };

    let stream = Unfold::new(
      connect(&url)
        .await?
        .map(parse::<B, Q, T> as ParseFn<_, _, _>),
    )
    .map(MessageResult::from as MapFn<B, Q, T>);
    let (send, recv) = stream.split();
    let (stream, subscription) = subscribe::subscribe(recv, send);
    let mut stream = stream.fuse();
    let mut subscription = Subscription::new(subscription);

    let connect = subscription.subscription.read().boxed();
    let message = drive(connect, &mut stream).await.map_err(|result| {
      result
        .map(|result| Error::Json(result.unwrap_err()))
        .map_err(Error::WebSocket)
        .unwrap_or_else(|err| err)
    })?;

    match message {
      Some(Ok(ControlMessage::Success)) => (),
      Some(Ok(_)) => {
        return Err(Error::Str(
          "server responded with unexpected initial message".into(),
        ))
      },
      Some(Err(())) => return Err(Error::Str("failed to read connected message".into())),
      None => {
        return Err(Error::Str(
          "stream was closed before connected message was received".into(),
        ))
      },
    }

    let authenticate = subscription.authenticate(key_id, secret).boxed();
    let () = drive(authenticate, &mut stream).await.map_err(|result| {
      result
        .map(|result| Error::Json(result.unwrap_err()))
        .map_err(Error::WebSocket)
        .unwrap_or_else(|err| err)
    })???;

    Ok((stream, subscription))
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::str::FromStr;
  use std::time::Duration;

  use chrono::DateTime;

  use futures::SinkExt as _;
  use futures::TryStreamExt as _;

  use serial_test::serial;

  use serde_json::from_str as json_from_str;

  use test_log::test;

  use tokio::time::timeout;

  use websocket_util::test::WebSocketStream;
  use websocket_util::tungstenite::Message;

  use crate::api::API_BASE_URL;
  use crate::websocket::test::mock_stream;
  use crate::Client;


  const CONN_RESP: &str = r#"[{"T":"success","msg":"connected"}]"#;
  // TODO: Until we can interpolate more complex expressions using
  //       `std::format` in a const context we have to hard code the
  //       values of `crate::websocket::test::KEY_ID` and
  //       `crate::websocket::test::SECRET` here.
  const AUTH_REQ: &str = r#"{"action":"auth","key":"USER12345678","secret":"justletmein"}"#;
  const AUTH_RESP: &str = r#"[{"T":"success","msg":"authenticated"}]"#;
  const SUB_REQ: &str = r#"{"action":"subscribe","bars":["AAPL","VOO"],"quotes":[],"trades":[]}"#;
  const SUB_RESP: &str = r#"[{"T":"subscription","bars":["AAPL","VOO"]}]"#;
  const SUB_ERR_REQ: &str = r#"{"action":"subscribe","bars":[],"quotes":[],"trades":[]}"#;
  const SUB_ERR_RESP: &str = r#"[{"T":"error","code":400,"msg":"invalid syntax"}]"#;


  /// Test that the [`Symbols::is_empty`] method works as expected.
  #[test]
  fn symbols_is_empty() {
    assert!(!Symbols::All.is_empty());
    assert!(!Symbols::List(SymbolList::from(["SPY"])).is_empty());
    assert!(Symbols::List(SymbolList::from([])).is_empty());
  }

  /// Check that we can deserialize and serialize the
  /// [`DataMessage::Bar`] variant.
  #[test]
  fn serialize_deserialize_bar() {
    let json = r#"{
  "T": "b",
  "S": "SPY",
  "o": 388.985,
  "h": 389.13,
  "l": 388.975,
  "c": 389.12,
  "v": 49378,
  "t": "2021-02-22T19:15:00Z"
}"#;

    let message = json_from_str::<DataMessage>(json).unwrap();
    let bar = match &message {
      DataMessage::Bar(bar) => bar,
      _ => panic!("Decoded unexpected message variant: {message:?}"),
    };
    assert_eq!(bar.symbol, "SPY");
    assert_eq!(bar.open_price, Num::new(388985, 1000));
    assert_eq!(bar.high_price, Num::new(38913, 100));
    assert_eq!(bar.low_price, Num::new(388975, 1000));
    assert_eq!(bar.close_price, Num::new(38912, 100));
    assert_eq!(bar.volume, Num::from(49378));
    assert_eq!(
      bar.timestamp,
      DateTime::<Utc>::from_str("2021-02-22T19:15:00Z").unwrap()
    );

    assert_eq!(
      json_from_str::<DataMessage>(&to_json(&message).unwrap()).unwrap(),
      message
    );
  }

  /// Check that we can serialize and deserialize the
  /// [`DataMessage::Quote`] variant.
  #[test]
  fn serialize_deserialize_quote() {
    let json: &str = r#"{
  "T": "q",
  "S": "NVDA",
  "bx": "P",
  "bp": 258.8,
  "bs": 2,
  "ax": "A",
  "ap": 259.99,
  "as": 5,
  "c": [
      "R"
  ],
  "z": "C",
  "t": "2022-01-18T23:09:42.151875584Z"
}"#;

    let message = json_from_str::<DataMessage>(json).unwrap();
    let quote = match &message {
      DataMessage::Quote(quote) => quote,
      _ => panic!("Decoded unexpected message variant: {message:?}"),
    };
    assert_eq!(quote.symbol, "NVDA");
    assert_eq!(quote.bid_price, Num::new(2588, 10));
    assert_eq!(quote.bid_size, Num::from(2));
    assert_eq!(quote.ask_price, Num::new(25999, 100));
    assert_eq!(quote.ask_size, Num::from(5));

    assert_eq!(
      quote.timestamp,
      DateTime::<Utc>::from_str("2022-01-18T23:09:42.151875584Z").unwrap()
    );

    assert_eq!(
      json_from_str::<DataMessage>(&to_json(&message).unwrap()).unwrap(),
      message
    );
  }


  /// A quote for an equity.
  #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
  struct DetailedQuote {
    /// The quote's symbol.
    #[serde(rename = "S")]
    symbol: String,
    /// The bid's price.
    #[serde(rename = "bp")]
    bid_price: Num,
    /// The bid's size.
    #[serde(rename = "bs")]
    bid_size: Num,
    /// The bid's exchange code.
    #[serde(rename = "bx")]
    bid_exchange_code: String,
    /// The ask's price.
    #[serde(rename = "ap")]
    ask_price: Num,
    /// The ask's size.
    #[serde(rename = "as")]
    ask_size: Num,
    /// The ask's exchange code.
    #[serde(rename = "ax")]
    ask_exchange_code: String,
    /// The trade's size.
    #[serde(rename = "s")]
    trade_size: Num,
    /// The quote's time stamp.
    #[serde(rename = "t")]
    timestamp: DateTime<Utc>,
    /// The quote's conditions.
    #[serde(rename = "c")]
    quote_conditions: Vec<String>,
    /// The tape.
    #[serde(rename = "z")]
    tape: String,
  }

  /// Check that we can serialize and deserialize the
  /// [`DataMessage::Quote`] variant with a custom quote type.
  #[test]
  fn serialize_deserialize_custom_quote() {
    let json: &str = r#"{
  "T": "q",
  "S": "NVDA",
  "bx": "P",
  "bp": 258.8,
  "bs": 2,
  "s": 3,
  "ax": "A",
  "ap": 259.99,
  "as": 5,
  "c": [
      "R"
  ],
  "z": "C",
  "t": "2022-01-18T23:09:42.151875584Z"
}"#;

    let message = json_from_str::<DataMessage<Bar, DetailedQuote, Trade>>(json).unwrap();
    let quote = match &message {
      DataMessage::Quote(quote) => quote,
      _ => panic!("Decoded unexpected message variant: {message:?}"),
    };
    assert_eq!(quote.symbol, "NVDA");
    assert_eq!(quote.bid_price, Num::new(2588, 10));
    assert_eq!(quote.bid_size, Num::from(2));
    assert_eq!(quote.bid_exchange_code, "P");
    assert_eq!(quote.ask_price, Num::new(25999, 100));
    assert_eq!(quote.ask_size, Num::from(5));
    assert_eq!(quote.ask_exchange_code, "A");
    assert_eq!(quote.trade_size, Num::from(3));

    assert_eq!(
      quote.timestamp,
      DateTime::<Utc>::from_str("2022-01-18T23:09:42.151875584Z").unwrap()
    );

    assert_eq!(
      json_from_str::<DataMessage<Bar, DetailedQuote, Trade>>(&to_json(&message).unwrap()).unwrap(),
      message
    );
  }

  /// Check that we can serialize and deserialize the
  /// [`DataMessage::Trade`] variant.
  #[test]
  fn serialize_deserialize_trade() {
    let json: &str = r#"{
  "T": "t",
  "i": 96921,
  "S": "AAPL",
  "x": "D",
  "p": 126.55,
  "s": 1,
  "t": "2021-02-22T15:51:44.208Z",
  "c": ["@", "I"],
  "z": "C"
}"#;

    let message = json_from_str::<DataMessage>(json).unwrap();
    let trade = match &message {
      DataMessage::Trade(trade) => trade,
      _ => panic!("Decoded unexpected message variant: {message:?}"),
    };
    assert_eq!(trade.symbol, "AAPL");
    assert_eq!(trade.trade_id, 96921);
    assert_eq!(trade.trade_price, Num::new(12655, 100));
    assert_eq!(trade.trade_size, Num::from(1));

    assert_eq!(
      trade.timestamp,
      DateTime::<Utc>::from_str("2021-02-22T15:51:44.208Z").unwrap()
    );

    assert_eq!(
      json_from_str::<DataMessage>(&to_json(&message).unwrap()).unwrap(),
      message
    );
  }


  /// A trade for an equity.
  #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
  struct DetailedTrade {
    /// The trade's symbol.
    #[serde(rename = "S")]
    symbol: String,
    /// The trade's ID.
    #[serde(rename = "i")]
    trade_id: u64,
    /// The trade's price.
    #[serde(rename = "p")]
    trade_price: Num,
    /// The trade's size.
    #[serde(rename = "s")]
    trade_size: u64,
    /// The trade's conditions.
    #[serde(rename = "c")]
    conditions: Vec<String>,
    /// The trade's time stamp.
    #[serde(rename = "t")]
    timestamp: DateTime<Utc>,
    /// The trade's exchange.
    #[serde(rename = "x")]
    exchange: String,
    /// The trade's tape.
    #[serde(rename = "z")]
    tape: String,
    /// The trade's update, may be "canceled", "corrected", or
    /// "incorrect".
    #[serde(rename = "u", default)]
    update: Option<String>,
  }

  /// Check that we can serialize and deserialize the
  /// [`DataMessage::Trade`] variant with a custom trade type.
  #[test]
  fn serialize_deserialize_custom_trade() {
    let json: &str = r#"{
  "T": "t",
  "i": 96921,
  "S": "AAPL",
  "x": "D",
  "p": 126.55,
  "s": 1,
  "t": "2021-02-22T15:51:44.208Z",
  "c": ["@", "I"],
  "z": "C",
  "u": "corrected"
}"#;

    let message = json_from_str::<DataMessage<Bar, Quote, DetailedTrade>>(json).unwrap();
    let trade = match &message {
      DataMessage::Trade(trade) => trade,
      _ => panic!("Decoded unexpected message variant: {message:?}"),
    };
    assert_eq!(trade.symbol, "AAPL");
    assert_eq!(trade.trade_id, 96921);
    assert_eq!(trade.trade_price, Num::new(12655, 100));
    assert_eq!(trade.trade_size, 1);

    assert_eq!(
      trade.timestamp,
      DateTime::<Utc>::from_str("2021-02-22T15:51:44.208Z").unwrap()
    );

    assert_eq!(trade.conditions, vec!["@", "I"]);
    assert_eq!(trade.tape, "C");
    assert_eq!(trade.update, Some("corrected".to_string()));

    assert_eq!(
      json_from_str::<DataMessage<Bar, Quote, DetailedTrade>>(&to_json(&message).unwrap()).unwrap(),
      message
    );
  }

  /// Check that we can serialize and deserialize the
  /// [`DataMessage::Success`] variant.
  #[test]
  fn serialize_deserialize_success() {
    let json = r#"{"T":"success","msg":"authenticated"}"#;
    let message = json_from_str::<DataMessage>(json).unwrap();
    let () = match message {
      DataMessage::Success => (),
      _ => panic!("Decoded unexpected message variant: {message:?}"),
    };

    assert_eq!(
      json_from_str::<DataMessage>(&to_json(&message).unwrap()).unwrap(),
      message
    );
  }

  /// Check that we can serialize and deserialize the
  /// [`DataMessage::Error`] variant.
  #[test]
  fn serialize_deserialize_error() {
    let json = r#"{"T":"error","code":400,"msg":"invalid syntax"}"#;
    let message = json_from_str::<DataMessage>(json).unwrap();
    let error = match &message {
      DataMessage::Error(error) => error,
      _ => panic!("Decoded unexpected message variant: {message:?}"),
    };

    assert_eq!(error.code, 400);
    assert_eq!(error.message, "invalid syntax");

    assert_eq!(
      json_from_str::<DataMessage>(&to_json(&message).unwrap()).unwrap(),
      message
    );

    let json = r#"{"T":"error","code":500,"msg":"internal error"}"#;
    let message = json_from_str::<DataMessage>(json).unwrap();
    let error = match &message {
      DataMessage::Error(error) => error,
      _ => panic!("Decoded unexpected message variant: {message:?}"),
    };

    assert_eq!(error.code, 500);
    assert_eq!(error.message, "internal error");

    assert_eq!(
      json_from_str::<DataMessage>(&to_json(&message).unwrap()).unwrap(),
      message
    );
  }

  /// Check that we can serialize and deserialize the
  /// [`Request::Authenticate`] variant properly.
  #[test]
  fn serialize_deserialize_authentication_request() {
    let request = Request::Authenticate {
      key_id: "KEY-ID".into(),
      secret: "SECRET-KEY".into(),
    };
    let json = to_json(&request).unwrap();
    let expected = r#"{"action":"auth","key":"KEY-ID","secret":"SECRET-KEY"}"#;
    assert_eq!(json, expected);
    assert_eq!(json_from_str::<Request<'_>>(&json).unwrap(), request);
  }

  /// Check that we can serialize and deserialize the
  /// [`Request::Subscribe`] variant properly.
  #[test]
  fn serialize_deserialize_subscribe_request() {
    let mut data = MarketData::default();
    data.set_bars(["AAPL", "VOO"]);
    let request = Request::Subscribe(Cow::Borrowed(&data));

    let json = to_json(&request).unwrap();
    let expected = r#"{"action":"subscribe","bars":["AAPL","VOO"],"quotes":[],"trades":[]}"#;
    assert_eq!(json, expected);
    assert_eq!(json_from_str::<Request<'_>>(&json).unwrap(), request);
  }

  /// Check that we can serialize and deserialize the
  /// [`Request::Subscribe`] variant properly.
  #[test]
  fn serialize_deserialize_unsubscribe_request() {
    let mut data = MarketData::default();
    data.set_bars(["VOO"]);
    let request = Request::Unsubscribe(Cow::Borrowed(&data));

    let json = to_json(&request).unwrap();
    let expected = r#"{"action":"unsubscribe","bars":["VOO"],"quotes":[],"trades":[]}"#;
    assert_eq!(json, expected);
    assert_eq!(json_from_str::<Request<'_>>(&json).unwrap(), request);
  }

  /// Check that we can correctly deserialize a `SymbolList` object.
  #[test]
  fn deserialize_symbol_list() {
    let json = r#"["AAPL","XLK","SPY"]"#;
    let list = json_from_str::<SymbolList>(json).unwrap();
    let expected = SymbolList::from(["AAPL", "SPY", "XLK"]);
    assert_eq!(list, expected);
  }

  /// Check that we can normalize `Symbol` slices.
  #[test]
  fn normalize_subscriptions() {
    let subscriptions = [];
    assert!(is_normalized(&subscriptions));

    let subscriptions = ["MSFT".into(), "SPY".into()];
    assert!(is_normalized(&subscriptions));

    let mut subscriptions = Cow::from(vec!["SPY".into(), "MSFT".into()]);
    assert!(!is_normalized(&subscriptions));
    subscriptions = normalize(subscriptions);
    assert!(is_normalized(&subscriptions));

    let expected = [Cow::from("MSFT"), "SPY".into()];
    assert_eq!(subscriptions.borrow(), expected);

    let mut subscriptions = Cow::from(vec!["SPY".into(), "MSFT".into(), "MSFT".into()]);
    assert!(!is_normalized(&subscriptions));
    subscriptions = normalize(subscriptions);
    assert!(is_normalized(&subscriptions));

    assert_eq!(subscriptions.borrow(), expected);
  }

  /// Check that we can correctly handle a successful subscription
  /// without pushing actual data.
  #[test(tokio::test)]
  async fn authenticate_and_subscribe() {
    async fn test(mut stream: WebSocketStream) -> Result<(), WebSocketError> {
      stream.send(Message::Text(CONN_RESP.to_string())).await?;
      // Authentication.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(AUTH_REQ.to_string()),
      );
      stream.send(Message::Text(AUTH_RESP.to_string())).await?;

      // Subscription.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(SUB_REQ.to_string()),
      );
      stream.send(Message::Text(SUB_RESP.to_string())).await?;
      stream.send(Message::Close(None)).await?;
      Ok(())
    }

    let (mut stream, mut subscription) =
      mock_stream::<RealtimeData<IEX>, _, _>(test).await.unwrap();

    let mut data = MarketData::default();
    data.set_bars(["AAPL", "VOO"]);

    let subscribe = subscription.subscribe(&data).boxed_local();
    let () = drive(subscribe, &mut stream)
      .await
      .unwrap()
      .unwrap()
      .unwrap();

    stream
      .map_err(Error::WebSocket)
      .try_for_each(|result| async { result.map(|_data| ()).map_err(Error::Json) })
      .await
      .unwrap();
  }

  /// Check that we correctly handle errors reported as part of
  /// subscription.
  #[test(tokio::test)]
  async fn subscribe_error() {
    async fn test(mut stream: WebSocketStream) -> Result<(), WebSocketError> {
      stream.send(Message::Text(CONN_RESP.to_string())).await?;
      // Authentication.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(AUTH_REQ.to_string()),
      );
      stream.send(Message::Text(AUTH_RESP.to_string())).await?;

      // Subscription.
      assert_eq!(
        stream.next().await.unwrap()?,
        Message::Text(SUB_ERR_REQ.to_string()),
      );
      stream.send(Message::Text(SUB_ERR_RESP.to_string())).await?;
      stream.send(Message::Close(None)).await?;
      Ok(())
    }

    let (mut stream, mut subscription) =
      mock_stream::<RealtimeData<IEX>, _, _>(test).await.unwrap();

    let data = MarketData::default();

    let subscribe = subscription.subscribe(&data).boxed_local();
    let error = drive(subscribe, &mut stream)
      .await
      .unwrap()
      .unwrap()
      .unwrap_err();

    match error {
      Error::Str(ref e) if e == "failed to subscribe: invalid syntax (400)" => {},
      e => panic!("received unexpected error: {e}"),
    }
  }

  /// Check that we can adjust the current market data subscription on
  /// the fly.
  #[test(tokio::test)]
  #[serial(realtime_data)]
  async fn subscribe_resubscribe() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let (mut stream, mut subscription) = client.subscribe::<RealtimeData<IEX>>().await.unwrap();

    let mut data = MarketData::default();
    data.set_bars(["AAPL", "SPY"]);

    let subscribe = subscription.subscribe(&data).boxed_local();
    let () = drive(subscribe, &mut stream)
      .await
      .unwrap()
      .unwrap()
      .unwrap();

    assert_eq!(subscription.subscriptions(), &data);

    let mut data = MarketData::default();
    data.set_bars(["XLK"]);
    let subscribe = subscription.subscribe(&data).boxed_local();
    let () = drive(subscribe, &mut stream)
      .await
      .unwrap()
      .unwrap()
      .unwrap();

    let mut expected = MarketData::default();
    expected.set_bars(["AAPL", "SPY", "XLK"]);
    assert_eq!(subscription.subscriptions(), &expected);
  }

  /// Check that we can stream realtime market data updates.
  ///
  /// Note that we do not have any control over whether the market is
  /// open or not and as such we can only try on a best-effort basis to
  /// receive and decode updates.
  #[test(tokio::test)]
  #[serial(realtime_data)]
  async fn stream_market_data_updates() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let (mut stream, mut subscription) = client.subscribe::<RealtimeData<IEX>>().await.unwrap();

    let data = MarketData {
      bars: Symbols::All,
      ..Default::default()
    };

    let subscribe = subscription.subscribe(&data).boxed_local();
    let () = drive(subscribe, &mut stream)
      .await
      .unwrap()
      .unwrap()
      .unwrap();

    assert_eq!(subscription.subscriptions(), &data);

    let read = stream
      .map_err(Error::WebSocket)
      .try_for_each(|result| async {
        result
          .map(|data| {
            assert!(data.is_bar());
          })
          .map_err(Error::Json)
      });

    if timeout(Duration::from_millis(100), read).await.is_ok() {
      panic!("realtime data stream got exhausted unexpectedly")
    }
  }

  /// Check that we can stream realtime stock quotes.
  ///
  /// Note that we do not have any control over whether the market is
  /// open or not and as such we can only try on a best-effort basis to
  /// receive and decode updates.
  #[test(tokio::test)]
  #[serial(realtime_data)]
  async fn stream_quotes() {
    async fn test<S>()
    where
      S: Source,
    {
      let api_info = ApiInfo::from_env().unwrap();
      let client = Client::new(api_info);
      let (mut stream, mut subscription) = client.subscribe::<RealtimeData<S>>().await.unwrap();

      let mut data = MarketData::default();
      data.set_quotes(["SPY"]);

      let subscribe = subscription.subscribe(&data).boxed_local();
      let () = drive(subscribe, &mut stream)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

      let read = stream
        .map_err(Error::WebSocket)
        .try_for_each(|result| async {
          result
            .map(|data| {
              assert!(data.is_quote());
            })
            .map_err(Error::Json)
        });

      if timeout(Duration::from_millis(100), read).await.is_ok() {
        panic!("realtime data stream got exhausted unexpectedly")
      }
    }

    test::<IEX>().await;

    #[derive(Default)]
    struct IexWithUrl;

    impl ToString for IexWithUrl {
      fn to_string(&self) -> String {
        "wss://stream.data.alpaca.markets/v2/iex".into()
      }
    }

    test::<CustomUrl<IexWithUrl>>().await;
  }

  /// Check that we can stream realtime stock trades.
  #[test(tokio::test)]
  #[serial(realtime_data)]
  async fn stream_trades() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let (mut stream, mut subscription) = client.subscribe::<RealtimeData<IEX>>().await.unwrap();

    let mut data = MarketData::default();
    data.set_trades(["SPY"]);

    let subscribe = subscription.subscribe(&data).boxed_local();
    let () = drive(subscribe, &mut stream)
      .await
      .unwrap()
      .unwrap()
      .unwrap();

    let read = stream
      .map_err(Error::WebSocket)
      .try_for_each(|result| async {
        result
          .map(|data| {
            assert!(data.is_trade());
          })
          .map_err(Error::Json)
      });

    if timeout(Duration::from_millis(100), read).await.is_ok() {
      panic!("realtime data stream got exhausted unexpectedly")
    }
  }

  /// Check that the Alpaca API reports no error when unsubscribing
  /// from a symbol not currently subscribed to.
  #[test(tokio::test)]
  #[serial(realtime_data)]
  async fn unsubscribe_not_subscribed_symbol() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let (mut stream, mut subscription) = client.subscribe::<RealtimeData<IEX>>().await.unwrap();

    let mut data = MarketData::default();
    data.set_bars(["AAPL"]);

    let unsubscribe = subscription.unsubscribe(&data).boxed_local();
    let () = drive(unsubscribe, &mut stream)
      .await
      .unwrap()
      .unwrap()
      .unwrap();

    assert_eq!(subscription.subscriptions(), &MarketData::default());
  }

  /// Test that we fail as expected when attempting to authenticate for
  /// real time market updates using invalid credentials.
  #[test(tokio::test)]
  #[serial(realtime_data)]
  async fn stream_with_invalid_credentials() {
    let api_info = ApiInfo::from_parts(API_BASE_URL, "invalid", "invalid-too").unwrap();
    let client = Client::new(api_info);
    let err = client.subscribe::<RealtimeData<IEX>>().await.unwrap_err();

    match err {
      Error::Str(ref e) if e.starts_with("failed to authenticate with server") => (),
      e => panic!("received unexpected error: {e}"),
    }
  }

  /// Check that we fail connection as expected on an invalid URL.
  #[test(tokio::test)]
  #[serial(realtime_data)]
  async fn stream_with_invalid_url() {
    #[derive(Default)]
    struct Invalid;

    impl ToString for Invalid {
      fn to_string(&self) -> String {
        "<invalid-url-is-invalid>".into()
      }
    }

    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let err = client
      .subscribe::<RealtimeData<CustomUrl<Invalid>>>()
      .await
      .unwrap_err();

    match err {
      Error::Url(..) => (),
      _ => panic!("Received unexpected error: {err:?}"),
    };
  }
}
