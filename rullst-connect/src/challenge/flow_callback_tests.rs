use super::*;
use crate::challenge::flow::callback::finish_login_at;
use crate::challenge::{CompletedLogin, finish_login};
use crate::extractors::AuthCallback;

struct Started {
    store: MemoryChallengeStore,
    provider: RecordingProvider,
    id: String,
    state: String,
    nonce: String,
    verifier: String,
}

async fn started(provider: RecordingProvider) -> Started {
    let store = MemoryChallengeStore::new(8).unwrap();
    let options =
        LoginOptions::oidc("google").with_return_to(ReturnPath::parse("/courses/7").unwrap());
    let redirect = begin_login_at(&store, &provider, options, NOW)
        .await
        .unwrap();
    let state = query(redirect.url(), "state").unwrap();
    let nonce = query(redirect.url(), "nonce").unwrap();
    // Read the verifier back, then store the same challenge under a new id.
    let stored = store.take(redirect.challenge_id(), NOW).await.unwrap();
    let verifier = stored.code_verifier().to_string();
    let id = store.save(&stored).await.unwrap();
    Started {
        store,
        provider,
        id: id.as_str().to_string(),
        state,
        nonce,
        verifier,
    }
}

fn callback(code: Option<&str>, state: Option<&str>, error: Option<&str>) -> AuthCallback {
    AuthCallback {
        code: code.map(str::to_string),
        state: state.map(str::to_string),
        error: error.map(str::to_string),
        error_description: None,
    }
}

async fn finish(
    started: &Started,
    provider_name: &str,
    id: Option<&str>,
    callback: &AuthCallback,
    now: u64,
) -> Result<CompletedLogin, LoginError> {
    finish_login_at(
        &started.store,
        &started.provider,
        provider_name,
        id,
        callback,
        now,
    )
    .await
}

#[tokio::test]
async fn a_matching_callback_signs_in_with_the_stored_verifier_and_nonce() {
    let flow = started(RecordingProvider::new("https://idp.example/authorize")).await;
    let ok = callback(Some("code-1"), Some(&flow.state), None);
    let login = finish(&flow, "google", Some(&flow.id), &ok, NOW + 599)
        .await
        .unwrap();
    assert_eq!(login.user().id, "subject-1");
    assert_eq!(login.return_to().unwrap().as_str(), "/courses/7");
    assert_eq!(
        flow.provider.exchanges(),
        [(
            "code-1".to_string(),
            Some(flow.verifier.clone()),
            Some(flow.nonce.clone())
        )]
    );
    let (user, return_to) = login.into_parts();
    assert_eq!(user.email.as_deref(), Some("ada@example.com"));
    assert_eq!(return_to.unwrap().as_str(), "/courses/7");
}

#[tokio::test]
async fn a_replayed_callback_finds_nothing() {
    let flow = started(RecordingProvider::new("https://idp.example/authorize")).await;
    let ok = callback(Some("code-1"), Some(&flow.state), None);
    finish(&flow, "google", Some(&flow.id), &ok, NOW)
        .await
        .unwrap();
    let replay = finish(&flow, "google", Some(&flow.id), &ok, NOW)
        .await
        .unwrap_err();
    assert!(matches!(
        replay,
        LoginError::Challenge(ChallengeError::NotFound)
    ));
    assert_eq!(flow.provider.exchanges().len(), 1);
}

#[tokio::test]
async fn missing_and_altered_cookies_leave_the_challenge_in_place() {
    let flow = started(RecordingProvider::new("https://idp.example/authorize")).await;
    let ok = callback(Some("code-1"), Some(&flow.state), None);
    let missing = finish(&flow, "google", None, &ok, NOW).await.unwrap_err();
    assert!(matches!(missing, LoginError::MissingChallenge));
    let mut altered = flow.id.clone();
    let last = if altered.ends_with('A') { "Q" } else { "A" };
    altered.replace_range(42.., last);
    for tampered in [altered.as_str(), "not-a-challenge-id", &flow.id[1..]] {
        let error = finish(&flow, "google", Some(tampered), &ok, NOW)
            .await
            .unwrap_err();
        assert!(
            matches!(error, LoginError::Challenge(ChallengeError::NotFound)),
            "{error:?}"
        );
    }
    assert!(flow.provider.exchanges().is_empty());
    assert_eq!(flow.store.len().unwrap(), 1);
    finish(&flow, "google", Some(&flow.id), &ok, NOW)
        .await
        .unwrap();
}

#[tokio::test]
async fn an_expired_challenge_is_refused() {
    let flow = started(RecordingProvider::new("https://idp.example/authorize")).await;
    let ok = callback(Some("code-1"), Some(&flow.state), None);
    let expired = finish(&flow, "google", Some(&flow.id), &ok, NOW + 600)
        .await
        .unwrap_err();
    assert!(matches!(
        expired,
        LoginError::Challenge(ChallengeError::Expired)
    ));
    assert!(flow.provider.exchanges().is_empty());
}

#[tokio::test]
async fn a_wrong_or_missing_state_consumes_the_challenge() {
    for state in [Some("s1234567890123456789012345678901"), None] {
        let flow = started(RecordingProvider::new("https://idp.example/authorize")).await;
        let wrong = callback(Some("code-1"), state, None);
        let error = finish(&flow, "google", Some(&flow.id), &wrong, NOW)
            .await
            .unwrap_err();
        assert!(matches!(error, LoginError::StateMismatch), "{error:?}");
        let ok = callback(Some("code-1"), Some(&flow.state), None);
        let retry = finish(&flow, "google", Some(&flow.id), &ok, NOW)
            .await
            .unwrap_err();
        assert!(matches!(
            retry,
            LoginError::Challenge(ChallengeError::NotFound)
        ));
        assert!(flow.provider.exchanges().is_empty());
    }
}

#[tokio::test]
async fn a_challenge_for_another_provider_is_refused() {
    let flow = started(RecordingProvider::new("https://idp.example/authorize")).await;
    let ok = callback(Some("code-1"), Some(&flow.state), None);
    let error = finish(&flow, "github", Some(&flow.id), &ok, NOW)
        .await
        .unwrap_err();
    assert!(matches!(error, LoginError::ProviderMismatch));
    assert!(flow.provider.exchanges().is_empty());
}

#[tokio::test]
async fn provider_errors_are_trusted_only_with_the_right_state() {
    let flow = started(RecordingProvider::new("https://idp.example/authorize")).await;
    let forged = callback(
        None,
        Some("s1234567890123456789012345678901"),
        Some("access_denied"),
    );
    let error = finish(&flow, "google", Some(&flow.id), &forged, NOW)
        .await
        .unwrap_err();
    assert!(matches!(error, LoginError::StateMismatch));

    let flow = started(RecordingProvider::new("https://idp.example/authorize")).await;
    let denied = callback(
        Some("code-1"),
        Some(&flow.state),
        Some("access_denied<script>\r\n"),
    );
    let error = finish(&flow, "google", Some(&flow.id), &denied, NOW)
        .await
        .unwrap_err();
    assert!(
        matches!(&error, LoginError::Denied(code) if code == "access_deniedscript"),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "the provider refused the authorization: access_deniedscript"
    );
    assert!(flow.provider.exchanges().is_empty());

    let flow = started(RecordingProvider::new("https://idp.example/authorize")).await;
    let long = format!("{}{}", "a.b-c_".repeat(11), "z".repeat(10));
    let denied = callback(None, Some(&flow.state), Some(&long));
    let error = finish(&flow, "google", Some(&flow.id), &denied, NOW)
        .await
        .unwrap_err();
    assert!(
        matches!(&error, LoginError::Denied(code) if *code == long[..64]),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_callback_without_a_code_is_refused() {
    for code in [None, Some("")] {
        let flow = started(RecordingProvider::new("https://idp.example/authorize")).await;
        let empty = callback(code, Some(&flow.state), None);
        let error = finish(&flow, "google", Some(&flow.id), &empty, NOW)
            .await
            .unwrap_err();
        assert!(matches!(error, LoginError::MissingCode), "{code:?}");
        assert!(flow.provider.exchanges().is_empty());
    }
}

#[tokio::test]
async fn exchange_failures_are_reported_after_consuming_the_challenge() {
    let mut provider = RecordingProvider::new("https://idp.example/authorize");
    provider.fail_exchange = true;
    let flow = started(provider).await;
    let ok = callback(Some("code-1"), Some(&flow.state), None);
    let error = finish(&flow, "google", Some(&flow.id), &ok, NOW)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        LoginError::Exchange(ConnectError::Token(_))
    ));
    assert_eq!(flow.store.len().unwrap(), 0);
}

#[tokio::test]
async fn finish_login_uses_the_current_time() {
    let store = MemoryChallengeStore::new(8).unwrap();
    let provider = RecordingProvider::new("https://idp.example/authorize");
    let now = unix_now().unwrap();
    let state = "s1234567890123456789012345678901";
    let verifier = "v123456789012345678901234567890123456789012";
    let lapsed =
        LoginChallenge::new("google", state, None, verifier, None, now - 700, now - 100).unwrap();
    let id = store.save(&lapsed).await.unwrap();
    let ok = callback(Some("code-1"), Some(state), None);
    let error = finish_login(&store, &provider, "google", Some(id.as_str()), &ok)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        LoginError::Challenge(ChallengeError::Expired)
    ));

    let live = LoginChallenge::new("google", state, None, verifier, None, now, now + 600).unwrap();
    let id = store.save(&live).await.unwrap();
    let login = finish_login(&store, &provider, "google", Some(id.as_str()), &ok)
        .await
        .unwrap();
    assert_eq!(login.user().id, "subject-1");
    assert_eq!(
        provider.exchanges(),
        [("code-1".to_string(), Some(verifier.to_string()), None)]
    );
}

#[tokio::test]
async fn the_offline_mock_runs_the_whole_flow_through_the_configured_callback() {
    let provider = crate::providers::GoogleProvider::try_new(
        "mock_google_client",
        secrecy::SecretString::from("mock_google_secret".to_string()),
        "http://localhost:3000/auth/google/callback",
    )
    .unwrap();
    let store = MemoryChallengeStore::new(8).unwrap();
    let redirect = begin_login(&store, &provider, LoginOptions::oidc("google"))
        .await
        .unwrap();
    // The mock "provider" sends the browser straight back to the callback.
    let url = url::Url::parse(redirect.url()).unwrap();
    assert_eq!(
        url[..url::Position::AfterPath].to_string(),
        "http://localhost:3000/auth/google/callback"
    );
    let callback: AuthCallback = serde_urlencoded::from_str(url.query().unwrap()).unwrap();
    assert_eq!(callback.code.as_deref(), Some("mock_authorization_code"));
    let login = finish_login(
        &store,
        &provider,
        "google",
        Some(redirect.challenge_id().as_str()),
        &callback,
    )
    .await
    .unwrap();
    assert_eq!(login.user().id, "mock-user");
    assert_eq!(login.user().email.as_deref(), Some("mock@example.invalid"));
    assert_eq!(login.user().email_verified, Some(true));
    assert!(store.is_empty().unwrap());
}
