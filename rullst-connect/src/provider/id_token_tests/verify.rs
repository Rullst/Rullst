//! Audience-bound `verify_id_token` contracts for client-supplied ID tokens.
use super::{RSA_KEY, SignedClient, claims, issuer};
use crate::ConnectError;
use crate::providers::{GoogleProvider, OidcProvider};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn signed_client(issuer: &'static str) -> Arc<SignedClient> {
    Arc::new(SignedClient {
        issuer,
        id_token: None,
        userinfo_calls: AtomicUsize::new(0),
        token_forms: std::sync::Mutex::new(Vec::new()),
    })
}

fn sign(payload: &Value) -> String {
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some("audit-key".into());
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(RSA_KEY).unwrap();
    jsonwebtoken::encode(&header, payload, &key).unwrap()
}

async fn verify(
    kind: &str,
    id_token: &str,
    nonce: &str,
) -> Result<crate::ConnectUser, ConnectError> {
    let client = signed_client(issuer(kind));
    let result = if kind == "google" {
        GoogleProvider::try_new(
            "client-id",
            "test-secret".into(),
            "https://app.example/callback",
        )
        .unwrap()
        .with_http_client(client.clone())
        .verify_id_token(id_token, nonce)
        .await
    } else {
        OidcProvider::discover_with_client(
            issuer(kind),
            "client-id",
            "test-secret",
            "https://app.example/callback",
            client.clone(),
        )
        .await
        .unwrap()
        .verify_id_token(id_token, nonce)
        .await
    };
    assert_eq!(
        client.userinfo_calls.load(Ordering::SeqCst),
        0,
        "no userinfo"
    );
    assert!(
        client.token_forms.lock().unwrap().is_empty(),
        "no code exchange"
    );
    result
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn verify_id_token_binds_audience_nonce_and_issuer() {
    for kind in ["google", "oidc"] {
        let valid = sign(&claims(kind));
        let user = verify(kind, &valid, "challenge-nonce").await.unwrap();
        assert_eq!(user.id, "subject-1");
        assert_eq!(
            secrecy::ExposeSecret::expose_secret(&user.access_token),
            valid,
            "{kind} returns the verified ID token, not a provider access token"
        );
        assert!(user.refresh_token.is_none());

        assert!(
            verify(kind, &valid, "other-nonce").await.is_err(),
            "{kind} wrong nonce"
        );
        for (claim, value) in [
            ("aud", json!("another-app")),
            ("aud", json!(["client-id", "another-app"])),
            ("azp", json!("another-app")),
            ("iss", json!("https://other.example")),
            ("exp", json!(1)),
        ] {
            let mut payload = claims(kind);
            payload[claim] = value;
            assert!(
                verify(kind, &sign(&payload), "challenge-nonce")
                    .await
                    .is_err(),
                "{kind} accepted {claim}"
            );
        }
        let mut no_nonce = claims(kind);
        no_nonce.as_object_mut().unwrap().remove("nonce");
        assert!(
            verify(kind, &sign(&no_nonce), "challenge-nonce")
                .await
                .is_err()
        );
        let tampered = format!("{}x", &valid[..valid.len() - 1]);
        assert!(verify(kind, &tampered, "challenge-nonce").await.is_err());
    }
}

#[tokio::test]
async fn verify_id_token_rejects_empty_or_oversized_input() {
    for kind in ["google", "oidc"] {
        for (id_token, nonce) in [("", "nonce"), ("header.payload.signature", "")] {
            let error = verify(kind, id_token, nonce).await.unwrap_err();
            assert!(
                matches!(error, ConnectError::Token(_) | ConnectError::Provider(_)),
                "{error:?}"
            );
        }
        let oversized = "a".repeat(16 * 1024 + 1);
        assert!(verify(kind, &oversized, "nonce").await.is_err());
    }
}

#[tokio::test]
async fn verify_id_token_mock_credentials_stay_offline() {
    let google =
        GoogleProvider::try_new("mock_client", "".into(), "https://app.example/callback").unwrap();
    let user = google
        .verify_id_token("offline-token", "nonce")
        .await
        .unwrap();
    assert_eq!(user.id, "mock-user");
    let oidc = OidcProvider::discover(
        "https://issuer.example",
        "mock_client",
        "",
        "https://app.example/callback",
    )
    .await
    .unwrap();
    assert_eq!(
        oidc.verify_id_token("offline-token", "nonce")
            .await
            .unwrap()
            .id,
        "mock-user"
    );
}
