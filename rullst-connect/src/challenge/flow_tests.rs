use super::*;
use crate::challenge::MemoryChallengeStore;
use crate::provider::ExchangeParams;
use crate::user::ConnectUser;
use async_trait::async_trait;
use std::sync::Mutex;

const NOW: u64 = 1_800_000_000;

/// One token exchange: code, PKCE verifier and expected nonce.
type Exchange = (String, Option<String>, Option<String>);

/// Records the exchange parameters it receives and returns a fixed user.
struct RecordingProvider {
    authorize: String,
    fail_exchange: bool,
    exchanges: Mutex<Vec<Exchange>>,
}

impl RecordingProvider {
    fn new(authorize: &str) -> Self {
        Self {
            authorize: authorize.to_string(),
            fail_exchange: false,
            exchanges: Mutex::new(Vec::new()),
        }
    }

    fn exchanges(&self) -> Vec<Exchange> {
        self.exchanges.lock().unwrap().clone()
    }
}

#[async_trait]
impl Provider for RecordingProvider {
    fn redirect_url(&self) -> String {
        self.authorize.clone()
    }

    async fn get_user(&self, params: ExchangeParams<'_>) -> Result<ConnectUser, ConnectError> {
        self.exchanges.lock().unwrap().push((
            params.auth_code.to_string(),
            params.code_verifier.map(str::to_string),
            params.expected_nonce.map(str::to_string),
        ));
        if self.fail_exchange {
            return Err(ConnectError::Token("exchange refused".to_string()));
        }
        Ok(ConnectUser {
            id: "subject-1".to_string(),
            name: "Ada".to_string(),
            email: Some("ada@example.com".to_string()),
            email_verified: Some(true),
            avatar_url: None,
            raw_data: serde_json::Value::Null,
            access_token: secrecy::SecretString::from(String::new()),
            refresh_token: None,
            expires_in: None,
        })
    }

    async fn get_user_from_token(&self, _access_token: &str) -> Result<ConnectUser, ConnectError> {
        Err(ConnectError::Token("not used".to_string()))
    }

    fn token_url(&self) -> String {
        "https://idp.example/token".to_string()
    }
}

fn query(url: &str, name: &str) -> Option<String> {
    url::Url::parse(url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

#[tokio::test]
async fn begin_stores_the_oidc_challenge_and_sends_only_public_values() {
    let store = MemoryChallengeStore::new(8).unwrap();
    let provider = RecordingProvider::new("https://idp.example/authorize?client_id=app");
    let options =
        LoginOptions::oidc("google").with_return_to(ReturnPath::parse("/courses/7").unwrap());
    let redirect = begin_login_at(&store, &provider, options, NOW)
        .await
        .unwrap();

    let url = redirect.url();
    assert!(url.starts_with("https://idp.example/authorize?client_id=app&"));
    let state = query(url, "state").unwrap();
    let nonce = query(url, "nonce").unwrap();
    let code_challenge = query(url, "code_challenge").unwrap();
    assert_eq!(query(url, "code_challenge_method").as_deref(), Some("S256"));
    assert_eq!(redirect.expires_at_unix_seconds(), NOW + 600);

    let stored = store.take(redirect.challenge_id(), NOW).await.unwrap();
    assert_eq!(stored.provider(), "google");
    assert_eq!(stored.state(), state);
    assert_eq!(stored.nonce(), Some(nonce.as_str()));
    assert_ne!(stored.nonce(), Some(stored.state()));
    assert!(crate::pkce::verify_pkce_challenge(
        stored.code_verifier(),
        &code_challenge
    ));
    assert!(!url.contains(stored.code_verifier()));
    assert_eq!(stored.return_to().unwrap().as_str(), "/courses/7");
    assert_eq!(stored.created_at(), NOW);
    assert_eq!(stored.expires_at(), NOW + 600);
    // Starting a login never talks to the token endpoint.
    assert!(provider.exchanges().is_empty());
}

#[tokio::test]
async fn an_oauth_login_has_no_nonce_and_a_custom_lifetime() {
    let store = MemoryChallengeStore::new(8).unwrap();
    let provider = RecordingProvider::new("https://idp.example/authorize");
    let options = LoginOptions::oauth("github").with_lifetime(Duration::from_secs(120));
    let redirect = begin_login_at(&store, &provider, options, NOW)
        .await
        .unwrap();
    assert_eq!(query(redirect.url(), "nonce"), None);
    assert_eq!(redirect.expires_at_unix_seconds(), NOW + 120);
    let cookie = ChallengeCookie::new("/auth/github", false).unwrap();
    assert_eq!(
        redirect.set_cookie(&cookie),
        format!(
            "rullst_oauth_challenge={}; Path=/auth/github; Max-Age=120; HttpOnly; SameSite=Lax",
            redirect.challenge_id().as_str()
        )
    );
    let stored = store.take(redirect.challenge_id(), NOW).await.unwrap();
    assert_eq!(stored.nonce(), None);
    assert_eq!(stored.provider(), "github");
    assert_eq!(stored.return_to(), None);
    assert_eq!(
        format!("{redirect:?}"),
        format!(
            "LoginRedirect {{ url: \"[REDACTED]\", challenge_id: ChallengeId(\"[REDACTED]\"), expires_at: {} }}",
            NOW + 120
        )
    );
}

#[tokio::test]
async fn begin_refuses_bad_options_and_unsafe_providers_without_storing() {
    let store = MemoryChallengeStore::new(1).unwrap();
    let provider = RecordingProvider::new("https://idp.example/authorize");
    for (options, expected) in [
        (LoginOptions::oidc("Google"), "provider name"),
        (
            LoginOptions::oidc("google").with_lifetime(Duration::ZERO),
            "lifetime",
        ),
        (
            LoginOptions::oidc("google").with_lifetime(Duration::from_secs(601)),
            "lifetime",
        ),
        (
            LoginOptions::oidc("google").with_lifetime(Duration::from_secs(u64::MAX)),
            "lifetime",
        ),
    ] {
        let error = begin_login_at(&store, &provider, options, NOW)
            .await
            .unwrap_err();
        assert!(
            matches!(error, LoginError::Challenge(ChallengeError::Invalid(reason)) if reason == expected),
            "{error:?}"
        );
    }
    for unsafe_url in [
        "http://idp.example/authorize",
        "https://idp.example/authorize?state=attacker",
        "https://idp.example/authorize#fragment",
    ] {
        let error = begin_login_at(
            &store,
            &RecordingProvider::new(unsafe_url),
            LoginOptions::oidc("google"),
            NOW,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(
                error,
                LoginError::Configuration(ConnectError::InvalidConfiguration {
                    field: "authorization_url",
                    ..
                })
            ),
            "{unsafe_url}: {error:?}"
        );
    }
    assert!(store.is_empty().unwrap());
    begin_login_at(&store, &provider, LoginOptions::oidc("google"), NOW)
        .await
        .unwrap();
    let full = begin_login_at(&store, &provider, LoginOptions::oidc("google"), NOW)
        .await
        .unwrap_err();
    assert!(matches!(
        full,
        LoginError::Challenge(ChallengeError::CapacityExceeded)
    ));
}

#[tokio::test]
async fn begin_login_uses_the_current_time() {
    let store = MemoryChallengeStore::new(8).unwrap();
    let provider = RecordingProvider::new("https://idp.example/authorize");
    let before = unix_now().unwrap();
    let redirect = begin_login(&store, &provider, LoginOptions::oauth("github"))
        .await
        .unwrap();
    let after = unix_now().unwrap();
    assert!(before > 1_767_225_599);
    assert!((before + 600..=after + 600).contains(&redirect.expires_at_unix_seconds()));
}

#[cfg(any(
    feature = "axum",
    feature = "actix",
    feature = "leptos",
    feature = "rullst"
))]
#[path = "flow_callback_tests.rs"]
mod finish;
