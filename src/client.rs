// Copyright (C) 2019 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::str::from_utf8;

use futures01::future::Future;
use futures01::stream::Stream;

use hyper::Body;
use hyper::Client as HttpClient;
use hyper::client::Builder as HttpClientBuilder;
use hyper::client::HttpConnector;
use hyper::http::StatusCode;
use hyper_tls::HttpsConnector;

use log::debug;
use log::info;
use log::Level::Debug;
use log::log_enabled;

use serde_json::Error as JsonError;

use websocket::WebSocketError;

use crate::api_info::ApiInfo;
use crate::endpoint::ConvertResult;
use crate::endpoint::Endpoint;
use crate::Error;
use crate::events::EventStream;
use crate::events::stream;


/// A builder for creating customized `Client` objects.
#[derive(Debug)]
pub struct Builder {
  builder: HttpClientBuilder,
  dns_threads: usize,
}

impl Builder {
  /// Adjust the maximum number of idle connections per host.
  pub fn max_idle_per_host(&mut self, max_idle: usize) -> &mut Self {
    let _ = self.builder.max_idle_per_host(max_idle);
    self
  }

  /// Set the number of DNS worker threads used by the HTTPS connector.
  pub fn dns_worker_threads(&mut self, threads: usize) -> &mut Self {
    self.dns_threads = threads;
    self
  }

  /// Build the final `Client` object.
  pub fn build(&self, api_info: ApiInfo) -> Result<Client, Error> {
    let https = HttpsConnector::new(self.dns_threads)?;
    let client = self.builder.build(https);

    Ok(Client { api_info, client })
  }
}

impl Default for Builder {
  #[cfg(test)]
  fn default() -> Self {
    // So here is the deal. In tests we use the block_on_all function to
    // wait for futures. This function waits until *all* spawned futures
    // completed. Now, by virtue of keeping idle connections around --
    // which effectively map to spawned tasks -- we will block until
    // those connections die. We can't have that happen for tests, so we
    // disable idle connections for them.
    // While at it, also use the minimum number of threads for the
    // `HttpsConnector`.
    let mut builder = HttpClient::builder();
    let _ = builder.max_idle_per_host(0);

    Self {
      builder,
      dns_threads: 1,
    }
  }

  #[cfg(not(test))]
  fn default() -> Self {
    Self {
      builder: HttpClient::builder(),
      dns_threads: 4,
    }
  }
}


/// A `Client` is the entity used by clients of this module for
/// interacting with the Alpaca API. It provides the highest-level
/// primitives and also implements the `Trader` trait, which abstracts
/// away the trading related functionality common among all supported
/// services.
#[derive(Debug)]
pub struct Client {
  api_info: ApiInfo,
  client: HttpClient<HttpsConnector<HttpConnector>, Body>,
}

impl Client {
  /// Instantiate a new `Builder` which allows for creating a customized `Client`.
  pub fn builder() -> Builder {
    Builder::default()
  }

  /// Create a new `Client` using the given key ID and secret for
  /// connecting to the API.
  pub fn new(api_info: ApiInfo) -> Result<Self, Error> {
    Builder::default().build(api_info)
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
    let req = R::request(&self.api_info, &input)?;
    if log_enabled!(Debug) {
      debug!("HTTP request: {:?}", req);
    } else {
      info!("HTTP request: {} to {}", req.method(), req.uri());
    }

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

        info!("HTTP status: {}", status);
        if log_enabled!(Debug) {
          match from_utf8(&body) {
            Ok(s) => debug!("HTTP body: {}", s),
            Err(b) => debug!("HTTP body: {}", b),
          }
        }

        let res = ConvertResult::<R::Output, R::Error>::from((status, body));
        Into::<Result<_, _>>::into(res)
      });

    Ok(Box::new(fut))
  }

  /// Subscribe to the given stream in order to receive updates.
  pub fn subscribe<S>(
    &self,
  ) -> impl Future<
    Item = impl Stream<Item = Result<S::Event, JsonError>, Error = WebSocketError>,
    Error = Error,
  >
  where
    S: EventStream,
  {
    stream::<S>(self.api_info.clone())
  }

  /// Retrieve the `ApiInfo` object used by this `Client` instance.
  pub fn api_info(&self) -> &ApiInfo {
    &self.api_info
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use tokio01::runtime::current_thread::block_on_all;

  use test_env_log::test;

  use crate::endpoint::ErrorMessage;
  use crate::Str;


  #[derive(Debug)]
  pub struct GetNotFound {}

  EndpointDef! {
    GetNotFound,
    Ok => (), [],
    Err => GetNotFoundError, []
  }

  impl Endpoint for GetNotFound {
    type Input = ();
    type Output = ();
    type Error = GetNotFoundError;

    fn path(_input: &Self::Input) -> Str {
      "/v1/foobarbaz".into()
    }
  }


  #[test]
  fn unexpected_status_code_return() -> Result<(), Error> {
    let api_info = ApiInfo::from_env()?;
    let client = Client::builder().max_idle_per_host(0).build(api_info)?;
    let future = client.issue::<GetNotFound>(())?;
    let err = block_on_all(future).unwrap_err();

    match err {
      GetNotFoundError::UnexpectedStatus(status, message) => {
        let expected = ErrorMessage {
          code: 40410000,
          message: "endpoint not found".to_string(),
        };
        assert_eq!(message, Ok(expected));
        assert_eq!(status, StatusCode::NOT_FOUND);
      },
      _ => panic!("Received unexpected error: {:?}", err),
    };
    Ok(())
  }
}
