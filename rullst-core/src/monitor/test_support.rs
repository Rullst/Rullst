//! Fixed tokens and request helpers for the monitor tests.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use std::net::SocketAddr;

/// `openssl rand -base64 32` output.
pub(super) const BASE64_TOKEN: &str = "DjxqcYY6VXbaPLLGGkjMN3aJCjqkMo8TcEjFfXD6f7Y=";
/// 32 random bytes as unpadded base64url.
pub(super) const BASE64URL_TOKEN: &str = "fH6MncgeJPW9jH2ICx_2ylFrxtHa5ztWWiN4xPEgE-I";
/// `openssl rand -hex 32` output.
pub(super) const HEX_TOKEN: &str =
    "e3dfebee04f62231e19ec1a191cb357bf6602bf9065d5a51e85467a9054de557";
/// A second token, as during a rotation.
pub(super) const ROTATED_TOKEN: &str = "NXRt2EGKIzhSo4cMVHuHW4oQ8r80oD7vX6VV1XZPpUM=";

/// A `GET` from `peer` with the given headers.
pub(super) fn get(uri: &str, peer: &str, headers: &[(&str, &str)]) -> Request<Body> {
    let mut builder = Request::builder().uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let mut request = builder.body(Body::empty()).unwrap();
    request.extensions_mut().insert(axum::extract::ConnectInfo(
        peer.parse::<SocketAddr>().unwrap(),
    ));
    request
}

/// The `Authorization` header for `token`.
pub(super) fn bearer(token: &str) -> (&'static str, String) {
    ("authorization", format!("Bearer {token}"))
}

/// Status and body of a response.
pub(super) async fn read(response: Response) -> (StatusCode, Vec<u8>) {
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap()
        .to_vec();
    (status, body)
}
