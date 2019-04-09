// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error as StdError;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use hyper::Error as HyperError;
use hyper::http::Error as HttpError;
use hyper_tls::Error as TlsError;
use serde_json::Error as JsonError;
use url::ParseError;

use crate::Str;


fn fmt_err(err: &dyn StdError, fmt: &mut Formatter<'_>) -> FmtResult {
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
  /// An error reported by the `hyper` crate.
  Hyper(HyperError),
  /// A JSON conversion error.
  Json(JsonError),
  /// An error directly originating in this module.
  Str(Str),
  /// A TLS related error.
  Tls(TlsError),
  /// An URL parsing error.
  Url(ParseError),
}

impl Display for Error {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    match self {
      Error::Http(err) => fmt_err(err, fmt),
      Error::Hyper(err) => fmt_err(err, fmt),
      Error::Json(err) => fmt_err(err, fmt),
      Error::Str(err) => fmt.write_str(err),
      Error::Tls(err) => fmt_err(err, fmt),
      Error::Url(err) => fmt_err(err, fmt),
    }
  }
}

impl From<HttpError> for Error {
  fn from(e: HttpError) -> Self {
    Error::Http(e)
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

impl From<TlsError> for Error {
  fn from(e: TlsError) -> Self {
    Error::Tls(e)
  }
}

impl From<ParseError> for Error {
  fn from(e: ParseError) -> Self {
    Error::Url(e)
  }
}
