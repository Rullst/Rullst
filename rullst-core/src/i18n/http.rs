//! The locale layer and extractor, and the `POST` locale switch.

use super::{LOCALE_COOKIE, Locale, Locales};
use crate::config::Environment;
use axum::body::Body;
use axum::extract::{FromRequestParts, Request};
use axum::http::request::Parts;
use axum::http::{Extensions, HeaderMap, HeaderName, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::MethodRouter;
use std::task::{Context, Poll};

/// Largest switch form body read.
const MAX_SWITCH_BODY_BYTES: usize = 4096;
/// Longest `next` path a switch redirects to.
const MAX_NEXT_BYTES: usize = 2048;
/// The locale cookie lasts one year.
const COOKIE_MAX_AGE_SECONDS: u32 = 31_536_000;

/// Tower layer that stores each request's negotiated [`Locale`] (see
/// [`Locales::negotiate`]) in the request extensions, where the [`Locale`]
/// extractor reads it. Created by [`Locales::layer`].
#[derive(Clone, Copy, Debug)]
pub struct LocaleLayer {
    locales: &'static Locales,
}

/// Service produced by [`LocaleLayer`].
#[derive(Clone, Copy, Debug)]
pub struct LocaleService<S> {
    inner: S,
    locales: &'static Locales,
}

impl<S> tower_layer::Layer<S> for LocaleLayer {
    type Service = LocaleService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LocaleService {
            inner,
            locales: self.locales,
        }
    }
}

impl<S, B> tower_service::Service<axum::http::Request<B>> for LocaleService<S>
where
    S: tower_service::Service<axum::http::Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: axum::http::Request<B>) -> Self::Future {
        let locale = self.locales.negotiate(request.headers());
        request.extensions_mut().insert(locale);
        self.inner.call(request)
    }
}

/// Rejection of the [`Locale`] extractor on a route without
/// [`Locales::layer`]: a `500` naming the missing layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the locale layer is not installed: add `.layer(LOCALES.layer())` to the router")]
pub struct MissingLocale;

impl IntoResponse for MissingLocale {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

/// Extracts the locale negotiated by [`Locales::layer`].
impl<S: Send + Sync> FromRequestParts<S> for Locale {
    type Rejection = MissingLocale;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Locale>()
            .copied()
            .ok_or(MissingLocale)
    }
}

#[derive(serde::Deserialize)]
struct SwitchForm {
    locale: String,
    next: Option<String>,
}

impl Locales {
    /// A layer that negotiates every request's [`Locale`] for the
    /// extractor.
    pub fn layer(&'static self) -> LocaleLayer {
        LocaleLayer { locales: self }
    }

    /// A `POST` route that switches the visitor's locale.
    ///
    /// It reads an `application/x-www-form-urlencoded` form with `locale`
    /// (a supported tag, compared ignoring case) and an optional `next`
    /// path, stores the tag in the [`LOCALE_COOKIE`] cookie (`Path=/`, one
    /// year, `SameSite=Lax`, `HttpOnly`, and `Secure` in staging and
    /// production) and answers `303 See Other` to `next` when it is a local
    /// path (`/…`, not `//` or `\`, printable ASCII, up to 2 KiB), else to
    /// `/`. An htmx request (`HX-Request: true`) gets `204` with
    /// `HX-Redirect` instead, so the whole page reloads in the new locale.
    /// An unsupported locale or a malformed form is a `400`; a body over
    /// 4 KiB is a `413`. Being a form `POST`, it is protected by the
    /// application's CSRF middleware like any other form, so include the
    /// `_token` field.
    pub fn switch_route(&'static self) -> MethodRouter {
        axum::routing::post(move |request: Request| self.switch(request))
    }

    async fn switch(&self, request: Request) -> Response {
        let secure = secure_cookies(request.extensions());
        let (parts, body) = request.into_parts();
        let Ok(bytes) = axum::body::to_bytes(body, MAX_SWITCH_BODY_BYTES).await else {
            return (StatusCode::PAYLOAD_TOO_LARGE, "Locale form too large").into_response();
        };
        let Ok(form) = serde_urlencoded::from_bytes::<SwitchForm>(&bytes) else {
            return (StatusCode::BAD_REQUEST, "Invalid locale form").into_response();
        };
        let Some(locale) = self.get(&form.locale) else {
            return (StatusCode::BAD_REQUEST, "Unsupported locale").into_response();
        };
        let target = form
            .next
            .as_deref()
            .filter(|next| is_local_path(next))
            .unwrap_or("/");
        switch_response(&parts.headers, locale, target, secure)
    }
}

fn switch_response(headers: &HeaderMap, locale: Locale, target: &str, secure: bool) -> Response {
    let htmx = headers
        .get("HX-Request")
        .is_some_and(|value| value.as_bytes() == b"true");
    let (status, location) = if htmx {
        (
            StatusCode::NO_CONTENT,
            HeaderName::from_static("hx-redirect"),
        )
    } else {
        (StatusCode::SEE_OTHER, header::LOCATION)
    };
    let mut response = Response::new(Body::empty());
    *response.status_mut() = status;
    let cookie = format!(
        "{LOCALE_COOKIE}={}; Path=/; Max-Age={COOKIE_MAX_AGE_SECONDS}; SameSite=Lax; HttpOnly{}",
        locale.tag(),
        if secure { "; Secure" } else { "" }
    );
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(target) {
        headers.insert(location, value);
    }
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        headers.insert(header::SET_COOKIE, value);
    }
    response
}

/// A same-origin path: starts with one `/`, has no `\` and only printable
/// ASCII (browsers drop tabs and newlines, which could form `//`).
fn is_local_path(next: &str) -> bool {
    next.len() <= MAX_NEXT_BYTES
        && next.starts_with('/')
        && !next.starts_with("//")
        && next
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b'\\')
}

/// `Secure` cookies follow the CSRF cookie's rule: the request's
/// [`Environment`], else the detected one, else secure.
fn secure_cookies(extensions: &Extensions) -> bool {
    extensions
        .get::<Environment>()
        .copied()
        .map(Environment::requires_secure_defaults)
        .unwrap_or_else(|| {
            Environment::detect(None).map_or(true, Environment::requires_secure_defaults)
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "http_tests.rs"]
mod tests;
