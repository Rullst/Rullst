use super::*;

fn reason(result: Result<impl std::fmt::Debug, ConnectError>) -> String {
    match result {
        Err(ConnectError::InvalidConfiguration { reason, .. }) => reason,
        other => panic!("expected a configuration error, got {other:?}"),
    }
}

#[test]
fn issuers_cannot_carry_a_query() {
    assert_eq!(
        reason(validate_issuer_url("https://idp.example/?tenant=a")),
        "issuer URLs cannot contain a query string or fragment"
    );
    assert!(validate_issuer_url("https://idp.example/tenant").is_ok());
}

#[test]
fn discovered_http_endpoints_need_the_exact_loopback_issuer_origin() {
    let refused =
        "discovered endpoints must use HTTPS; HTTP is limited to the exact issuer loopback origin";
    let public = Url::parse("https://idp.example").expect("issuer");
    assert_eq!(
        reason(validate_discovery_endpoint(
            "token_endpoint",
            "http://idp.example/token",
            &public
        )),
        refused
    );
    let loopback = Url::parse("http://127.0.0.1:8080").expect("issuer");
    for endpoint in ["http://127.0.0.1:9090/token", "http://localhost:8080/token"] {
        assert_eq!(
            reason(validate_discovery_endpoint(
                "token_endpoint",
                endpoint,
                &loopback
            )),
            refused,
            "{endpoint}"
        );
    }
    assert!(
        validate_discovery_endpoint("token_endpoint", "http://127.0.0.1:8080/token", &loopback)
            .is_ok()
    );
    assert!(
        validate_discovery_endpoint("token_endpoint", "https://other.example/token", &public)
            .is_ok()
    );
}

#[test]
fn hosts_have_no_path_or_query() {
    for host in ["idp.example/tenant", "idp.example?x=1"] {
        assert_eq!(
            reason(validate_https_host("domain", host)),
            "must be a host name without path or query",
            "{host}"
        );
    }
    assert_eq!(
        validate_https_host("domain", " idp.example:8443/ ").expect("host"),
        "idp.example:8443"
    );
}

#[test]
fn urls_are_http_without_embedded_credentials() {
    assert_eq!(
        reason(validate_https_endpoint(
            "endpoint",
            "ftp://idp.example/file"
        )),
        "must be an absolute HTTP or HTTPS URL"
    );
    for url in ["https://user@idp.example/", "https://:secret@idp.example/"] {
        assert_eq!(
            reason(validate_https_endpoint("endpoint", url)),
            "embedded URL credentials are not allowed",
            "{url}"
        );
    }
}
