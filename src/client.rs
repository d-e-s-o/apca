// Copyright (C) 2019-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::borrow::Cow;
use std::future::Future;
use std::str::from_utf8;

use http::request::Builder as HttpRequestBuilder;
use http_endpoint::Endpoint;
// HyperBody implements From<Cow<'static, [u8]]>>, whereas reqwest::Body has
// From<'static, [u8]> and From<hyper::Body>. This is used to allow conversion
// from Endpoint::body(input) -> hyper::Body -> reqwest::Body.
use hyper::Body as HyperBody;

use reqwest::Body;
use reqwest::Client as HttpClient;
use reqwest::ClientBuilder as HttpClientBuilder;
use reqwest::Request;

use tracing::debug;
use tracing::instrument;
use tracing::span;
use tracing::trace;
use tracing::Level;
use tracing_futures::Instrument;

use url::Url;

use crate::api::HDR_KEY_ID;
use crate::api::HDR_SECRET;
use crate::api_info::ApiInfo;
use crate::error::RequestError;
use crate::subscribable::Subscribable;
use crate::Error;


/// A builder for creating customized `Client` objects.
#[derive(Debug)]
pub struct Builder {
  builder: HttpClientBuilder,
}

impl Builder {
  /// Adjust the maximum number of idle connections per host.
  #[inline]
  pub fn max_idle_per_host(mut self, max_idle: usize) -> Self {
    self.builder = self.builder.pool_max_idle_per_host(max_idle);
    self
  }

  /// Build the final `Client` object.
  pub fn build(self, api_info: ApiInfo) -> Client {
    debug!("{:?}", self.builder);
    let client = self.builder.build().unwrap();
    debug!("{:?}", client);
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
    let builder = HttpClient::builder().pool_max_idle_per_host(0);
    Self { builder }
  }

  #[cfg(not(test))]
  #[inline]
  fn default() -> Self {
    Self {
      builder: HttpClient::builder(),
    }
  }
}


/// A `Client` is the entity used by clients of this module for
/// interacting with the Alpaca API.
#[derive(Debug)]
pub struct Client {
  api_info: ApiInfo,
  client: HttpClient,
}

impl Client {
  /// Instantiate a new `Builder` which allows for creating a customized `Client`.
  #[inline]
  pub fn builder() -> Builder {
    Builder::default()
  }

  /// Create a new `Client` using the given key ID and secret for
  /// connecting to the API.
  #[inline]
  pub fn new(api_info: ApiInfo) -> Self {
    Builder::default().build(api_info)
  }

  /// Create a `Request` to the endpoint.
  fn request<R>(&self, input: &R::Input) -> Result<Request, R::Error>
  where
    R: Endpoint, <R as Endpoint>::Error: From<crate::endpoint::ConversionError>
  {
    let mut url = R::base_url()
      .map(|url| Url::parse(url.as_ref()).expect(
        "endpoint definition contains invalid URL"))
      .unwrap_or_else(|| self.api_info.api_base_url.clone());

    url.set_path(&R::path(input));
    url.set_query(R::query(input)?.as_ref().map(AsRef::as_ref));

    let request = HttpRequestBuilder::new()
      .method(R::method())
      .uri(url.as_str())
      // Add required authentication information.
      .header(HDR_KEY_ID, self.api_info.key_id.as_str())
      .header(HDR_SECRET, self.api_info.secret.as_str())
      .body(Body::from(HyperBody::from(
        R::body(input)?.unwrap_or_else(|| Cow::Borrowed(&[0; 0])),
      )))?;
    let request = Request::try_from(request).map_err(
      |x| crate::endpoint::ConversionError::from(x))?;

    Ok(request)
  }

  /// Create and issue a request and decode the response.
  pub fn issue<R>(
    &self,
    input: &R::Input,
  ) -> impl Future<Output = Result<R::Output, RequestError<R::Error>>> + '_
  where
    R: Endpoint, <R as Endpoint>::Error: From<crate::endpoint::ConversionError>
  {
    let result = self.request::<R>(input);
    async move {
      let request = result.map_err(RequestError::Endpoint)?;
      let span = span!(
        Level::INFO,
        "issue",
        method = display(request.method()),
        uri = display(request.url())
      );
      self.issue_::<R>(request).instrument(span).await
    }
  }

  /// Issue a request.
  #[allow(clippy::cognitive_complexity)]
  async fn issue_<R>(&self, request: Request) -> Result<R::Output, RequestError<R::Error>>
  where
    R: Endpoint,
  {
    debug!("requesting");
    trace!(body = debug(request.body()));

    let result = self.client.execute(request).await?;
    let status = result.status();
    debug!(status = debug(&status));
    trace!(response = debug(&result));

    // We unconditionally wait for the full body to be received
    // before even evaluating the header. That is mostly done for
    // simplicity and it shouldn't really matter anyway because most
    // if not all requests evaluate the body on success and on error
    // the server shouldn't send back much.
    // TODO: However, there may be one case that has the potential
    //       to cause trouble: when we receive, for example, the
    //       list of all orders it now needs to be stored in memory
    //       in its entirety. That may blow things.
    let bytes = result.bytes().await?;
    let body = bytes.as_ref();

    match from_utf8(body) {
      Ok(s) => trace!(body = display(&s)),
      Err(b) => trace!(body = display(&b)),
    }

    R::evaluate(status, body).map_err(RequestError::Endpoint)
  }

  /// Subscribe to the given subscribable in order to receive updates.
  ///
  /// # Notes
  /// - this method is only a short-hand for
  ///   [`S::connect`][Subscribable::connect] that supplies the client's
  ///   [`ApiInfo`] object to the call; if your [`Subscribable`]
  ///   requires a different input then invoke its `connect` method
  ///   yourself
  #[instrument(level = "debug", skip(self))]
  pub async fn subscribe<S>(&self) -> Result<(S::Stream, S::Subscription), Error>
  where
    S: Subscribable<Input = ApiInfo>,
  {
    S::connect(&self.api_info).await
  }

  /// Retrieve the `ApiInfo` object used by this `Client` instance.
  #[inline]
  pub fn api_info(&self) -> &ApiInfo {
    &self.api_info
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use http::StatusCode;
  #[cfg(feature = "gzip")]
  use reqwest::ClientBuilder as HttpClientBuilder;
  use test_log::test;

  use crate::endpoint::ApiError;
  use crate::Str;


  Endpoint! {
    GetNotFound(()),
    Ok => (), [],
    Err => GetNotFoundError, []

    fn path(_input: &Self::Input) -> Str {
      "/v2/foobarbaz".into()
    }
  }

  #[test(tokio::test)]
  async fn unexpected_status_code_return() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::builder().max_idle_per_host(0).build(api_info);
    let result = client.issue::<GetNotFound>(&()).await;
    let err = result.unwrap_err();

    match err {
      RequestError::Endpoint(GetNotFoundError::UnexpectedStatus(status, message)) => {
        let expected = ApiError {
          code: 40410000,
          message: "endpoint not found".to_string(),
        };
        assert_eq!(message, Ok(expected));
        assert_eq!(status, StatusCode::NOT_FOUND);
      },
      _ => panic!("Received unexpected error: {:?}", err),
    };
  }

  // Confirms that gzip can be set on the `reqwest::ClientBuilder` when the gzip
  // feature is enabled for apca. Because [the docs](https://docs.rs/reqwest/0.11.10/reqwest/struct.ClientBuilder.html#method.gzip)
  // state that the `reqwest::ClientBuilder::gzip` method is only enabled when
  // the gzip feature is set to true, this test is really confirming that the
  // feature is correctly passed to the reqwest module in the Cargo.toml. To run
  // this test, use `cargo test --features gzip`.
  #[cfg(feature = "gzip")]
  #[test]
  fn gzip_is_enabled() {
    let builder = HttpClientBuilder::new();
    let _ = builder.gzip(true);
  }
}
