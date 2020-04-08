// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error as StdError;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use http::Error as HttpError;
use http::StatusCode as HttpStatusCode;
use http_endpoint::Error as EndpointError;
use hyper::Error as HyperError;
use serde_json::Error as JsonError;
use tungstenite::tungstenite::Error as WebSocketError;
use url::ParseError;

use crate::Str;


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
      Error::Http(err) => write!(fmt, "{}", err),
      Error::HttpStatus(status) => write!(fmt, "Received HTTP status: {}", status),
      Error::Hyper(err) => write!(fmt, "{}", err),
      Error::Json(err) => write!(fmt, "{}", err),
      Error::Str(err) => fmt.write_str(err),
      Error::Url(err) => write!(fmt, "{}", err),
      Error::WebSocket(err) => write!(fmt, "{}", err),
    }
  }
}

impl StdError for Error {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    match self {
      Error::Http(err) => err.source(),
      Error::HttpStatus(..) => None,
      Error::Hyper(err) => err.source(),
      Error::Json(err) => err.source(),
      Error::Str(..) => None,
      Error::Url(err) => err.source(),
      Error::WebSocket(err) => err.source(),
    }
  }
}

impl From<EndpointError> for Error {
  fn from(src: EndpointError) -> Self {
    match src {
      EndpointError::Http(err) => Error::Http(err),
      EndpointError::HttpStatus(status) => Error::HttpStatus(status),
      EndpointError::Hyper(err) => Error::Hyper(err),
      EndpointError::Json(err) => Error::Json(err),
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
