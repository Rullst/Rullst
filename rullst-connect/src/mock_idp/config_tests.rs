use super::*;

fn config(
    issuer: &str,
    client_id: &str,
    secret: &str,
    redirect: &str,
) -> Result<MockIdpConfig, crate::ConnectError> {
    MockIdpConfig::try_new(issuer, client_id, secret, redirect)
}

#[test]
fn a_validated_config_reports_its_endpoints_and_user() {
    let config = config(
        "http://localhost:9000",
        "local-client",
        "local-secret",
        "http://127.0.0.1:4000/auth/callback",
    )
    .expect("loopback config");
    assert_eq!(config.issuer(), "http://localhost:9000");
    assert_eq!(config.client_id(), "local-client");
    assert_eq!(config.redirect_uri(), "http://127.0.0.1:4000/auth/callback");

    let user = MockIdpUser::try_new("subject-7", "Ada", "ada@example.invalid")
        .expect("user")
        .with_picture("https://cdn.example.com/ada.png")
        .expect("HTTPS picture");
    assert_eq!(
        user.picture.as_deref(),
        Some("https://cdn.example.com/ada.png")
    );
    let config = config.with_user(user.clone());
    assert_eq!(config.user, user);
    assert_eq!(config.client_id(), "local-client");
}

#[test]
fn only_http_loopback_endpoints_are_accepted() {
    for (issuer, redirect) in [
        ("https://127.0.0.1:8080", "http://127.0.0.1:3000/callback"),
        ("http://127.0.0.1:8080", "https://localhost:3000/callback"),
    ] {
        assert!(
            config(issuer, "client", "secret", redirect).is_err(),
            "{issuer} {redirect}"
        );
    }
}

#[test]
fn credentials_and_profile_fields_are_bounded_without_controls() {
    let issuer = "http://127.0.0.1:8080";
    let redirect = "http://127.0.0.1:3000/callback";
    assert!(config(issuer, &"c".repeat(128), &"s".repeat(320), redirect).is_ok());
    for (client_id, secret) in [
        (String::new(), "secret".to_string()),
        ("c".repeat(129), "secret".to_string()),
        ("cli\u{7}ent".to_string(), "secret".to_string()),
        ("client".to_string(), "s".repeat(321)),
    ] {
        assert!(
            config(issuer, &client_id, &secret, redirect).is_err(),
            "{client_id:?} {secret:?}"
        );
    }
    assert!(MockIdpUser::try_new("s".repeat(128), "n".repeat(320), "a@example.invalid").is_ok());
    assert!(MockIdpUser::try_new("s".repeat(129), "Name", "a@example.invalid").is_err());
}
