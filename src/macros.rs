// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

/// A macro used for defining the properties for a request to a
/// particular HTTP endpoint.
macro_rules! EndpointDef {
  ( $name:ident,
    Ok => $out:ty, $ok:ident, [$($ok_status:ident,)*],
    Err => $err:ident, [$($err_status:ident => $variant:ident,)*] ) => {

    EndpointDefImpl! {
      $name,
      Ok => $out, $ok, [$($ok_status,)*],
      Err => $err, [
        // Every request can fall prey to the rate limit and so we
        // include this variant into all our error definitions.
        /* 429 */ TOO_MANY_REQUESTS => RateLimitExceeded,
        $($err_status => $variant,)*
      ]
    }
  };
}

macro_rules! EndpointDefImpl {
  ( $name:ident,
    Ok => $out:ty, $ok:ident, [$($ok_status:ident,)*],
    Err => $err:ident, [$($err_status:ident => $variant:ident,)*] ) => {

    /// A thin wrapper around the output value.
    #[derive(Clone, Debug, ::serde::Deserialize, PartialEq)]
    #[allow(missing_copy_implementations)]
    pub struct $ok($out);

    #[allow(unused_qualifications)]
    impl ::std::ops::Deref for $ok {
      type Target = $out;

      fn deref(&self) -> &Self::Target {
        &self.0
      }
    }

    #[allow(unused)]
    #[allow(unused_qualifications)]
    impl ::std::convert::From<(::hyper::http::StatusCode, ::std::vec::Vec<u8>)>
      for crate::requestor::ConvertResult<$ok, $err> {

      #[allow(unused)]
      fn from(data: (::hyper::http::StatusCode, ::std::vec::Vec<u8>)) -> Self {
        let (status, body) = data;
        match status {
          $(
            ::hyper::http::StatusCode::$ok_status => {
              match $name::parse(&body) {
                Ok(obj) => crate::requestor::ConvertResult(Ok(obj)),
                Err(err) => crate::requestor::ConvertResult(Err(err)),
              }
            },
          )*
          $(
            ::hyper::http::StatusCode::$err_status => {
              crate::requestor::ConvertResult(Err($err::$variant))
            },
          )*
          _ => crate::requestor::ConvertResult(Err($err::UnexpectedStatus(status))),
        }
      }
    }

    #[derive(Debug)]
    #[allow(missing_docs)]
    pub enum $err {
      $(
        $variant,
      )*
      /// An HTTP status not present in the endpoint's definition was
      /// encountered.
      UnexpectedStatus(::hyper::http::StatusCode),
      /// An error reported by the `hyper` crate.
      Hyper(::hyper::Error),
      /// A JSON conversion error.
      Json(::serde_json::Error),
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
            $err::$variant => {
              crate::Error::HttpStatus(::hyper::http::StatusCode::$err_status)
            },
          )*
          $err::UnexpectedStatus(status) => crate::Error::HttpStatus(status),
          $err::Hyper(err) => crate::Error::Hyper(err),
          $err::Json(err) => crate::Error::Json(err),
        }
      }
    }
  };
}
