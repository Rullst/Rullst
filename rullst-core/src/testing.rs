#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use axum::{
    Router,
    body::{Body, Bytes, to_bytes},
    http::{HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode, header},
    response::Response,
};
use serde::Serialize;
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use tower::ServiceExt;

mod csp_policy;
mod html_check;
mod html_normalize;
mod snapshot;

pub use crate::assert_html_snapshot;
pub use html_check::{HtmlIssue, HtmlIssueKind, html_issues};
pub use html_normalize::{CSRF_TOKEN_PLACEHOLDER, NONCE_PLACEHOLDER, normalize_html};
#[doc(hidden)]
pub use snapshot::__assert_html_snapshot;
pub use snapshot::{SnapshotOptions, UPDATE_SNAPSHOTS_ENV};

#[cfg_attr(mutants, mutants::skip)]
const DEFAULT_MAX_BODY: usize = 10 * 1024 * 1024;

/// A fluent testing wrapper around Axum's `Router` to enable declarative E2E assertions.
#[non_exhaustive]
pub struct TestApp {
    router: Router,
    max_body_bytes: usize,
}

impl TestApp {
    /// Creates a new `TestApp` from the given Axum `Router`.
    pub fn new(router: Router) -> Self {
        Self {
            router,
            max_body_bytes: DEFAULT_MAX_BODY,
        }
    }

    /// Creates a new `TestApp` with a custom maximum response body limit in bytes.
    pub fn new_with_limit(router: Router, max_body_bytes: usize) -> Self {
        Self {
            router,
            max_body_bytes,
        }
    }

    /// Initiates a GET request.
    pub fn get(&self, uri: &str) -> TestRequestBuilder {
        TestRequestBuilder::new(self.router.clone(), Method::GET, uri, self.max_body_bytes)
    }

    /// Initiates a POST request.
    pub fn post(&self, uri: &str) -> TestRequestBuilder {
        TestRequestBuilder::new(self.router.clone(), Method::POST, uri, self.max_body_bytes)
    }

    /// Initiates a PUT request.
    pub fn put(&self, uri: &str) -> TestRequestBuilder {
        TestRequestBuilder::new(self.router.clone(), Method::PUT, uri, self.max_body_bytes)
    }

    /// Initiates a PATCH request.
    pub fn patch(&self, uri: &str) -> TestRequestBuilder {
        TestRequestBuilder::new(self.router.clone(), Method::PATCH, uri, self.max_body_bytes)
    }

    /// Initiates a DELETE request.
    pub fn delete(&self, uri: &str) -> TestRequestBuilder {
        TestRequestBuilder::new(
            self.router.clone(),
            Method::DELETE,
            uri,
            self.max_body_bytes,
        )
    }
}

/// A request builder for constructing HTTP requests in tests.
/// Implements `IntoFuture` so it can be `.await`ed directly to send the request.
#[non_exhaustive]
pub struct TestRequestBuilder {
    router: Router,
    method: Method,
    uri: String,
    headers: HeaderMap,
    body: Option<Body>,
    max_body_bytes: usize,
}

impl TestRequestBuilder {
    pub(crate) fn new(router: Router, method: Method, uri: &str, max_body_bytes: usize) -> Self {
        Self {
            router,
            method,
            uri: uri.to_string(),
            headers: HeaderMap::new(),
            body: None,
            max_body_bytes,
        }
    }

    /// Sets a custom response body limit in bytes for this specific request.
    pub fn max_body_bytes(mut self, max_body_bytes: usize) -> Self {
        self.max_body_bytes = max_body_bytes;
        self
    }

    /// Adds a header to the request.
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        HeaderValue: TryFrom<V>,
    {
        if let (Ok(k), Ok(v)) = (HeaderName::try_from(key), HeaderValue::try_from(value)) {
            self.headers.insert(k, v);
        }
        self
    }

    /// Sets the JSON payload for the request.
    pub fn json<T: Serialize>(mut self, data: &T) -> Self {
        self.headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        let body_bytes = serde_json::to_vec(data).expect("Failed to serialize body as JSON");
        self.body = Some(Body::from(body_bytes));
        self
    }

    /// Sets the URL-encoded form payload for the request.
    pub fn form<T: Serialize>(mut self, data: &T) -> Self {
        self.headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        let body_string = serde_urlencoded::to_string(data)
            .expect("Failed to serialize body as form URL-encoded");
        self.body = Some(Body::from(body_string));
        self
    }

    /// Sets the raw body of the request.
    pub fn body<B: Into<Body>>(mut self, body: B) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Sends the request and returns the response.
    pub async fn send(self) -> TestResponse {
        let mut req_builder = Request::builder().method(self.method).uri(&self.uri);

        for (k, v) in self.headers {
            if let Some(k) = k {
                req_builder = req_builder.header(k, v);
            }
        }

        let body = self.body.unwrap_or_else(Body::empty);
        let req = req_builder
            .body(body)
            .expect("Failed to build HTTP request");

        let response = self
            .router
            .oneshot(req)
            .await
            .expect("Failed to execute request on Router");
        TestResponse::new_with_limit(response, self.max_body_bytes).await
    }
}

impl IntoFuture for TestRequestBuilder {
    type Output = TestResponse;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

/// A wrapper around Axum's `Response` that provides fluent assertion methods.
#[non_exhaustive]
pub struct TestResponse {
    status: StatusCode,
    headers: HeaderMap,
    body_bytes: Bytes,
}

impl TestResponse {
    pub(crate) async fn new_with_limit(response: Response, max_body_bytes: usize) -> Self {
        let (parts, body) = response.into_parts();
        let body_bytes = to_bytes(body, max_body_bytes)
            .await
            .unwrap_or_else(|err| {
                panic!(
                    "Failed to read response body bytes: response body exceeds the maximum configured limit of {} bytes (or another stream error occurred: {})",
                    max_body_bytes, err
                );
            });

        Self {
            status: parts.status,
            headers: parts.headers,
            body_bytes,
        }
    }

    /// Returns the HTTP status code.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns a reference to the response headers.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns the response body parsed as a string.
    pub fn body_string(&self) -> String {
        String::from_utf8_lossy(&self.body_bytes).to_string()
    }

    /// Deserializes the JSON response body.
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> T {
        serde_json::from_slice(&self.body_bytes).expect("Failed to parse response body as JSON")
    }

    /// Returns the value of a specific cookie from the `Set-Cookie` header.
    pub fn cookie_value(&self, name: &str) -> Option<String> {
        self.headers
            .get_all(header::SET_COOKIE)
            .iter()
            .find_map(|value| {
                let cookie_str = value.to_str().ok()?;
                let main_part = cookie_str.split(';').next()?.trim();
                let (cookie_name, cookie_val) = main_part.split_once('=')?;
                if cookie_name.trim() == name {
                    Some(cookie_val.trim().to_string())
                } else {
                    None
                }
            })
    }

    /// Asserts that the response status code matches the expected value.
    pub fn assert_status(&self, expected: u16) -> &Self {
        assert_eq!(
            self.status.as_u16(),
            expected,
            "Expected HTTP status code {}, but received {}.\nResponse Body: {}",
            expected,
            self.status.as_u16(),
            self.body_string()
        );
        self
    }

    /// Asserts that the response body contains the given text.
    pub fn assert_see(&self, expected: &str) -> &Self {
        let body_str = self.body_string();
        assert!(
            body_str.contains(expected),
            "Expected response body to contain '{}', but it did not.\nResponse Body: {}",
            expected,
            body_str
        );
        self
    }

    /// Asserts that the response body does not contain the given text.
    pub fn assert_dont_see(&self, expected: &str) -> &Self {
        let body_str = self.body_string();
        assert!(
            !body_str.contains(expected),
            "Expected response body NOT to contain '{}', but it did.\nResponse Body: {}",
            expected,
            body_str
        );
        self
    }

    /// Asserts that a response header matches the expected value.
    pub fn assert_header(&self, name: &str, expected: &str) -> &Self {
        let header_val = self
            .headers
            .get(name)
            .unwrap_or_else(|| {
                panic!(
                    "Expected header '{}' to be present, but it was missing",
                    name
                )
            })
            .to_str()
            .unwrap_or_else(|_| panic!("Failed to convert value of header '{}' to a string", name));

        assert_eq!(
            header_val, expected,
            "Expected header '{}' to be '{}', but got '{}'",
            name, expected, header_val
        );
        self
    }

    /// Asserts the security headers of Rullst's browser baseline: a
    /// non-empty `Content-Security-Policy`, `X-Content-Type-Options: nosniff`
    /// and frame protection, either `X-Frame-Options` `DENY` or `SAMEORIGIN`
    /// or a CSP `frame-ancestors` directive.
    ///
    /// Added in 13.0.0-alpha.5.
    pub fn assert_security_headers(&self) -> &Self {
        let policy = self.content_security_policy();
        assert!(
            policy
                .as_deref()
                .is_some_and(|policy| !policy.trim().is_empty()),
            "Expected a Content-Security-Policy header, but it was missing or empty"
        );
        let nosniff = self.header_text(header::X_CONTENT_TYPE_OPTIONS);
        assert!(
            nosniff.is_some_and(|value| value.trim().eq_ignore_ascii_case("nosniff")),
            "Expected X-Content-Type-Options: nosniff, but got {nosniff:?}"
        );
        let frame = self.header_text(header::X_FRAME_OPTIONS);
        let refused = frame.is_some_and(|value| {
            ["DENY", "SAMEORIGIN"]
                .iter()
                .any(|option| value.trim().eq_ignore_ascii_case(option))
        });
        let ancestors = policy.as_deref().is_some_and(|policy| {
            policy.split([';', ',']).any(|directive| {
                directive
                    .split_ascii_whitespace()
                    .next()
                    .is_some_and(|name| name.eq_ignore_ascii_case("frame-ancestors"))
            })
        });
        assert!(
            refused || ancestors,
            "Expected frame protection (X-Frame-Options DENY or SAMEORIGIN, or a CSP frame-ancestors directive), but got X-Frame-Options {frame:?} and Content-Security-Policy {policy:?}"
        );
        self
    }

    /// Asserts that an HTML response (`text/html` or
    /// `application/xhtml+xml`) has none of the problems [`html_issues`]
    /// finds under the response's own `Content-Security-Policy`: markup
    /// escaped twice, broken attribute quoting, boolean attributes set to
    /// `"false"`, inline `<script>`/`<style>` without the CSP nonce, and
    /// `style`/`on*` attributes the policy blocks. Other content types pass
    /// unchecked.
    ///
    /// Added in 13.0.0-alpha.5.
    pub fn assert_html_hygiene(&self) -> &Self {
        let html = self.header_text(header::CONTENT_TYPE).is_some_and(|value| {
            let value = value.trim_start().to_ascii_lowercase();
            value.starts_with("text/html") || value.starts_with("application/xhtml+xml")
        });
        if !html {
            return self;
        }
        let issues = html_issues(
            &self.body_string(),
            self.content_security_policy().as_deref(),
        );
        assert!(
            issues.is_empty(),
            "Expected HTML without escaping, quoting or CSP problems, but found {}:\n{}",
            issues.len(),
            issues
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        );
        self
    }

    /// The first value of `name` as text.
    fn header_text(&self, name: HeaderName) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }

    /// Every `Content-Security-Policy` value, joined as separate policies.
    fn content_security_policy(&self) -> Option<String> {
        let policies = self
            .headers
            .get_all(header::CONTENT_SECURITY_POLICY)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect::<Vec<_>>();
        (!policies.is_empty()).then(|| policies.join(", "))
    }

    /// Asserts that the response body matches the given JSON structure.
    pub fn assert_json<
        T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug + PartialEq,
    >(
        &self,
        expected: &T,
    ) -> &Self {
        let actual: T = self.json();
        assert_eq!(
            actual, *expected,
            "JSON structures do not match.\nExpected: {:?}\nActual: {:?}",
            expected, actual
        );
        self
    }

    /// Asserts that a cookie with the given name is present and matches the expected value.
    pub fn assert_cookie(&self, name: &str, expected: &str) -> &Self {
        let actual = self.cookie_value(name).unwrap_or_else(|| {
            panic!(
                "Expected cookie '{}' to be present, but it was missing",
                name
            )
        });

        assert_eq!(
            actual, expected,
            "Expected cookie '{}' to be '{}', but got '{}'",
            name, expected, actual
        );
        self
    }

    /// Asserts that a cookie with the given name exists in the response.
    pub fn assert_has_cookie(&self, name: &str) -> &Self {
        assert!(
            self.cookie_value(name).is_some(),
            "Expected cookie '{}' to be present, but it was missing",
            name
        );
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_max_body() {
        assert_eq!(DEFAULT_MAX_BODY, 10 * 1024 * 1024); // Kills * replaced with + mutant
    }
}

#[cfg(test)]
#[path = "testing_contract_tests.rs"]
mod contract_tests;

#[cfg(test)]
#[path = "testing/snapshot_tests.rs"]
mod snapshot_tests;

#[cfg(test)]
#[path = "testing/response_check_tests.rs"]
mod response_check_tests;
