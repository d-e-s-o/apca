// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error as StdError;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use hyper::Error as HyperError;
use hyper::http::Error as HttpError;
use hyper::http::StatusCode as HttpStatusCode;
use serde_json::Error as JsonError;
use tungstenite::tungstenite::Error as WebSocketError;
use url::ParseError;

use crate::endpoint::EndpointError;
use crate::Str;


pub fn fmt_err(err: &dyn StdError, fmt: &mut Formatter<'_>) -> FmtResult {
  write!(fmt, "{}", err)?;
  if let Some(src) = err.source() {
    write!(fmt, ": ")?;
    fmt_err(src, fmt)?;
  }
  Ok(())
}


/// The error type as used by this crate.
#[derive(Debug)]
pub enum Error {
  /// An HTTP related error.
  Http(HttpError),
  /// We encountered an HTTP that either represents a failure or is not
  /// supported.
  HttpStatus(HttpStatusCode),
  /// An error reported by the `hyper` crate.
  Hyper(HyperError),
  /// A JSON conversion error.
  Json(JsonError),
  /// An error directly originating in this module.
  Str(Str),
  /// An URL parsing error.
  Url(ParseError),
  /// A websocket error.
  WebSocket(WebSocketError),
}

impl Display for Error {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      Error::Http(err) => fmt_err(err, fmt),
      Error::HttpStatus(status) => write!(fmt, "Received HTTP status: {}", status),
      Error::Hyper(err) => fmt_err(err, fmt),
      Error::Json(err) => fmt_err(err, fmt),
      Error::Str(err) => fmt.write_str(err),
      Error::Url(err) => fmt_err(err, fmt),
      Error::WebSocket(err) => fmt_err(err, fmt),
    }
  }
}

impl StdError for Error {}

impl From<EndpointError> for Error {
  fn from(e: EndpointError) -> Self {
    match e {
      EndpointError::Http(e) => Error::Http(e),
      EndpointError::Json(e) => Error::Json(e),
    }
  }
}

impl From<HttpError> for Error {
  fn from(e: HttpError) -> Self {
    Error::Http(e)
  }
}

impl From<HttpStatusCode> for Error {
  fn from(e: HttpStatusCode) -> Self {
    Error::HttpStatus(e)
  }
}

impl From<HyperError> for Error {
  fn from(e: HyperError) -> Self {
    Error::Hyper(e)
  }
}

impl From<JsonError> for Error {
  fn from(e: JsonError) -> Self {
    Error::Json(e)
  }
}

impl From<ParseError> for Error {
  fn from(e: ParseError) -> Self {
    Error::Url(e)
  }
}

impl From<WebSocketError> for Error {
  fn from(e: WebSocketError) -> Self {
    Error::WebSocket(e)
  }
}
