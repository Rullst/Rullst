//! Signed, network-free regression contracts for the real OIDC verification paths.
use super::ExchangeParams;
use crate::client::{HttpClient, HttpRequest, HttpResponse};
use crate::providers::{AppleProvider, GoogleProvider, OidcProvider};
use crate::{ConnectError, Provider};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

// Public, test-only fixture already used by the provider unit tests.
const RSA_KEY: &[u8] = b"-----BEGIN PRIVATE KEY-----\n\
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDUJ5CJ1uq6Fa6u\n\
D2dVnj5f4QYm6xN5oiFxJeISHSMIfL8cQQtOwPpW73VomiiuUw0f7Qa6ermrLe+z\n\
oHQLv003Sx4PUL68wxObVFXQPFInUwbLHbqDNzZt7idEtrK3n0Bytjol7n/g3CRf\n\
0icl0kYcO1YbswfJvmJzjOKXVesK9s5CgugaL6dR7TIdJ8gboICYAf5k7T9pPAMV\n\
ELz5yEM2SajzQhiU1K9Y0bH0wzBC1aRpBVLU5z9iG7AO4EJ5CGyi5IQcJciYgp1H\n\
GCFWKGY2NOQ2besmWUFOPHJe+EZEixq87cXod8Llx5nZtqYoPlomgYciHEj9jpFF\n\
nINRvvK5AgMBAAECggEAa4AAxVeZaOFDuf8kJUYh5QNo0p+bJq74sxS3EOaisdJE\n\
JFTxHd66+wIrQ+2ZX3vF0r+QAT3ehtan9yT+qFUvEy2E9c28WHmgnbyGHxXxqutv\n\
LczKjWKUue9LBo5s0I5pYbbkkAPh2Fa0N7mNDKUX0YZfg3mcIKXPzS0+Q+DNUKgC\n\
d38wKKLBKlqLbsPU3KVkL2hoTnZTp85NoXxbnMCrtrodjDrEjdcgvVeWeTVFqO/q\n\
haA11bPIKxu5Gll9zCy9VkdRdHjNkEI9Qcld6Lw2MTU/bDwvuQGsr6IxAz2or+Ib\n\
tzEJJDNXir3DP2iCdyUaBabo5cnYy6AAQ6gxuFYEAQKBgQDsjzVHA9Bbrhw2XGZ/\n\
2sZIV4GxKuGuNWWm/CMAh7u34JnO2xKj5kSAi3rEkFAiWEclFZlxVlnYNx8dYdh4\n\
aM7BMw0cSrzKSc7Xa2eAjQ0K2p+vx+u2QhJy/qVRESG0egmcsGWAwrdBPGpqKHYH\n\
wAki3dqzMlRgZMhCgtBpxISQ8QKBgQDlluvhyFW48d1F7yNQBTLGLwM7uHurnMms\n\
09c/jSbQGn+nOlli+jTLyMX4DjQIz6+2O5hR0KALWNcq1dZTy3EKFHgvqtkJF4Hm\n\
RDwCRcqDZxbFx+ewIj79ycVhB+mYiD+pJcf5f1Ogc1w3DZuhzTrRsGBo5lLtjM3d\n\
AoAzskt+SQKBgC+szPf69MsFU/pAtQefd8asnB6wnbsWV95HgmZg9JwiT904mZEe\n\
nz+o3J0w2HWThQMcT0hgNss0kLjDN3VM6h5Vw5aoGVRLe7w+kSV/R9mgJf6vM/oP\n\
Zth2Kask4L4Wukkx48MHexdSrb+nV+JH+Y9lVuY2hnrG1PVSl945FN6BAoGBAK98\n\
bS31/7fOfzBOOjKW1plvI8yJFVY2EFzeyz8TN+CG8J201s/1qVc+TjttN86oWIk1\n\
Ahc/HKWvsT9XlWwVK4Dl5nug3iW55xtHeorOJ53KtThVtT0G4BkCGbEx6BYjxm0W\n\
qMSG0zfoFUsrRpMlGFlgtEBaFHboUg4lNDLPjC6pAoGAFpXBekgzkUMfTpDQGJNX\n\
8aFTRTMSQKuPPq3B4UeJI+tVlSUdHCXIa7oGBTLYIWMCgWjpbDaU/Fkczljlk6d2\n\
TxJdI1XtSKNCfaAPkAND44lK1zdpnnImQQA8/r7ohOSUTfMT98q2atLHRbmldnwg\n\
z1F4IZ42Gry2+4guKvvM+O8=\n\
-----END PRIVATE KEY-----";
const RSA_N: &str = "1CeQidbquhWurg9nVZ4-X-EGJusTeaIhcSXiEh0jCHy_HEELTsD6Vu91aJoorlMNH-0Gunq5qy3vs6B0C79NN0seD1C-vMMTm1RV0DxSJ1MGyx26gzc2be4nRLayt59AcrY6Je5_4NwkX9InJdJGHDtWG7MHyb5ic4zil1XrCvbOQoLoGi-nUe0yHSfIG6CAmAH-ZO0_aTwDFRC8-chDNkmo80IYlNSvWNGx9MMwQtWkaQVS1Oc_YhuwDuBCeQhsouSEHCXImIKdRxghVihmNjTkNm3rJllBTjxyXvhGRIsavO3F6HfC5ceZ2bamKD5aJoGHIhxI_Y6RRZyDUb7yuQ";
const EC_KEY: &str = "-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgkdn4ngP0MJj/+G/Z\n0FgfmUYbc26Oidgl0NZoUXoMm6KhRANCAARcJ2gzcG1e8qufjKrOWQSmC4OoQkAU\nk/Tz7c8S43tqF0VK/mNC462881k2cryVtuV5FkH1XoPACJzJUQ5igUZV\n-----END PRIVATE KEY-----";

struct SignedClient {
    issuer: &'static str,
    id_token: Option<String>,
    userinfo_calls: AtomicUsize,
    token_forms: std::sync::Mutex<Vec<String>>,
}
#[async_trait]
impl HttpClient for SignedClient {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, ConnectError> {
        let body = if req.url.contains("openid-configuration") {
            json!({
                "issuer": self.issuer,
                "authorization_endpoint": "https://issuer.example/authorize",
                "token_endpoint": "https://issuer.example/token",
                "userinfo_endpoint": "https://issuer.example/userinfo",
                "jwks_uri": "https://issuer.example/jwks"
            })
        } else if req.url.ends_with("/token") {
            self.token_forms
                .lock()
                .unwrap()
                .push(req.form.unwrap_or_default());
            let mut body = json!({
                "access_token":"opaque-access-token", "expires_in":3600,
                "refresh_token":"rotated-refresh-token"
            });
            if let Some(token) = &self.id_token {
                body["id_token"] = json!(token);
            }
            body
        } else if req.url.ends_with("/jwks")
            || req.url.ends_with("/certs")
            || req.url.ends_with("/keys")
        {
            json!({"keys":[{"kid":"audit-key", "kty":"RSA", "alg":"RS256", "use":"sig", "n":RSA_N, "e":"AQAB"}]})
        } else {
            self.userinfo_calls.fetch_add(1, Ordering::SeqCst);
            json!({"sub":"subject-1","name":"User","email":"a@example.invalid"})
        };
        Ok(HttpResponse { status: 200, body })
    }
}

fn issuer(kind: &str) -> &'static str {
    match kind {
        "google" => "https://accounts.google.com",
        "apple" => "https://appleid.apple.com",
        _ => "https://issuer.example",
    }
}
fn claims(kind: &str) -> Value {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    json!({"iss":issuer(kind), "aud":"client-id", "sub":"subject-1", "iat":now,
        "exp":now+3600, "nonce":"challenge-nonce", "name":"User"})
}
async fn provider(kind: &str, payload: Option<&Value>) -> (Box<dyn Provider>, Arc<SignedClient>) {
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some("audit-key".into());
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(RSA_KEY).unwrap();
    let client = Arc::new(SignedClient {
        issuer: issuer(kind),
        id_token: payload.map(|claims| jsonwebtoken::encode(&header, claims, &key).unwrap()),
        userinfo_calls: AtomicUsize::new(0),
        token_forms: std::sync::Mutex::new(Vec::new()),
    });
    let provider: Box<dyn Provider> = match kind {
        "google" => Box::new(
            GoogleProvider::try_new(
                "client-id",
                "test-secret".into(),
                "https://app.example/callback",
            )
            .unwrap()
            .with_http_client(client.clone()),
        ),
        "apple" => Box::new(
            AppleProvider::try_new(
                "client-id",
                "team-id",
                "key-id",
                EC_KEY,
                "https://app.example/callback",
            )
            .unwrap()
            .with_http_client(client.clone()),
        ),
        _ => Box::new(
            OidcProvider::discover_with_client(
                issuer(kind),
                "client-id",
                "test-secret",
                "https://app.example/callback",
                client.clone(),
            )
            .await
            .unwrap(),
        ),
    };
    (provider, client)
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn signed_id_tokens_require_expiration_even_with_a_nonce() {
    for kind in ["google", "apple", "oidc"] {
        let mut payload = claims(kind);
        payload.as_object_mut().unwrap().remove("exp");
        let (provider, _) = provider(kind, Some(&payload)).await;
        assert!(
            provider
                .get_user(ExchangeParams {
                    auth_code: "code",
                    expected_nonce: Some("challenge-nonce"),
                    ..Default::default()
                })
                .await
                .is_err(),
            "{kind} accepted missing exp"
        );
    }
}
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn signed_id_tokens_require_identity_claims_and_authorized_party() {
    for kind in ["google", "apple", "oidc"] {
        let valid = claims(kind);
        let mut invalid = Vec::new();
        for claim in ["iss", "aud", "sub", "iat"] {
            let mut payload = valid.clone();
            payload.as_object_mut().unwrap().remove(claim);
            invalid.push((claim, payload));
        }
        for (name, claim, value) in [
            ("empty subject", "sub", json!("")),
            ("wrong issuer", "iss", json!("https://other.example")),
            ("wrong audience", "aud", json!("other-client")),
            ("wrong azp", "azp", json!("other-client")),
            ("malformed azp", "azp", json!(42)),
            (
                "missing multi-audience azp",
                "aud",
                json!(["client-id", "other-client"]),
            ),
            ("future issuance", "iat", json!(u64::MAX)),
        ] {
            let mut payload = valid.clone();
            payload[claim] = value;
            invalid.push((name, payload));
        }
        for (name, payload) in invalid {
            let (provider, _) = provider(kind, Some(&payload)).await;
            assert!(
                provider
                    .get_user(ExchangeParams {
                        auth_code: "code",
                        expected_nonce: Some("challenge-nonce"),
                        ..Default::default()
                    })
                    .await
                    .is_err(),
                "{kind} accepted {name}"
            );
        }
        for payload in [valid.clone(), {
            let mut multiple = valid;
            multiple["aud"] = json!(["client-id"]);
            multiple["azp"] = json!("client-id");
            multiple
        }] {
            let (provider, _) = provider(kind, Some(&payload)).await;
            let user = provider
                .get_user(ExchangeParams {
                    auth_code: "code",
                    expected_nonce: Some("challenge-nonce"),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(user.id, "subject-1");
        }
    }
}
#[tokio::test]
async fn nonce_bound_login_cannot_downgrade_to_unsigned_userinfo() {
    for kind in ["google", "oidc"] {
        let (provider, client) = provider(kind, None).await;
        assert!(
            provider
                .get_user(ExchangeParams {
                    auth_code: "code",
                    expected_nonce: Some("challenge-nonce"),
                    ..Default::default()
                })
                .await
                .is_err(),
            "{kind} silently bypassed nonce"
        );
        assert_eq!(client.userinfo_calls.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn apple_refresh_verifies_id_token_and_preserves_opaque_access_token() {
    let (provider, _) = provider("apple", Some(&claims("apple"))).await;
    let user = provider.refresh_token("old-refresh-token").await.unwrap();
    assert_eq!(user.id, "subject-1");
    assert_eq!(
        secrecy::ExposeSecret::expose_secret(&user.access_token),
        "opaque-access-token"
    );
    assert_eq!(
        secrecy::ExposeSecret::expose_secret(user.refresh_token.as_ref().unwrap()),
        "rotated-refresh-token"
    );
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn oidc_refresh_uses_the_refresh_token_parameter() {
    let (provider, client) = provider("oidc", Some(&claims("oidc"))).await;
    assert_eq!(
        provider
            .refresh_token("old-refresh-token")
            .await
            .unwrap()
            .id,
        "subject-1"
    );
    let forms = client.token_forms.lock().unwrap();
    let form: std::collections::HashMap<String, String> =
        url::form_urlencoded::parse(forms[0].as_bytes())
            .into_owned()
            .collect();
    assert_eq!(
        form.get("refresh_token").map(String::as_str),
        Some("old-refresh-token")
    );
    assert_eq!(
        form.get("grant_type").map(String::as_str),
        Some("refresh_token")
    );
    assert!(!form.contains_key("code"));
}

#[tokio::test]
async fn oauth_only_adapters_reject_nonce_requirements_before_transport() {
    use crate::providers::*;
    struct CountingClient(AtomicUsize);
    #[async_trait]
    impl HttpClient for CountingClient {
        async fn execute(&self, _req: HttpRequest) -> Result<HttpResponse, ConnectError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Err(ConnectError::Provider("unexpected transport".into()))
        }
    }
    let client = Arc::new(CountingClient(AtomicUsize::new(0)));
    let callback = "https://app.example/callback";
    let providers: Vec<Box<dyn Provider>> = vec![
        Box::new(
            GithubProvider::try_new("client-id", "test-secret".into(), callback)
                .unwrap()
                .with_http_client(client.clone()),
        ),
        Box::new(
            FacebookProvider::try_new("client-id", "test-secret".into(), callback)
                .unwrap()
                .with_http_client(client.clone()),
        ),
        Box::new(
            DiscordProvider::try_new("client-id", "test-secret".into(), callback)
                .unwrap()
                .with_http_client(client.clone()),
        ),
        Box::new(
            MicrosoftProvider::try_new("client-id", "test-secret".into(), callback)
                .unwrap()
                .with_http_client(client.clone()),
        ),
        Box::new(
            LinkedinProvider::try_new("client-id", "test-secret".into(), callback)
                .unwrap()
                .with_http_client(client.clone()),
        ),
        Box::new(
            XProvider::try_new("client-id", "test-secret".into(), callback)
                .unwrap()
                .with_http_client(client.clone()),
        ),
        Box::new(
            Auth0Provider::try_new(
                "client-id",
                "test-secret".into(),
                callback,
                "tenant.auth0.com",
            )
            .unwrap()
            .with_http_client(client.clone()),
        ),
        Box::new(
            CognitoProvider::try_new(
                "client-id",
                "test-secret".into(),
                callback,
                "https://tenant.auth.us-east-1.amazoncognito.com",
            )
            .unwrap()
            .with_http_client(client.clone()),
        ),
    ];
    for provider in providers {
        assert!(
            matches!(provider.get_user(ExchangeParams { auth_code:"code", expected_nonce:Some("nonce"), ..Default::default() }).await,
            Err(ConnectError::Provider(reason)) if reason.contains("OAuth-only"))
        );
    }
    assert_eq!(client.0.load(Ordering::SeqCst), 0);
}
