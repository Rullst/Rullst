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

/// A fluent testing wrapper around Axum's `Router` to enable declarative E2E assertions.
#[non_exhaustive]
pub struct TestApp {
    router: Router,
}

impl TestApp {
    /// Creates a new `TestApp` from the given Axum `Router`.
    pub fn new(router: Router) -> Self {
        Self { router }
    }

    /// Initiates a GET request.
    pub fn get(&self, uri: &str) -> TestRequestBuilder {
        TestRequestBuilder::new(self.router.clone(), Method::GET, uri)
    }

    /// Initiates a POST request.
    pub fn post(&self, uri: &str) -> TestRequestBuilder {
        TestRequestBuilder::new(self.router.clone(), Method::POST, uri)
    }

    /// Initiates a PUT request.
    pub fn put(&self, uri: &str) -> TestRequestBuilder {
        TestRequestBuilder::new(self.router.clone(), Method::PUT, uri)
    }

    /// Initiates a PATCH request.
    pub fn patch(&self, uri: &str) -> TestRequestBuilder {
        TestRequestBuilder::new(self.router.clone(), Method::PATCH, uri)
    }

    /// Initiates a DELETE request.
    pub fn delete(&self, uri: &str) -> TestRequestBuilder {
        TestRequestBuilder::new(self.router.clone(), Method::DELETE, uri)
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
}

impl TestRequestBuilder {
    pub(crate) fn new(router: Router, method: Method, uri: &str) -> Self {
        Self {
            router,
            method,
            uri: uri.to_string(),
            headers: HeaderMap::new(),
            body: None,
        }
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
        TestResponse::new(response).await
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
    pub(crate) async fn new(response: Response) -> Self {
        let (parts, body) = response.into_parts();
        // Read up to 10MB limit in testing
        let body_bytes = to_bytes(body, 10 * 1024 * 1024)
            .await
            .expect("Failed to read response body bytes");

        Self {
            status: parts.status,
            headers: parts.headers,
            body_bytes,
        }
    }

    /// Returns the HTTP status code.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns a reference to the response headers.
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
                let cookie = cookie::Cookie::parse(cookie_str).ok()?;
                if cookie.name() == name {
                    Some(cookie.value().to_string())
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
