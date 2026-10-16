//! Explicit `Cache-Control` policies for responses that may be cached.
//!
//! The production security baseline answers every response without a
//! `Cache-Control` header with `no-store`, the right default for pages that
//! show a user's data. A route whose responses may be stored, such as a
//! public download or a calendar feed, opts in here: with a layer for every
//! successful response of a route, or per response from a handler.
//!
//! ```rust,no_run
//! use rullst_core::{cache_control, Router, routing::get};
//! use std::time::Duration;
//!
//! # async fn feed() -> &'static str { "BEGIN:VCALENDAR" }
//! # async fn download() -> &'static str { "report" }
//! let app = Router::new()
//!     // Browsers and shared caches (proxies, CDNs) may keep it 5 minutes.
//!     .route("/calendar.ics", get(feed).layer(cache_control::public(Duration::from_secs(300))))
//!     // Only the user's browser may keep it, for an hour.
//!     .route("/reports/latest", get(download).layer(cache_control::private(Duration::from_secs(3600))));
//! ```
//!
//! Use `public` only for responses that are the same for every visitor: a
//! shared cache may serve one visitor's response to another. The CSRF
//! middleware never sets its cookie on a response that shared caches may
//! store.

use axum::{
    http::{HeaderMap, HeaderValue, Request, Response, StatusCode, header},
    response::{IntoResponseParts, ResponseParts},
};
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// A `Cache-Control` value with an explicit lifetime.
///
/// As a layer ([`Self::layer`], [`public`], [`private`]) it labels the `2xx`
/// and `304` responses of the routes it wraps that have no `Cache-Control`
/// yet; errors keep the security baseline's `no-store`. Returned from a
/// handler (it implements [`IntoResponseParts`]) it sets the header on that
/// response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheControl {
    value: HeaderValue,
}

impl CacheControl {
    /// `public, max-age=<seconds>`: browsers and shared caches may store the
    /// response for `max_age` (whole seconds).
    pub fn public(max_age: Duration) -> Self {
        Self::with_max_age("public", max_age)
    }

    /// `private, max-age=<seconds>`: only the user's browser may store the
    /// response, for `max_age` (whole seconds).
    pub fn private(max_age: Duration) -> Self {
        Self::with_max_age("private", max_age)
    }

    fn with_max_age(scope: &str, max_age: Duration) -> Self {
        let value = format!("{scope}, max-age={}", max_age.as_secs());
        Self {
            // Digits and letters only, so this never falls back.
            value: HeaderValue::try_from(value)
                .unwrap_or_else(|_| HeaderValue::from_static("no-store")),
        }
    }

    /// The header value, such as `public, max-age=300`.
    pub fn header_value(&self) -> &HeaderValue {
        &self.value
    }

    /// A layer that applies this policy to the routes it wraps.
    pub fn layer(self) -> CacheControlLayer {
        CacheControlLayer { policy: self }
    }
}

impl IntoResponseParts for CacheControl {
    type Error = Infallible;

    fn into_response_parts(self, mut parts: ResponseParts) -> Result<ResponseParts, Infallible> {
        parts
            .headers_mut()
            .insert(header::CACHE_CONTROL, self.value);
        Ok(parts)
    }
}

/// A layer setting `public, max-age=<seconds>` on the successful responses
/// of the routes it wraps; see [`CacheControl::public`].
pub fn public(max_age: Duration) -> CacheControlLayer {
    CacheControl::public(max_age).layer()
}

/// A layer setting `private, max-age=<seconds>` on the successful responses
/// of the routes it wraps; see [`CacheControl::private`].
pub fn private(max_age: Duration) -> CacheControlLayer {
    CacheControl::private(max_age).layer()
}

/// Tower layer of a [`CacheControl`] policy.
#[derive(Debug, Clone)]
pub struct CacheControlLayer {
    policy: CacheControl,
}

impl<S> tower_layer::Layer<S> for CacheControlLayer {
    type Service = CacheControlService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheControlService {
            inner,
            value: self.policy.value.clone(),
        }
    }
}

/// Service produced by [`CacheControlLayer`].
#[derive(Debug, Clone)]
pub struct CacheControlService<S> {
    inner: S,
    value: HeaderValue,
}

impl<S, RequestBody, ResponseBody> tower_service::Service<Request<RequestBody>>
    for CacheControlService<S>
where
    S: tower_service::Service<Request<RequestBody>, Response = Response<ResponseBody>>,
    S::Future: Send + 'static,
    S::Error: 'static,
    ResponseBody: 'static,
{
    type Response = Response<ResponseBody>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<RequestBody>) -> Self::Future {
        let response = self.inner.call(request);
        let value = self.value.clone();
        Box::pin(async move {
            let mut response = response.await?;
            label(response.status(), response.headers_mut(), value);
            Ok(response)
        })
    }
}

/// Sets `value` on a `2xx` or `304` response without a `Cache-Control`.
fn label(status: StatusCode, headers: &mut HeaderMap, value: HeaderValue) {
    if status.is_success() || status == StatusCode::NOT_MODIFIED {
        headers.entry(header::CACHE_CONTROL).or_insert(value);
    }
}

/// Whether a shared cache may store a response with these headers: a
/// `Cache-Control` with `public` or `s-maxage`, and without `private` or
/// `no-store`.
pub(crate) fn allows_shared_caching(headers: &HeaderMap) -> bool {
    let mut shared = false;
    for directive in headers
        .get_all(header::CACHE_CONTROL)
        .iter()
        .flat_map(|value| value.as_bytes().split(|byte| *byte == b','))
    {
        let name = directive
            .split(|byte| *byte == b'=')
            .next()
            .unwrap_or_default()
            .trim_ascii();
        if name.eq_ignore_ascii_case(b"private") || name.eq_ignore_ascii_case(b"no-store") {
            return false;
        }
        shared |= name.eq_ignore_ascii_case(b"public") || name.eq_ignore_ascii_case(b"s-maxage");
    }
    shared
}

#[cfg(test)]
#[path = "cache_control_tests.rs"]
mod tests;
