//! Interoperability contracts for generic OIDC issuers: optional profile
//! claims (OIDC Core 5.1) and ID tokens without `kid` (OIDC Core 10.1).
use super::{RSA_KEY, RSA_N, claims};
use crate::client::{HttpClient, HttpRequest, HttpResponse};
use crate::provider::ExchangeParams;
use crate::providers::OidcProvider;
use crate::{ConnectError, Provider};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const ISSUER: &str = "https://issuer.example";

/// A generic issuer whose ID token, userinfo body and JWK Set are chosen by
/// each test.
struct IssuerClient {
    id_token: Option<String>,
    userinfo: Value,
    jwks: Value,
    jwks_calls: AtomicUsize,
}

#[async_trait]
impl HttpClient for IssuerClient {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, ConnectError> {
        let body = if req.url.contains("openid-configuration") {
            json!({
                "issuer": ISSUER,
                "authorization_endpoint": "https://issuer.example/authorize",
                "token_endpoint": "https://issuer.example/token",
                "userinfo_endpoint": "https://issuer.example/userinfo",
                "jwks_uri": "https://issuer.example/jwks"
            })
        } else if req.url.ends_with("/token") {
            let mut body = json!({
                "access_token": "opaque-access-token",
                "expires_in": 3600,
                "refresh_token": "rotated-refresh-token"
            });
            if let Some(token) = &self.id_token {
                body["id_token"] = json!(token);
            }
            body
        } else if req.url.ends_with("/jwks") {
            self.jwks_calls.fetch_add(1, Ordering::SeqCst);
            self.jwks.clone()
        } else {
            self.userinfo.clone()
        };
        Ok(HttpResponse { status: 200, body })
    }
}

fn rsa_jwk(extra: Value) -> Value {
    let mut jwk = json!({"kty": "RSA", "n": RSA_N, "e": "AQAB"});
    if let (Some(jwk), Some(extra)) = (jwk.as_object_mut(), extra.as_object()) {
        jwk.extend(extra.clone());
    }
    jwk
}

fn sign(kid: Option<&str>, payload: &Value) -> String {
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = kid.map(str::to_owned);
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(RSA_KEY).unwrap();
    jsonwebtoken::encode(&header, payload, &key).unwrap()
}

fn client(id_token: Option<String>, userinfo: Value, jwks: Value) -> Arc<IssuerClient> {
    Arc::new(IssuerClient {
        id_token,
        userinfo,
        jwks,
        jwks_calls: AtomicUsize::new(0),
    })
}

async fn oidc(client: Arc<IssuerClient>) -> OidcProvider {
    OidcProvider::discover_with_client(
        ISSUER,
        "client-id",
        "test-secret",
        "https://app.example/callback",
        client,
    )
    .await
    .unwrap()
}

/// Valid ID-token claims for this issuer with `name` replaced by `profile`.
fn claims_with_profile(profile: Value) -> Value {
    let mut payload = claims("oidc");
    let object = payload.as_object_mut().unwrap();
    object.remove("name");
    if let Some(profile) = profile.as_object() {
        object.extend(profile.clone());
    }
    payload
}

fn keyed_jwks() -> Value {
    json!({"keys": [rsa_jwk(json!({"kid": "audit-key", "alg": "RS256", "use": "sig"}))]})
}

async fn login(payload: &Value) -> Result<crate::ConnectUser, ConnectError> {
    let provider = oidc(client(
        Some(sign(Some("audit-key"), payload)),
        json!({}),
        keyed_jwks(),
    ))
    .await;
    provider
        .get_user(ExchangeParams {
            auth_code: "code",
            expected_nonce: Some("challenge-nonce"),
            ..Default::default()
        })
        .await
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn id_tokens_without_a_name_claim_sign_in_with_a_display_name_fallback() {
    for (profile, expected) in [
        (json!({"name": "Ada Lovelace"}), "Ada Lovelace"),
        (
            json!({"given_name": "Ada", "family_name": "Lovelace", "preferred_username": "ada"}),
            "Ada Lovelace",
        ),
        (json!({"family_name": "Lovelace"}), "Lovelace"),
        (
            json!({"name": " ", "preferred_username": "ada", "nickname": "countess"}),
            "ada",
        ),
        (json!({"nickname": "countess"}), "countess"),
        // Keycloak/Cognito-style tokens with only identity and email claims.
        (json!({"email": "ada@example.invalid"}), ""),
        (json!({}), ""),
    ] {
        let user = login(&claims_with_profile(profile.clone()))
            .await
            .unwrap_or_else(|_| panic!("{profile} was rejected"));
        assert_eq!(user.id, "subject-1");
        assert_eq!(user.name, expected, "{profile}");
    }
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn refresh_and_verify_id_token_accept_a_token_without_a_name_claim() {
    let payload = claims_with_profile(json!({"preferred_username": "ada"}));
    let id_token = sign(Some("audit-key"), &payload);
    let provider = oidc(client(Some(id_token.clone()), json!({}), keyed_jwks())).await;

    let refreshed = provider.refresh_token("refresh-token").await.unwrap();
    assert_eq!(refreshed.name, "ada");

    let verified = provider
        .verify_id_token(&id_token, "challenge-nonce")
        .await
        .unwrap();
    assert_eq!(verified.name, "ada");
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn userinfo_without_a_name_claim_still_returns_the_profile() {
    let provider = oidc(client(
        None,
        json!({"sub": "subject-1", "preferred_username": "ada", "email": "ada@example.invalid"}),
        keyed_jwks(),
    ))
    .await;
    let user = provider.get_user_from_token("access-token").await.unwrap();
    assert_eq!(user.id, "subject-1");
    assert_eq!(user.name, "ada");
    assert_eq!(user.email.as_deref(), Some("ada@example.invalid"));

    let user = provider
        .get_user(ExchangeParams {
            auth_code: "code",
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(user.name, "ada");
}

async fn kidless_login(id_token: String, jwks: Value) -> Result<crate::ConnectUser, ConnectError> {
    let provider = oidc(client(Some(id_token), json!({}), jwks)).await;
    provider
        .get_user(ExchangeParams {
            auth_code: "code",
            expected_nonce: Some("challenge-nonce"),
            ..Default::default()
        })
        .await
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn kidless_id_tokens_verify_against_a_single_key_set() {
    let payload = claims("oidc");
    for key in [
        rsa_jwk(json!({})),
        rsa_jwk(json!({"kid": "only-key", "alg": "RS256", "use": "sig"})),
        rsa_jwk(json!({"key_ops": ["verify"]})),
    ] {
        let jwks = json!({"keys": [key]});
        let user = kidless_login(sign(None, &payload), jwks.clone())
            .await
            .unwrap_or_else(|_| panic!("{jwks} was rejected"));
        assert_eq!(user.id, "subject-1");

        let provider = oidc(client(None, json!({}), jwks)).await;
        let verified = provider
            .verify_id_token(&sign(None, &payload), "challenge-nonce")
            .await
            .unwrap();
        assert_eq!(verified.id, "subject-1");
    }
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn kidless_id_tokens_fail_closed_unless_the_only_key_fits() {
    let id_token = sign(None, &claims("oidc"));
    let ec_p256 = json!({
        "kty": "EC", "crv": "P-256", "use": "sig",
        "x": "XCdoM3BtXvKrn4yqzlkEpguDqEJAFJP08-3PEuN7ahc",
        "y": "RUr-Y0Ljrbzzk2NnK8lbblORZB9V6DwAicyVDmKBRlU"
    });
    for (case, jwks) in [
        ("empty set", json!({"keys": []})),
        (
            "two keys",
            json!({"keys": [
                rsa_jwk(json!({"kid": "a"})),
                rsa_jwk(json!({"kid": "b"}))
            ]}),
        ),
        (
            "signature key beside an encryption key",
            json!({"keys": [
                rsa_jwk(json!({"use": "sig"})),
                rsa_jwk(json!({"use": "enc"}))
            ]}),
        ),
        (
            "encryption key",
            json!({"keys": [rsa_jwk(json!({"use": "enc"}))]}),
        ),
        (
            "encrypt-only key_ops",
            json!({"keys": [rsa_jwk(json!({"key_ops": ["encrypt"]}))]}),
        ),
        (
            "different declared alg",
            json!({"keys": [rsa_jwk(json!({"alg": "RS512"}))]}),
        ),
        (
            "declared PS256",
            json!({"keys": [rsa_jwk(json!({"alg": "PS256"}))]}),
        ),
        ("EC key for an RS256 token", json!({"keys": [ec_p256]})),
    ] {
        let error = kidless_login(id_token.clone(), jwks).await.err();
        assert!(
            matches!(error, Some(ConnectError::Provider(ref message)) if message.contains("no 'kid' header")),
            "{case} was not rejected by key selection"
        );
    }
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn kidless_id_tokens_still_need_a_valid_signature_and_asymmetric_alg() {
    let jwks = json!({"keys": [rsa_jwk(json!({}))]});
    let genuine = sign(None, &claims("oidc"));
    let mut other_claims = claims("oidc");
    other_claims["sub"] = json!("someone-else");
    let other = sign(None, &other_claims);
    let genuine_parts: Vec<&str> = genuine.split('.').collect();
    let other_parts: Vec<&str> = other.split('.').collect();
    let forged = format!(
        "{}.{}.{}",
        genuine_parts[0], other_parts[1], genuine_parts[2]
    );
    let error = kidless_login(forged, jwks.clone()).await.err();
    assert!(matches!(
        error,
        Some(ConnectError::Provider(ref message)) if message.contains("signature or claims validation failed")
    ));

    let symmetric = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &claims("oidc"),
        &jsonwebtoken::EncodingKey::from_secret(b"guessable-shared-secret"),
    )
    .unwrap();
    let issuer = client(Some(symmetric), json!({}), jwks);
    let provider = oidc(issuer.clone()).await;
    let error = provider
        .get_user(ExchangeParams {
            auth_code: "code",
            expected_nonce: Some("challenge-nonce"),
            ..Default::default()
        })
        .await
        .err();
    assert!(matches!(
        error,
        Some(ConnectError::Provider(ref message)) if message.contains("symmetric algorithm")
    ));
    assert_eq!(issuer.jwks_calls.load(Ordering::SeqCst), 0);
}
