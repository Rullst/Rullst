//! Offline authorization-request contracts, independent of token-exchange mocks.

use crate::prelude::*;
use url::Url;

const CLIENT: &str = "offline-client+tag&field=encoded";
const REDIRECT: &str = "https://app.example.test/callback?next=%2Fcourse&view=login";
const STATE: &str = "csrf +&/=é";
const CHALLENGE: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
const SCOPES: [&str; 3] = ["openid", "email", "custom:read"];

fn secret() -> SecretString {
    SecretString::from("public-offline-test-value".to_owned())
}

fn assert_parameter(url: &Url, name: &str, expected: Option<&str>) {
    let actual: Vec<_> = url
        .query_pairs()
        .filter(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
        .collect();
    let expected: Vec<_> = expected.into_iter().map(str::to_owned).collect();
    assert_eq!(actual, expected, "parameter {name} at {}", url.path());
}

fn assert_authorization(raw: &str, endpoint: &str, state: Option<&str>, pkce: Option<&str>) {
    let url = Url::parse(raw).unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.fragment(), None);
    let mut base = url.clone();
    base.set_query(None);
    assert_eq!(base.as_str(), endpoint);
    assert_parameter(&url, "response_type", Some("code"));
    assert_parameter(&url, "client_id", Some(CLIENT));
    assert_parameter(&url, "redirect_uri", Some(REDIRECT));
    assert_parameter(&url, "scope", Some("openid email custom:read"));
    assert_parameter(&url, "state", state);
    assert_parameter(&url, "code_challenge", pkce);
    assert_parameter(&url, "code_challenge_method", pkce.map(|_| "S256"));
    assert_parameter(&url, "client_secret", None);
    assert_parameter(&url, "access_token", None);
    if url.host_str() == Some("appleid.apple.com") {
        assert_parameter(&url, "response_mode", Some("form_post"));
    }
}

fn assert_variants(provider: &impl Provider, endpoint: &str) {
    for (raw, state, pkce) in [
        (provider.redirect_url(), None, None),
        (provider.redirect_url_with_state(STATE), Some(STATE), None),
        (
            provider.redirect_url_with_pkce(CHALLENGE),
            None,
            Some(CHALLENGE),
        ),
        (
            provider.redirect_url_with_pkce_and_state(CHALLENGE, STATE),
            Some(STATE),
            Some(CHALLENGE),
        ),
    ] {
        assert_authorization(&raw, endpoint, state, pkce);
    }
}

macro_rules! provider_contract {
    ($name:ident, $constructor:expr, $endpoint:expr) => {
        #[test]
        fn $name() {
            let provider = ($constructor).with_scopes(&SCOPES);
            assert_variants(&provider, $endpoint);
            assert_authorization(
                &provider
                    .with_state(STATE)
                    .with_pkce(CHALLENGE)
                    .redirect_url(),
                $endpoint,
                Some(STATE),
                Some(CHALLENGE),
            );
        }
    };
}

provider_contract!(
    google,
    GoogleProvider::try_new(CLIENT, secret(), REDIRECT).unwrap(),
    "https://accounts.google.com/o/oauth2/v2/auth"
);
provider_contract!(
    microsoft,
    MicrosoftProvider::try_new(CLIENT, secret(), REDIRECT).unwrap(),
    "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
);
provider_contract!(
    discord,
    DiscordProvider::try_new(CLIENT, secret(), REDIRECT).unwrap(),
    "https://discord.com/api/oauth2/authorize"
);
provider_contract!(
    linkedin,
    LinkedinProvider::try_new(CLIENT, secret(), REDIRECT).unwrap(),
    "https://www.linkedin.com/oauth/v2/authorization"
);
provider_contract!(
    github,
    GithubProvider::try_new(CLIENT, secret(), REDIRECT).unwrap(),
    "https://github.com/login/oauth/authorize"
);
provider_contract!(
    facebook,
    FacebookProvider::try_new(CLIENT, secret(), REDIRECT).unwrap(),
    "https://www.facebook.com/v19.0/dialog/oauth"
);
provider_contract!(
    x,
    XProvider::try_new(CLIENT, secret(), REDIRECT).unwrap(),
    "https://twitter.com/i/oauth2/authorize"
);
provider_contract!(
    cognito,
    CognitoProvider::try_new(CLIENT, secret(), REDIRECT, "https://cognito.example.test").unwrap(),
    "https://cognito.example.test/oauth2/authorize"
);
provider_contract!(
    auth0,
    Auth0Provider::try_new(CLIENT, secret(), REDIRECT, "auth0.example.test").unwrap(),
    "https://auth0.example.test/authorize"
);
provider_contract!(
    apple,
    AppleProvider::try_new(
        CLIENT,
        "offline-team",
        "offline-key-id",
        "unused-key",
        REDIRECT
    )
    .unwrap(),
    "https://appleid.apple.com/auth/authorize"
);

#[tokio::test]
async fn discovered_oidc_keeps_a_single_code_response_type() {
    use crate::client::{HttpClient, HttpRequest, HttpResponse};
    use std::sync::{Arc, atomic::AtomicUsize, atomic::Ordering};

    #[derive(Default)]
    struct DiscoveryOnly(AtomicUsize);

    #[async_trait::async_trait]
    impl HttpClient for DiscoveryOnly {
        async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, ConnectError> {
            assert_eq!(request.method, "GET");
            assert_eq!(
                request.url,
                "https://issuer.example.test/.well-known/openid-configuration"
            );
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(HttpResponse {
                status: 200,
                body: serde_json::json!({
                    "issuer": "https://issuer.example.test",
                    "authorization_endpoint": "https://issuer.example.test/authorize",
                    "token_endpoint": "https://issuer.example.test/token",
                    "userinfo_endpoint": "https://issuer.example.test/userinfo",
                    "jwks_uri": "https://issuer.example.test/jwks"
                }),
            })
        }
    }

    let client = Arc::new(DiscoveryOnly::default());
    let provider = OidcProvider::discover_with_client(
        "https://issuer.example.test",
        CLIENT,
        "public-offline-test-value",
        REDIRECT,
        client.clone(),
    )
    .await
    .unwrap()
    .with_scopes(&SCOPES);
    let endpoint = "https://issuer.example.test/authorize";
    assert_variants(&provider, endpoint);
    assert_authorization(
        &provider
            .with_state(STATE)
            .with_pkce(CHALLENGE)
            .redirect_url(),
        endpoint,
        Some(STATE),
        Some(CHALLENGE),
    );
    assert_eq!(client.0.load(Ordering::SeqCst), 1);
}

#[test]
fn generic_parameter_helper_remains_response_type_neutral() {
    let mut params = super::build_oauth_params(
        "https://custom.example.test/authorize?tenant=local",
        CLIENT,
        REDIRECT,
        "read",
        None,
        None,
    );
    params.append_pair("response_type", "custom");
    let url = Url::parse(&params.finish()).unwrap();
    assert_parameter(&url, "tenant", Some("local"));
    assert_parameter(&url, "response_type", Some("custom"));
}

#[test]
fn offline_redirects_preserve_their_local_contract() {
    let google = GoogleProvider::try_new("mock_google", secret(), REDIRECT).unwrap();
    let cognito = CognitoProvider::try_new(
        "mock_cognito",
        secret(),
        REDIRECT,
        "https://cognito.example.test",
    )
    .unwrap();
    for raw in [
        google.redirect_url_with_pkce_and_state(CHALLENGE, STATE),
        cognito.redirect_url_with_pkce_and_state(CHALLENGE, STATE),
    ] {
        let url = Url::parse(&raw).unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("example.invalid"));
        assert_eq!(url.path(), "/rullst-connect/mock");
        assert_parameter(&url, "response_type", None);
        assert_parameter(&url, "state", Some(STATE));
        assert_parameter(&url, "code_challenge", Some(CHALLENGE));
        assert_parameter(&url, "code_challenge_method", Some("S256"));
    }
}
