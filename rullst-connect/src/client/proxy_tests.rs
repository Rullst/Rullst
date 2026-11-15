use super::*;

fn refused(result: Result<url::Url, crate::ConnectError>) -> String {
    match result {
        Err(crate::ConnectError::InvalidConfiguration { field, reason }) => {
            assert_eq!(field, "proxy_url");
            reason
        }
        other => panic!("expected a proxy configuration error, got {other:?}"),
    }
}

#[test]
fn proxy_urls_are_bounded_authority_only_http_endpoints() {
    let length = "proxy URL must contain 1 to 2048 bytes";
    assert_eq!(refused(validate_proxy_url(String::new(), false)), length);
    let long = |bytes: usize| {
        let base = "http://proxy.example/";
        format!("{base}{}", "a".repeat(bytes - base.len()))
    };
    assert_eq!(refused(validate_proxy_url(long(2_049), false)), length);
    // A 2,048-byte URL passes the length check and fails on its path.
    let authority_only = "proxy URL must contain only scheme and authority";
    assert_eq!(
        refused(validate_proxy_url(long(2_048), false)),
        authority_only
    );
    for url in ["http://proxy.example/?a=1", "http://proxy.example/#f"] {
        assert_eq!(
            refused(validate_proxy_url(url.to_string(), false)),
            authority_only,
            "{url}"
        );
    }
    assert_eq!(
        refused(validate_proxy_url(
            "socks5://proxy.example".to_string(),
            false
        )),
        "proxy URL must use HTTP or HTTPS and include a host"
    );
    for url in ["http://user@proxy.example", "http://:secret@proxy.example"] {
        assert_eq!(
            refused(validate_proxy_url(url.to_string(), false)),
            "proxy URL must not contain embedded credentials",
            "{url}"
        );
    }
    let accepted = validate_proxy_url("https://proxy.example:8443".to_string(), true)
        .expect("authority-only HTTPS proxy");
    assert_eq!(accepted.as_str(), "https://proxy.example:8443/");
}

#[test]
fn proxy_credentials_are_bounded_and_single_line() {
    let accepted = |username: &str, password: &str| validate_proxy_credentials(username, password);
    assert!(accepted(&"u".repeat(256), &"p".repeat(1_024)).is_ok());
    let size = "proxy credentials exceed the accepted size boundary";
    let line = "proxy credentials contain forbidden CR/LF characters";
    for (username, password, reason) in [
        (String::new(), "password".to_string(), size),
        ("u".repeat(257), "password".to_string(), size),
        ("user".to_string(), "p".repeat(1_025), size),
        ("us\ner".to_string(), "password".to_string(), line),
        ("user".to_string(), "pass\rword".to_string(), line),
    ] {
        match accepted(&username, &password) {
            Err(crate::ConnectError::InvalidConfiguration { reason: found, .. }) => {
                assert_eq!(found, reason, "{username:?} {password:?}");
            }
            other => panic!("expected {reason}, got {other:?}"),
        }
    }
}
