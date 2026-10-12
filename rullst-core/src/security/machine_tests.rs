#![allow(clippy::expect_used)]

use super::*;

#[test]
fn a_placeholder_matches_exactly_one_ordinary_segment() {
    for (pattern, path) in [
        ("/api/posts/{id}", "/api/posts/7"),
        ("/api/posts/{id}", "/api/posts/a.b"),
        ("/api/{tenant}/posts/{id}", "/api/acme/posts/7"),
        ("/billing/reconcile", "/billing/reconcile"),
        ("/", "/"),
    ] {
        assert!(path_matches(pattern, path), "{pattern} {path}");
    }
    for (pattern, path) in [
        ("/api/posts/{id}", "/api/posts/"),
        ("/api/posts/{id}", "/api/posts/."),
        ("/api/posts/{id}", "/api/posts/.."),
        ("/api/posts/{id}", "/api/posts/7/8"),
        ("/api/posts/{id}", "/api/posts"),
        ("/api/posts/{id}", "/api/users/7"),
        ("/api/{tenant}/posts/{id}", "/api/acme/users/7"),
        ("/billing/reconcile", "/billing/reconcile/"),
        ("/billing/reconcile", "/billing"),
    ] {
        assert!(!path_matches(pattern, path), "{pattern} {path}");
    }
}

#[test]
fn routes_overlap_when_one_path_can_match_both() {
    assert!(overlaps("/a/{id}", "/a/b"));
    assert!(overlaps("/a/b", "/a/{id}"));
    assert!(overlaps("/a/{x}/c", "/a/b/{y}"));
    assert!(overlaps("/a/b", "/a/b"));
    assert!(!overlaps("/a/{id}", "/a/{id}/c"));
    assert!(!overlaps("/a/{id}", "/b/{id}"));
    assert!(!overlaps("/a/{x}/c", "/a/{y}/d"));
}

const TOKEN_A: &str = "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6";
const TOKEN_B: &str = "f0e9d8c7b6a5f4e3d2c1b0a9f8e7d6c5";

fn bearer(method: Method, path: &str, token: &str) -> MachineEndpoint {
    MachineEndpoint::bearer(method, path, token).expect("valid bearer endpoint")
}

fn request(method: Method, path: &str, token: Option<&str>, body: Vec<u8>) -> Request {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    builder.body(axum::body::Body::from(body)).expect("request")
}

#[test]
fn bearer_tokens_need_their_length_printable_bytes_and_eight_distinct_values() {
    assert!(MachineEndpoint::bearer(Method::POST, "/hooks/a", "abcdefgh".repeat(4)).is_ok());
    for weak in [
        "abcdefg".repeat(5),
        "abcdefghijklmnop qrstuvwxyz0123456".to_owned(),
        "abcdefghijklmnopqrstuvwxyz01234".to_owned(),
    ] {
        assert!(
            matches!(
                MachineEndpoint::bearer(Method::POST, "/hooks/a", weak),
                Err(MachineEndpointError::InvalidCredential)
            ),
            "weak token accepted"
        );
    }
}

#[tokio::test]
async fn each_route_checks_its_own_token() {
    let policy = MachineEndpointPolicy::new(vec![
        bearer(Method::POST, "/hooks/a", TOKEN_A),
        bearer(Method::POST, "/hooks/b", TOKEN_B),
    ])
    .expect("policy");
    let accepted = policy
        .authenticate(request(Method::POST, "/hooks/b", Some(TOKEN_B), Vec::new()))
        .await;
    assert!(accepted.is_ok());
    let rejected = policy
        .authenticate(request(Method::POST, "/hooks/b", Some(TOKEN_A), Vec::new()))
        .await;
    assert_eq!(rejected.err(), Some(StatusCode::UNAUTHORIZED));
}

#[tokio::test]
async fn an_authentication_proof_covers_only_its_own_route_and_policy() {
    let policy = MachineEndpointPolicy::new(vec![
        bearer(Method::POST, "/hooks/a", TOKEN_A),
        bearer(Method::PUT, "/hooks/a", TOKEN_A),
        bearer(Method::POST, "/hooks/b", TOKEN_B),
    ])
    .expect("policy");
    let other = MachineEndpointPolicy::new(vec![bearer(Method::POST, "/hooks/a", TOKEN_A)])
        .expect("policy");
    let with_proof = |method: Method, path: &str, policy: &MachineEndpointPolicy| {
        let mut request = request(method, path, None, Vec::new());
        request.extensions_mut().insert(MachineAuthenticated {
            method: Method::POST,
            path: "/hooks/a".to_owned(),
            policy: policy.clone(),
        });
        request
    };

    assert!(
        policy
            .authenticate(with_proof(Method::POST, "/hooks/a", &policy))
            .await
            .is_ok()
    );
    for (method, path, proof_policy) in [
        (Method::PUT, "/hooks/a", &policy),
        (Method::POST, "/hooks/b", &policy),
        (Method::POST, "/hooks/a", &other),
    ] {
        let result = policy
            .authenticate(with_proof(method.clone(), path, proof_policy))
            .await;
        assert_eq!(
            result.err(),
            Some(StatusCode::UNAUTHORIZED),
            "{method} {path}"
        );
    }
}

#[tokio::test]
async fn machine_bodies_are_buffered_up_to_one_mebibyte() {
    let policy = MachineEndpointPolicy::new(vec![bearer(Method::POST, "/hooks/a", TOKEN_A)])
        .expect("policy");
    let accepted = policy
        .authenticate(request(
            Method::POST,
            "/hooks/a",
            Some(TOKEN_A),
            vec![b'x'; 1_048_576],
        ))
        .await
        .expect("a body at the limit");
    let body = axum::body::to_bytes(accepted.into_body(), 2 * 1_048_576)
        .await
        .expect("replayed body");
    assert_eq!(body.len(), 1_048_576);

    let rejected = policy
        .authenticate(request(
            Method::POST,
            "/hooks/a",
            Some(TOKEN_A),
            vec![b'x'; 1_048_577],
        ))
        .await;
    assert_eq!(rejected.err(), Some(StatusCode::PAYLOAD_TOO_LARGE));
}
