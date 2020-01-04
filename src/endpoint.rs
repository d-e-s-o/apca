// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use hyper::Body;
use hyper::Error as HyperError;
use hyper::http::Error as HttpError;
use hyper::http::StatusCode;
use hyper::Method;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Error as JsonError;
use serde_json::from_slice;

use crate::Str;


#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ErrorMessage {
  /// An error code as provided by Alpaca.
  #[serde(rename = "code")]
  pub code: u64,
  /// A message as provided by Alpaca.
  #[serde(rename = "message")]
  pub message: String,
}

impl Display for ErrorMessage {
  fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
    write!(fmt, "{} ({})", self.message, self.code)
  }
}


/// A trait describing an HTTP endpoint.
///
/// An endpoint for our intents and purposes is basically a path and an
/// HTTP request method (e.g., GET or POST). The path will be combined
/// with an "authority" (scheme, host, and port) into a full URL. Query
/// parameters are supported as well.
/// An endpoint is used by the `Trader` who invokes the various methods.
pub trait Endpoint {
  /// The type of data being passed in as part of a request to this
  /// endpoint.
  type Input;
  /// The type of data being returned in the response from this
  /// endpoint.
  type Output: DeserializeOwned;
  /// The type of error this endpoint can report.
  type Error: From<HttpError> + From<HyperError> + From<JsonError>;
  /// An error emitted by the API.
  type ApiError: DeserializeOwned;

  /// Retrieve the HTTP method to use.
  ///
  /// The default method being used is GET.
  fn method() -> Method {
    Method::GET
  }

  /// Inquire the path the request should go to.
  fn path(input: &Self::Input) -> Str;

  /// Inquire the query the request should use.
  ///
  /// By default no query is emitted.
  #[allow(unused)]
  fn query(input: &Self::Input) -> Option<Str> {
    None
  }

  /// Retrieve the request's body.
  ///
  /// By default this method creates an empty body.
  #[allow(unused)]
  fn body(input: &Self::Input) -> Result<Body, JsonError> {
    Ok(Body::empty())
  }

  /// Parse the body into the final result.
  ///
  /// By default this method directly parses the body as JSON.
  fn parse(body: &[u8]) -> Result<Self::Output, Self::Error> {
    from_slice::<Self::Output>(body).map_err(Self::Error::from)
  }

  /// Parse an API error.
  fn parse_err(body: &[u8]) -> Result<Self::ApiError, Vec<u8>> {
    from_slice::<Self::ApiError>(body).map_err(|_| body.to_vec())
  }

  /// Evaluate an HTTP status and body, converting it into an output or
  /// error, depending on the status.
  ///
  /// This method is not meant to be implemented manually. It will be
  /// auto-generated.
  fn evaluate(status: StatusCode, body: &[u8]) -> Result<Self::Output, Self::Error>;
}


/// A macro used for defining the properties for a request to a
/// particular HTTP endpoint.
macro_rules! EndpointDef {
  ( $(#[$docs:meta])* $name:ident($in:ty),
    Ok => $out:ty, [$($(#[$ok_docs:meta])* $ok_status:ident,)*],
    Err => $err:ident, [$($(#[$err_docs:meta])* $err_status:ident => $variant:ident,)*]
    $($defs:tt)* ) => {

    EndpointDefImpl! {
      $(#[$docs])* $name($in),
      Ok => $out, [$($ok_status,)*],
      Err => $err, [
        // Every request can result in an authentication failure or fall
        // prey to the rate limit and so we include these variants into
        // all our error definitions.
        /// Authentication failed for the request.
        /* 401 */ UNAUTHORIZED => AuthenticationFailed,
        /// The rate limit was exceeded, causing the request to be
        /// denied.
        /* 429 */ TOO_MANY_REQUESTS => RateLimitExceeded,
        $($(#[$err_docs])* $err_status => $variant,)*
      ],
      ApiErr => crate::endpoint::ErrorMessage,
      $($defs)*
    }
  };
}

macro_rules! EndpointDefImpl {
  ( $(#[$docs:meta])* $name:ident($in:ty),
    // We just ignore any documentation for success cases: there is
    // nowhere we can put it.
    Ok => $out:ty, [$($(#[$ok_docs:meta])* $ok_status:ident,)*],
    Err => $err:ident, [$($(#[$err_docs:meta])* $err_status:ident => $variant:ident,)*],
    ApiErr => $api_err:ty,
    $($defs:tt)* ) => {

    $(#[$docs])*
    #[derive(Clone, Copy, Debug)]
    pub struct $name;

    /// An enum representing the various errors this endpoint may
    /// encounter.
    #[allow(unused_qualifications)]
    #[derive(Debug)]
    pub enum $err {
      $(
        $(#[$err_docs])*
        $variant(Result<$api_err, Vec<u8>>),
      )*
      /// An HTTP status not present in the endpoint's definition was
      /// encountered.
      UnexpectedStatus(::hyper::http::StatusCode, Result<$api_err, Vec<u8>>),
      /// An HTTP related error.
      Http(::hyper::http::Error),
      /// An error reported by the `hyper` crate.
      Hyper(::hyper::Error),
      /// A JSON conversion error.
      Json(::serde_json::Error),
    }

    #[allow(unused_qualifications)]
    impl ::std::fmt::Display for $err {
      fn fmt(&self, fmt: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        fn format_message(message: &Result<$api_err, Vec<u8>>) -> String {
          match message {
            Ok(err) => err.to_string(),
            Err(body) => {
              match std::str::from_utf8(&body) {
                Ok(body) => format!("{}", body),
                Err(err) => format!("{}", err),
              }
            },
          }
        }

        match self {
          $(
            $err::$variant(message) => {
              let status = ::hyper::http::StatusCode::$err_status;
              let message = format_message(message);
              write!(fmt, "HTTP status {}: {}", status, message)
            },
          )*
          $err::UnexpectedStatus(status, message) => {
            let message = format_message(message);
            write!(fmt, "Unexpected HTTP status {}: {}", status, message)
          },
          $err::Http(err) => write!(fmt, "{}", err),
          $err::Hyper(err) => write!(fmt, "{}", err),
          $err::Json(err) => write!(fmt, "{}", err),
        }
      }
    }

    #[allow(unused_qualifications)]
    impl ::std::error::Error for $err {
      fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {
        match self {
          $(
            $err::$variant(..) => None,
          )*
          $err::UnexpectedStatus(..) => None,
          $err::Http(err) => err.source(),
          $err::Hyper(err) => err.source(),
          $err::Json(err) => err.source(),
        }
      }
    }

    #[allow(unused_qualifications)]
    impl ::std::convert::From<::hyper::http::Error> for $err {
      fn from(src: ::hyper::http::Error) -> Self {
        $err::Http(src)
      }
    }

    #[allow(unused_qualifications)]
    impl ::std::convert::From<::hyper::Error> for $err {
      fn from(src: ::hyper::Error) -> Self {
        $err::Hyper(src)
      }
    }

    #[allow(unused_qualifications)]
    impl ::std::convert::From<::serde_json::Error> for $err {
      fn from(src: ::serde_json::Error) -> Self {
        $err::Json(src)
      }
    }

    #[allow(unused_qualifications)]
    impl ::std::convert::From<$err> for crate::Error {
      fn from(src: $err) -> Self {
        match src {
          $(
            $err::$variant(_) => {
              crate::Error::HttpStatus(::hyper::http::StatusCode::$err_status)
            },
          )*
          $err::UnexpectedStatus(status, _) => crate::Error::HttpStatus(status),
          $err::Http(err) => crate::Error::Http(err),
          $err::Hyper(err) => crate::Error::Hyper(err),
          $err::Json(err) => crate::Error::Json(err),
        }
      }
    }

    #[allow(unused_qualifications)]
    impl crate::endpoint::Endpoint for $name {
      type Input = $in;
      type Output = $out;
      type Error = $err;
      type ApiError = $api_err;

      $($defs)*

      #[allow(unused_qualifications)]
      fn evaluate(
        status: ::hyper::http::StatusCode,
        body: &[u8],
      ) -> Result<$out, $err> {
        match status {
          $(
            ::hyper::http::StatusCode::$ok_status => {
              <$name as crate::endpoint::Endpoint>::parse(&body)
            },
          )*
          status => {
            let res = <$name as crate::endpoint::Endpoint>::parse_err(&body);
            match status {
              $(
                ::hyper::http::StatusCode::$err_status => {
                  Err($err::$variant(res))
                },
              )*
              _ => Err($err::UnexpectedStatus(status, res)),
            }
          },
        }
      }
    }
  };
}
