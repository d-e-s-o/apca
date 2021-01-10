// Copyright (C) 2019-2021 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error as StdError;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::str::from_utf8;

use http::Error as HttpError;
use http::StatusCode as HttpStatusCode;
use http_endpoint::Error as EndpointError;
use hyper::Error as HyperError;
use serde_json::Error as JsonError;
use url::ParseError;
use websocket_util::tungstenite::Error as WebSocketError;

use crate::Str;


/// An error encountered while issuing a request.
#[derive(Debug)]
pub enum RequestError<E> {
  /// An endpoint reported error.
  Endpoint(E),
  /// An error reported by the `hyper` crate.
  Hyper(HyperError),
}

impl<E> Display for RequestError<E>
where
  E: Display,
{
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      Self::Endpoint(err) => write!(fmt, "{}", err),
      Self::Hyper(err) => write!(fmt, "{}", err),
    }
  }
}

impl<E> StdError for RequestError<E>
where
  E: StdError,
{
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    match self {
      Self::Endpoint(..) => None,
      Self::Hyper(err) => err.source(),
    }
  }
}

impl<E> From<HyperError> for RequestError<E> {
  fn from(e: HyperError) -> Self {
    Self::Hyper(e)
  }
}


/// The error type as used by this crate.
#[derive(Debug)]
pub enum Error {
  /// An HTTP related error.
  Http(HttpError),
  /// We encountered an HTTP that either represents a failure or is not
  /// supported.
  HttpStatus(HttpStatusCode, Vec<u8>),
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
      Error::HttpStatus(status, data) => {
        write!(fmt, "Received HTTP status: {}: ", status)?;
        match from_utf8(&data) {
          Ok(s) => fmt.write_str(s)?,
          Err(b) => write!(fmt, "{:?}", b)?,
        }
        Ok(())
      },
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
      EndpointError::HttpStatus(status, data) => Error::HttpStatus(status, data),
      EndpointError::Json(err) => Error::Json(err),
    }
  }
}

impl From<HttpError> for Error {
  fn from(e: HttpError) -> Self {
    Error::Http(e)
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
