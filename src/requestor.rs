// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env::var_os;
use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt;

use futures::future::Future;
use futures::stream::Stream;

use hyper::Body;
use hyper::Client;
use hyper::client::HttpConnector;
use hyper::http::request::Builder;
use hyper::http::StatusCode;
use hyper::Request;
use hyper_tls::HttpsConnector;

use serde::de::DeserializeOwned;
use serde_json::Error as JsonError;
use serde_json::from_slice;

use url::Url;

use crate::api::API_BASE_URL;
use crate::api::HDR_KEY_ID;
use crate::api::HDR_SECRET;
use crate::ENV_API;
use crate::ENV_KEY_ID;
use crate::ENV_SECRET;
use crate::Error;
use crate::Str;


/// A result type used solely for the purpose of communicating
/// the result of a conversion to the `Requestor`.
///
/// This type is pretty much a `Result`, but given that it is local to
/// our crate we can implement non-local traits for it. We exploit this
/// fact in the `Requestor` struct which relies on a From conversion
/// from an (HTTP status, Body)-pair yielding such a result.
#[derive(Debug)]
pub struct ConvertResult<T, E>(pub Result<T, E>);

impl<T, E> Into<Result<T, E>> for ConvertResult<T, E> {
  fn into(self) -> Result<T, E> {
    self.0
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
  type Input;
  type Output;
  type Error;

  /// Inquire the path the request should go to.
  fn path(input: &Self::Input) -> Str;

  /// Inquire the query the request should use.
  ///
  /// By default no query is emitted.
  #[allow(unused)]
  fn query(input: &Self::Input) -> Option<Str> {
    None
  }

  /// Create a request builder.
  ///
  /// By default this method creates a `Builder` for a GET request.
  #[allow(unused)]
  fn builder(url: &str, input: &Self::Input) -> Builder {
    Request::get(url)
  }

  /// Take the previously returned `Builder` and create the final
  /// `Request` out of it.
  ///
  /// By default this method creates a request with an empty body.
  #[allow(unused)]
  fn request(builder: &mut Builder, input: &Self::Input) -> Result<Request<Body>, Error> {
    builder.body(Body::empty()).map_err(Error::from)
  }

  /// Parse the body into the final result.
  ///
  /// By default this method directly parses the body as JSON.
  fn parse(body: &[u8]) -> Result<Self::Output, Self::Error>
  where
    Self::Output: DeserializeOwned,
    Self::Error: From<JsonError>,
  {
    from_slice::<Self::Output>(body).map_err(Self::Error::from)
  }
}


/// A `Requestor` is the entity used by clients of this module for
/// interacting with the Alpaca API. It provides the highest-level
/// primitives and also implements the `Trader` trait, which abstracts
/// away the trading related functionality common among all supported
/// services.
#[derive(Debug)]
pub struct Requestor {
  api_base: Url,
  key_id: Vec<u8>,
  secret: Vec<u8>,
  client: Client<HttpsConnector<HttpConnector>, Body>,
}

impl Requestor {
  /// Create a new `Requestor` using the given key ID and secret for
  /// connecting to the API.
  fn new<I, S>(api_base: Url, key_id: I, secret: S) -> Result<Self, Error>
  where
    I: Into<Vec<u8>>,
    S: Into<Vec<u8>>,
  {
    // So here is the deal. In tests we use the block_on_all function to
    // wait for futures. This function waits until *all* spawned futures
    // completed. Now, by virtue of keeping idle connections around --
    // which effectively map to spawned tasks -- we will block until
    // those connections die. We can't have that happen for tests, so we
    // disable idle connections for them.
    // While at it, also use the minimum number of threads for the
    // `HttpsConnector`.
    #[cfg(test)]
    fn client() -> Result<Client<HttpsConnector<HttpConnector>, Body>, Error> {
      let https = HttpsConnector::new(1)?;
      let client = Client::builder()
        .max_idle_per_host(0)
        .build::<_, Body>(https);
      Ok(client)
    }
    #[cfg(not(test))]
    fn client() -> Result<Client<HttpsConnector<HttpConnector>, Body>, Error> {
      let https = HttpsConnector::new(4)?;
      let client = Client::builder().build::<_, Body>(https);
      Ok(client)
    }

    Ok(Self {
      api_base,
      key_id: key_id.into(),
      secret: secret.into(),
      client: client()?,
    })
  }

  /// Create a new `Requestor` with information from the environment.
  pub fn from_env() -> Result<Self, Error> {
    let api_base = var_os(ENV_API)
      .unwrap_or_else(|| OsString::from(API_BASE_URL))
      .into_string()
      .map_err(|_| {
        Error::Str(format!("{} environment variable is not a valid string", ENV_API).into())
      })?;
    let api_base = Url::parse(&api_base)?;

    let key_id = var_os(ENV_KEY_ID)
      .ok_or_else(|| Error::Str(format!("{} environment variable not found", ENV_KEY_ID).into()))?;
    let secret = var_os(ENV_SECRET)
      .ok_or_else(|| Error::Str(format!("{} environment variable not found", ENV_SECRET).into()))?;

    let key_id = key_id.as_os_str().as_bytes();
    let secret = secret.as_os_str().as_bytes();
    Self::new(api_base, key_id, secret)
  }

  /// Create and issue a request and decode the response.
  pub fn issue<R>(
    &self,
    input: R::Input,
  ) -> Result<impl Future<Item = R::Output, Error = R::Error>, Error>
  where
    R: Endpoint,
    R::Output: Send + 'static,
    R::Error: From<hyper::Error> + Send + 'static,
    ConvertResult<R::Output, R::Error>: From<(StatusCode, Vec<u8>)>,
  {
    let mut url = self.api_base.clone();
    url.set_path(&R::path(&input));
    url.set_query(R::query(&input).as_ref().map(AsRef::as_ref));

    let mut bldr = R::builder(url.as_str(), &input);
    // Add required authentication information.
    let bldr = bldr
      .header(HDR_KEY_ID, self.key_id.as_slice())
      .header(HDR_SECRET, self.secret.as_slice());

    // Now build and issue the actual request.
    let req = R::request(bldr, &input)?;
    let fut = self
      .client
      .request(req)
      .and_then(|res| {
        let status = res.status();
        // We unconditionally wait for the full body to be received
        // before even evaluating the header. That is mostly done for
        // simplicity and it shouldn't really matter anyway because most
        // if not all requests evaluate the body on success and on error
        // the server shouldn't send back much.
        // TODO: However, there may be one case that has the potential
        //       to cause trouble: when we receive, for example, the
        //       list of all orders it now needs to be stored in memory
        //       in its entirety. That may blow things.
        res.into_body().concat2().map(move |body| (status, body))
      })
      .map_err(R::Error::from)
      .and_then(|(status, body)| {
        let bytes = body.into_bytes();
        let body = Vec::from(bytes.as_ref());
        let res = ConvertResult::<R::Output, R::Error>::from((status, body));
        Into::<Result<_, _>>::into(res)
      });

    Ok(Box::new(fut))
  }
}
