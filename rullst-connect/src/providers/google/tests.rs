//! Unit and mock tests for Google OAuth2 and OpenID Connect token exchange.

#![allow(clippy::unwrap_used)]

use super::*;
use crate::client::{HttpClient, HttpRequest, HttpResponse};
use crate::provider::Provider;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

#[test]
fn test_google_redirect_url() {
    let provider = GoogleProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
    );

    let url = provider.redirect_url();
    assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
    assert!(url.contains("client_id=client_id"));
    assert!(url.contains("redirect_uri=https%3A%2F%2Fredirect.url"));
}

struct MockGoogleClient {
    token_status: u16,
    token_body: serde_json::Value,
    user_status: u16,
    user_body: serde_json::Value,
}

#[async_trait]
impl HttpClient for MockGoogleClient {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, crate::error::ConnectError> {
        if req.url.contains("token") {
            Ok(HttpResponse {
                status: self.token_status,
                body: self.token_body.clone(),
            })
        } else if req.url.contains("userinfo") {
            Ok(HttpResponse {
                status: self.user_status,
                body: self.user_body.clone(),
            })
        } else {
            Err(crate::error::ConnectError::Provider(
                "Unexpected URL".to_string(),
            ))
        }
    }
}

#[tokio::test]
async fn test_google_get_user_success() {
    let provider = GoogleProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
    )
    .with_http_client(Arc::new(MockGoogleClient {
        token_status: 200,
        token_body: json!({
            "access_token": "mock_access_token",
            "expires_in": 3600
        }), // Omit id_token so it uses userinfo
        user_status: 200,
        user_body: json!({
            "sub": "user_123",
            "name": "Test User",
            "email": "test@example.com",
            "picture": "https://avatar.url",
            "email_verified": true
        }),
    }));

    let user = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(user.id, "user_123");
    assert_eq!(user.name, "Test User");
    assert_eq!(user.email.as_deref(), Some("test@example.com"));
}

#[tokio::test]
async fn test_google_token_error() {
    let provider = GoogleProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
    )
    .with_http_client(Arc::new(MockGoogleClient {
        token_status: 400,
        token_body: json!({"error": "invalid_grant"}),
        user_status: 200,
        user_body: json!({}),
    }));

    let err = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            ..Default::default()
        })
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        crate::error::ConnectError::ProviderApiError { .. }
    ));
}

#[tokio::test]
async fn test_google_missing_id() {
    let provider = GoogleProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
    )
    .with_http_client(Arc::new(MockGoogleClient {
        token_status: 200,
        token_body: json!({"access_token": "mock_access_token"}),
        user_status: 200,
        user_body: json!({"name": "No ID User"}),
    }));

    let err = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            ..Default::default()
        })
        .await
        .unwrap_err();

    assert!(matches!(err, crate::error::ConnectError::Provider(_)));
}

#[tokio::test]
async fn test_google_id_token_invalid_jwt() {
    let provider = GoogleProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
    )
    .with_http_client(Arc::new(MockGoogleClient {
        token_status: 200,
        token_body: json!({
            "access_token": "mock_access_token",
            "id_token": "invalid_jwt_format"
        }),
        user_status: 200,
        user_body: json!({}),
    }));

    let err = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            ..Default::default()
        })
        .await
        .unwrap_err();

    assert!(
        matches!(err, crate::error::ConnectError::Provider(msg) if msg.contains("Failed to decode Google id_token header"))
    );
}

#[tokio::test]
async fn test_google_id_token_missing_kid() {
    // Create a JWT without kid
    let id_token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjMifQ.ZHVtbXk".to_string();

    let provider = GoogleProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
    )
    .with_http_client(Arc::new(MockGoogleClient {
        token_status: 200,
        token_body: json!({
            "access_token": "mock_access_token",
            "id_token": id_token
        }),
        user_status: 200,
        user_body: json!({}),
    }));

    let err = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            ..Default::default()
        })
        .await
        .unwrap_err();

    assert!(
        matches!(err, crate::error::ConnectError::Provider(msg) if msg.contains("Missing 'kid' header"))
    );
}

#[tokio::test]
async fn test_google_id_token_kid_not_found() {
    // Create a JWT with kid
    let id_token =
        "eyJhbGciOiJIUzI1NiIsImtpZCI6Im5vbl9leGlzdGVudF9raWQifQ.eyJzdWIiOiIxMjMifQ.ZHVtbXk"
            .to_string();

    struct KidNotFoundClient(String);
    #[async_trait]
    impl HttpClient for KidNotFoundClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
            if req.url.contains("token") {
                Ok(HttpResponse {
                    status: 200,
                    body: json!({
                        "access_token": "mock_access_token",
                        "id_token": self.0
                    }),
                })
            } else if req.url.contains("certs") {
                Ok(HttpResponse {
                    status: 200,
                    body: json!({
                        "keys": [
                            {
                                "kid": "other_kid",
                                "kty": "RSA",
                                "n": "123",
                                "e": "AQAB"
                            }
                        ]
                    }),
                })
            } else {
                Ok(HttpResponse {
                    status: 200,
                    body: json!({}),
                })
            }
        }
    }

    let provider = GoogleProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
    )
    .with_http_client(Arc::new(KidNotFoundClient(id_token)));

    let err = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            ..Default::default()
        })
        .await
        .unwrap_err();

    assert!(matches!(err, crate::error::ConnectError::Provider(msg) if msg.contains("not found")));
}

#[tokio::test]
async fn test_google_id_token_valid() {
    let pem = b"-----BEGIN PRIVATE KEY-----\n\
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

    let n_val = "1CeQidbquhWurg9nVZ4-X-EGJusTeaIhcSXiEh0jCHy_HEELTsD6Vu91aJoorlMNH-0Gunq5qy3vs6B0C79NN0seD1C-vMMTm1RV0DxSJ1MGyx26gzc2be4nRLayt59AcrY6Je5_4NwkX9InJdJGHDtWG7MHyb5ic4zil1XrCvbOQoLoGi-nUe0yHSfIG6CAmAH-ZO0_aTwDFRC8-chDNkmo80IYlNSvWNGx9MMwQtWkaQVS1Oc_YhuwDuBCeQhsouSEHCXImIKdRxghVihmNjTkNm3rJllBTjxyXvhGRIsavO3F6HfC5ceZ2bamKD5aJoGHIhxI_Y6RRZyDUb7yuQ";
    let e_val = "AQAB";

    let priv_key = jsonwebtoken::EncodingKey::from_rsa_pem(pem).unwrap();
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some("valid_kid".to_string());

    // Expiration in the future
    let exp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600;

    let claims = json!({
        "iss": "https://accounts.google.com",
        "aud": "client_id",
        "exp": exp,
        "sub": "user_id_123",
        "name": "Test User",
        "email": "test@example.com",
        "picture": "https://avatar.url",
        "email_verified": true,
        "nonce": "test_nonce"
    });

    let id_token = jsonwebtoken::encode(&header, &claims, &priv_key).unwrap();

    struct ValidClient {
        id_token: String,
        n: String,
        e: String,
    }
    #[async_trait]
    impl HttpClient for ValidClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
            if req.url.contains("token") {
                Ok(HttpResponse {
                    status: 200,
                    body: json!({
                        "access_token": "mock_access_token",
                        "id_token": self.id_token
                    }),
                })
            } else if req.url.contains("certs") {
                Ok(HttpResponse {
                    status: 200,
                    body: json!({
                        "keys": [
                            {
                                "kid": "valid_kid",
                                "kty": "RSA",
                                "alg": "RS256",
                                "n": self.n,
                                "e": self.e
                            }
                        ]
                    }),
                })
            } else {
                Ok(HttpResponse {
                    status: 200,
                    body: json!({}),
                })
            }
        }
    }

    let provider = GoogleProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
    )
    .with_http_client(Arc::new(ValidClient {
        id_token,
        n: n_val.to_string(),
        e: e_val.to_string(),
    }));

    let user = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            expected_nonce: Some("test_nonce"),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(user.id, "user_id_123");
    assert_eq!(user.name, "Test User");
    assert_eq!(user.email.as_deref(), Some("test@example.com"));

    let err = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            expected_nonce: Some("wrong_nonce"),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(
        matches!(err, crate::error::ConnectError::Provider(msg) if msg.contains("nonce mismatch"))
    );
}

#[tokio::test]
async fn test_google_refresh_token_success() {
    let provider = GoogleProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
    )
    .with_http_client(Arc::new(MockGoogleClient {
        token_status: 200,
        token_body: json!({
            "access_token": "new_access_token",
            "refresh_token": "new_refresh_token",
            "expires_in": 3600
        }),
        user_status: 200,
        user_body: json!({
            "sub": "user_123",
            "name": "Test User Refreshed",
            "email": "test@example.com",
            "picture": "https://avatar.url",
            "email_verified": true
        }),
    }));

    let user = provider.refresh_token("old_refresh").await.unwrap();
    assert_eq!(user.id, "user_123");
    assert_eq!(user.name, "Test User Refreshed");
    use secrecy::ExposeSecret;
    assert_eq!(
        user.refresh_token.unwrap().expose_secret(),
        "new_refresh_token"
    );
}

#[tokio::test]
async fn test_google_revoke_token() {
    struct MockRevokeClient(u16);
    #[async_trait]
    impl HttpClient for MockRevokeClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
            if req.url.contains("revoke") {
                Ok(HttpResponse {
                    status: self.0,
                    body: json!({}),
                })
            } else {
                Err(crate::error::ConnectError::Provider(
                    "Unexpected URL".to_string(),
                ))
            }
        }
    }

    let provider = GoogleProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
    )
    .with_http_client(Arc::new(MockRevokeClient(200)));
    provider.revoke_token("some_token").await.unwrap();

    let provider_err = GoogleProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
    )
    .with_http_client(Arc::new(MockRevokeClient(500)));
    assert!(provider_err.revoke_token("some_token").await.is_err());
}

#[test]
#[cfg(feature = "retry")]
fn test_google_with_retry() {
    let provider = GoogleProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
    );
    let original_client = provider.http_client.clone();
    let provider = provider.with_retry(3);
    assert_eq!(provider.client_id, "client_id");
    // New client must differ from the one before calling with_retry.
    assert!(!std::sync::Arc::ptr_eq(
        &provider.http_client,
        &original_client
    ));
    // Kills the mutant `replace with_retry -> Self with Default::default()`:
    // Default::default() would clone the global DEFAULT_HTTP_CLIENT, so the
    // new http_client would be ptr_eq to it. A real with_retry creates a
    // fresh ReqwestClient, which is a distinct allocation.
    assert!(
        !std::sync::Arc::ptr_eq(&provider.http_client, &crate::client::DEFAULT_HTTP_CLIENT),
        "with_retry must create a new client, not reuse DEFAULT_HTTP_CLIENT"
    );
}

#[tokio::test]
async fn test_google_id_token_invalid_algorithm() {
    let secret = b"super_secret_key_123456789012345";
    let priv_key = jsonwebtoken::EncodingKey::from_secret(secret);
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    header.kid = Some("valid_kid".to_string());

    let claims = serde_json::json!({
        "iss": "https://accounts.google.com",
        "aud": "client_id",
        "exp": 9999999999u64,
        "sub": "user_123",
        "nonce": "test_nonce"
    });

    let id_token = jsonwebtoken::encode(&header, &claims, &priv_key).unwrap();

    struct MockClient {
        id_token: String,
    }
    #[async_trait]
    impl HttpClient for MockClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
            if req.url.contains("token") {
                Ok(HttpResponse {
                    status: 200,
                    body: serde_json::json!({
                        "access_token": "mock",
                        "id_token": self.id_token
                    }),
                })
            } else if req.url.contains("certs") {
                Ok(HttpResponse {
                    status: 200,
                    body: serde_json::json!({
                        "keys": [
                            {
                                "kid": "valid_kid",
                                "kty": "RSA",
                                "alg": "RS256",
                                "n": "sWwEyNwXz_oht6BVZqJiGoKVFRWyeesgSgJYcM4GwWS_Y45iEkZdbYuPlewORhVz8JE7tfTmVVInRmLDAoAEeTB-knrZPjaL0poZmCiCGbbNOa8lUXPbJJrYFbQlGhwMOBfZOpeJcjat3xuJRtqkmaq6_bGu9BfJGUOwzZ3rP835JChqR_oOmUpcC6EPR9BB0pdrvBYZ_tlsKhgmNJI6dtK1NfQTiIr4tj49IiSaVCI2cyIxKf2kzWu5j9YfqKtcTUlqQkO26WCcdBjO2NLRiV0Sn-QLGPlQJ0oDmQjD_SUO9xnsNmtIpbdkH6J-nrKH0wW9FQW79617Up6qbu7XBQ",
                                "e": "AQAB"
                            }
                        ]
                    }),
                })
            } else {
                Ok(HttpResponse {
                    status: 200,
                    body: serde_json::json!({}),
                })
            }
        }
    }

    let provider = GoogleProvider::new(
        "client_id".to_string(),
        secrecy::SecretString::from("client_secret".to_string()),
        "https://redirect.url".to_string(),
    )
    .with_http_client(std::sync::Arc::new(MockClient { id_token }));

    let err = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            expected_nonce: Some("test_nonce"),
            ..Default::default()
        })
        .await
        .unwrap_err();

    assert!(
        matches!(err, crate::error::ConnectError::Provider(msg) if msg.contains("Unsupported algorithm"))
    );
}
