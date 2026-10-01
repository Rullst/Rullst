//! Interoperability contracts for generic OIDC issuers: optional profile
//! claims (OIDC Core 5.1).
use super::{RSA_KEY, RSA_N, claims};
use crate::client::{HttpClient, HttpRequest, HttpResponse};
use crate::provider::ExchangeParams;
use crate::providers::OidcProvider;
use crate::{ConnectError, Provider};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

const ISSUER: &str = "https://issuer.example";

/// A generic issuer whose ID token, userinfo body and JWK Set are chosen by
/// each test.
struct IssuerClient {
    id_token: Option<String>,
    userinfo: Value,
    jwks: Value,
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
