//! Machine registrations, constant paths and route methods.
use super::*;

fn routes(code: &str) -> MachineRoutes {
    let mut constants = Constants::default();
    constants.include(code);
    let mut routes = MachineRoutes::default();
    routes.include(code, &constants);
    routes
}

#[test]
fn every_registering_constructor_and_write_method_counts() {
    let code = r#"
        MachineEndpoint::bearer(Method::POST, "/api/messages", token)?,
        MachineEndpoint::verified_bearer(axum::http::Method::PUT, "/api/posts/{id}", v)?,
        MachineEndpoint::signed_webhook(rullst::server::Method::PATCH, "/hooks/:id", v)?,
        MachineEndpoint::address_token(
            Method::DELETE,
            "/pay/{reference}/{token}",
            Verifier(key.clone(), [1, 2]),
        )?,
        MachineEndpoint::mutual_tls(Method::POST, "/mtls/{{id}}", v)?,
    "#;
    let found = routes(code);
    assert_eq!(
        found,
        MachineRoutes(vec![
            ("POST", "/api/messages".to_owned()),
            ("PUT", "/api/posts/{}".to_owned()),
            ("PATCH", "/hooks/{}".to_owned()),
            ("DELETE", "/pay/{}/{}".to_owned()),
            ("POST", "/mtls/{}".to_owned()),
        ])
    );
    assert!(found.covers(&["PUT"], "/api/posts/:post_id"));
    assert!(found.covers(&["DELETE"], "/pay/{ref}/{t}"));
    assert!(!found.covers(&["POST"], "/api/posts/{id}"));
    assert!(!found.covers(&["PUT", "GET"], "/api/posts/{id}"));
    assert!(!found.covers(&[], "/api/posts/{id}"));
    assert!(!found.covers(&["PUT"], "/api/posts/{id}/extra"));
    assert!(!found.covers(&["PUT"], "/api/other/{id}"));
    assert!(found.declares("/api/posts/{x}") && !found.declares("/api/posts"));
}

#[test]
fn unresolvable_registrations_give_no_evidence() {
    for code in [
        r#"MachineEndpoint::bearer(method, "/api/{id}", token)"#,
        r#"MachineEndpoint::bearer(Method::GET, "/api/{id}", token)"#,
        r#"MachineEndpoint::bearer(Method::POST, format!("/api/{}", id), token)"#,
        r#"MachineEndpoint::bearer(Method::POST, "/api/{id}" , token"#,
        r#"MachineEndpoint::bearer(Method::POST, "/api/\"{id}", token)"#,
        r#"MachineEndpoint::bearer(Method::POST, "api/{id}", token)"#,
        r#"MachineEndpoint::bearer(Method::POST, UNKNOWN, token)"#,
        r#"MachineEndpoint::bearer(Method::POST, path(), token)"#,
        r#"MachineEndpoint::authentication(Method::POST, "/api/{id}")"#,
        r#"MachineEndpointPolicy::new(Method::POST, "/api/{id}")"#,
        r#"MachineEndpoint::bearer(Method::POST)"#,
    ] {
        assert!(routes(code).0.is_empty(), "{code}");
    }
}

#[test]
fn constant_paths_resolve_within_the_crate() {
    let code = r#"
        pub const ROUTE: &str = "/payments/{reference}/{token}";
        pub(crate) const OTHER: &'static str = "/other/{id}";
        const NOT_A_STR: u32 = 7;
        const ESCAPED: &str = "/a\"b/{id}";
        fn register() {
            MachineEndpoint::address_token(Method::POST, infinitepay::ROUTE, v);
            MachineEndpoint::signed_webhook(Method::PUT, &OTHER, v);
            MachineEndpoint::signed_webhook(Method::PUT, ESCAPED, v);
            MachineEndpoint::signed_webhook(Method::PUT, NOT_A_STR, v);
        }
    "#;
    assert_eq!(
        routes(code),
        MachineRoutes(vec![
            ("POST", "/payments/{}/{}".to_owned()),
            ("PUT", "/other/{}".to_owned()),
        ])
    );
    // Two different values under one name resolve to nothing.
    let ambiguous = r#"
        mod a { const ROUTE: &str = "/a/{id}"; }
        mod b { const ROUTE: &str = "/b/{id}"; }
        const SAME: &str = "/same/{id}";
        const SAME: &str = "/same/{id}";
        const CONSTANT_ROUTE: &str = "/x/{id}";
        MachineEndpoint::bearer(Method::POST, ROUTE, t);
        MachineEndpoint::bearer(Method::POST, SAME, t);
    "#;
    assert_eq!(
        routes(ambiguous),
        MachineRoutes(vec![("POST", "/same/{}".to_owned())])
    );
    let mut constants = Constants::default();
    constants.include("let constant = 1; const_fn(); constants: &str = \"/x\";");
    assert!(constants.0.is_empty());
}

#[test]
fn route_methods_are_whole_calls() {
    assert_eq!(route_methods(r#"post("/a/{id}" => h),"#), ["POST"]);
    assert_eq!(
        route_methods(r#".route("/a/{id}", get(show).put(edit).delete(drop))"#),
        ["GET", "PUT", "DELETE"]
    );
    assert_eq!(route_methods("patch(h) ws(socket)"), ["PATCH", "WS"]);
    assert!(route_methods("budget(1); repost(2); undelete(3); output(4)").is_empty());
    assert_eq!(
        arguments("a, (b, c), [d, e], \"f, ) g\") rest"),
        ["a", " (b, c)", " [d, e]", " \"f, ) g\""]
    );
    assert!(arguments("unterminated, call").is_empty());
}
