// Copyright (C) 2019-2020 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::str;

use futures::stream::Stream;

use hyper::Body;
use hyper::body::to_bytes;
use hyper::Client as HttpClient;
use hyper::client::Builder as HttpClientBuilder;
use hyper::client::HttpConnector;
use hyper::http::request::Builder as HttpRequestBuilder;
use hyper::http::StatusCode;
use hyper::Request;
use hyper_tls::HttpsConnector;

use log::debug;
use log::info;
use log::Level::Debug;
use log::log_enabled;

use serde_json::Error as JsonError;

use tungstenite::tungstenite::Error as WebSocketError;

use crate::api::HDR_KEY_ID;
use crate::api::HDR_SECRET;
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
}

impl Builder {
  /// Adjust the maximum number of idle connections per host.
  pub fn max_idle_per_host(&mut self, max_idle: usize) -> &mut Self {
    let _ = self.builder.max_idle_per_host(max_idle);
    self
  }

  /// Build the final `Client` object.
  pub fn build(&self, api_info: ApiInfo) -> Client {
    let https = HttpsConnector::new();
    let client = self.builder.build(https);

    Client { api_info, client }
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
    }
  }

  #[cfg(not(test))]
  fn default() -> Self {
    Self {
      builder: HttpClient::builder(),
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
  pub fn new(api_info: ApiInfo) -> Self {
    Builder::default().build(api_info)
  }

  /// Create a `Request` to the endpoint.
  ///
  /// Typically the default implementation is just fine.
  fn request<R>(&self, input: &R::Input) -> Result<Request<Body>, R::Error>
  where
    R: Endpoint,
  {
    let mut url = self.api_info.base_url.clone();
    url.set_path(&R::path(&input));
    url.set_query(R::query(&input).as_ref().map(AsRef::as_ref));

    let request = HttpRequestBuilder::new()
      .method(R::method())
      .uri(url.as_str())
      // Add required authentication information.
      .header(HDR_KEY_ID, self.api_info.key_id.as_str())
      .header(HDR_SECRET, self.api_info.secret.as_str())
      .body(R::body(input)?)?;

    Ok(request)
  }

  /// Create and issue a request and decode the response.
  pub async fn issue<R>(&self, input: R::Input) -> Result<R::Output, R::Error>
  where
    R: Endpoint,
    ConvertResult<R::Output, R::Error>: From<(StatusCode, Vec<u8>)>,
  {
    let req = self.request::<R>(&input)?;
    if log_enabled!(Debug) {
      debug!("HTTP request: {:?}", req);
    } else {
      info!("HTTP request: {} to {}", req.method(), req.uri());
    }

    let result = self.client.request(req).await?;
    let status = result.status();
    // We unconditionally wait for the full body to be received
    // before even evaluating the header. That is mostly done for
    // simplicity and it shouldn't really matter anyway because most
    // if not all requests evaluate the body on success and on error
    // the server shouldn't send back much.
    // TODO: However, there may be one case that has the potential
    //       to cause trouble: when we receive, for example, the
    //       list of all orders it now needs to be stored in memory
    //       in its entirety. That may blow things.
    let bytes = to_bytes(result.into_body()).await?;
    let body = Vec::from(bytes.as_ref());

    info!("HTTP status: {}", status);
    if log_enabled!(Debug) {
      match str::from_utf8(&body) {
        Ok(s) => debug!("HTTP body: {}", s),
        Err(b) => debug!("HTTP body: {}", b),
      }
    }

    ConvertResult::<R::Output, R::Error>::from((status, body)).into()
  }

  /// Subscribe to the given stream in order to receive updates.
  pub async fn subscribe<S>(
    &self,
  ) -> Result<impl Stream<Item = Result<Result<S::Event, JsonError>, WebSocketError>>, Error>
  where
    S: EventStream,
  {
    let mut url = self.api_info.base_url.clone();
    url.set_scheme("wss").map_err(|()| {
      Error::Str(format!("unable to change URL scheme for {}: invalid URL?", url).into())
    })?;
    url.set_path("stream");

    let api_info = ApiInfo {
      base_url: url,
      key_id: self.api_info.key_id.clone(),
      secret: self.api_info.secret.clone(),
    };

    stream::<S>(api_info).await
  }

  /// Retrieve the `ApiInfo` object used by this `Client` instance.
  pub fn api_info(&self) -> &ApiInfo {
    &self.api_info
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use test_env_log::test;

  use crate::endpoint::ErrorMessage;
  use crate::Str;


  #[derive(Debug)]
  pub struct GetNotFound {}

  EndpointDef! {
    GetNotFound(()),
    Ok => (), [],
    Err => GetNotFoundError, []

    fn path(_input: &Self::Input) -> Str {
      "/v2/foobarbaz".into()
    }
  }

  #[test(tokio::test)]
  async fn unexpected_status_code_return() -> Result<(), Error> {
    let api_info = ApiInfo::from_env()?;
    let client = Client::builder().max_idle_per_host(0).build(api_info);
    let result = client.issue::<GetNotFound>(()).await;
    let err = result.unwrap_err();

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
