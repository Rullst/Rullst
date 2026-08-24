//! Unit and mock tests for Apple OAuth2 and token exchange.

#![allow(clippy::unwrap_used)]

use super::types::*;
use super::*;
use crate::client::{HttpClient, HttpRequest, HttpResponse};
use crate::provider::Provider;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

#[test]
fn test_apple_redirect_url() {
    let provider = AppleProvider::new(
        "client_id".to_string(),
        "team_id".to_string(),
        "key_id".to_string(),
        "private_key".to_string(),
        "https://redirect.url".to_string(),
    );

    let url = provider.redirect_url();
    assert!(url.starts_with("https://appleid.apple.com/auth/authorize?"));
    assert!(url.contains("client_id=client_id"));
    assert!(url.contains("redirect_uri=https%3A%2F%2Fredirect.url"));
    assert!(url.contains("response_mode=form_post"));
}

#[tokio::test]
async fn test_apple_empty_credentials_use_offline_mock() {
    let provider = AppleProvider::try_new("", "", "", "", "https://app.example/callback")
        .expect("mock provider");
    assert_eq!(
        provider.credential_mode(),
        crate::configuration::CredentialMode::Mock
    );
    let user = provider
        .get_user(crate::provider::ExchangeParams::default())
        .await
        .expect("offline user");
    assert_eq!(user.id, "mock-user");
    assert!(provider.redirect_url().contains("example.invalid"));
}

#[test]
fn test_apple_generate_client_secret_exp() {
    let provider = AppleProvider::new(
        "client_id".to_string(),
        "team_id".to_string(),
        "key_id".to_string(),
        "-----BEGIN PRIVATE KEY-----\n\
        MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgkdn4ngP0MJj/+G/Z\n\
        0FgfmUYbc26Oidgl0NZoUXoMm6KhRANCAARcJ2gzcG1e8qufjKrOWQSmC4OoQkAU\n\
        k/Tz7c8S43tqF0VK/mNC462881k2cryVtuV5FkH1XoPACJzJUQ5igUZV\n\
        -----END PRIVATE KEY-----"
            .to_string(),
        "https://redirect.url".to_string(),
    );

    let secret = provider.generate_client_secret().unwrap();
    // Decode without signature verification to check claims
    let parts: Vec<&str> = secret.split('.').collect();
    use base64::Engine;
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .unwrap();
    let claims: AppleClaims = serde_json::from_slice(&payload_bytes).unwrap();

    assert_eq!(claims.exp, claims.iat + 300);
    assert_eq!(claims.iss, "team_id");
    assert_eq!(claims.sub, "client_id");
}

#[tokio::test]
async fn test_apple_get_user_from_token_invalid() {
    let provider = AppleProvider::new(
        "client_id".to_string(),
        "team_id".to_string(),
        "key_id".to_string(),
        "private_key".to_string(),
        "https://redirect.url".to_string(),
    );

    let res = provider.get_user_from_token("invalid.token.format").await;
    assert!(res.is_err());
    match res.unwrap_err() {
        crate::error::ConnectError::Provider(msg) => {
            assert!(msg.contains("Apple id_token"));
        }
        _ => panic!("Expected Provider error"),
    }
}

#[tokio::test]
async fn test_apple_generate_client_secret_invalid_key() {
    let provider = AppleProvider::new(
        "client_id".to_string(),
        "team_id".to_string(),
        "key_id".to_string(),
        "invalid_private_key_pem".to_string(),
        "https://redirect.url".to_string(),
    );

    let err = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            ..Default::default()
        })
        .await
        .unwrap_err();

    assert!(matches!(err, crate::error::ConnectError::Jwt(_)));
}

struct MockAppleClient {
    token_status: u16,
    token_body: serde_json::Value,
}

#[async_trait]
impl HttpClient for MockAppleClient {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, crate::error::ConnectError> {
        if req.url.contains("token") {
            Ok(HttpResponse {
                status: self.token_status,
                body: self.token_body.clone(),
            })
        } else {
            Ok(HttpResponse {
                status: 200,
                body: json!({}),
            })
        }
    }
}

#[tokio::test]
async fn test_apple_token_error() {
    let provider = AppleProvider::new(
        "client_id".to_string(),
        "team_id".to_string(),
        "key_id".to_string(),
        "private_key".to_string(),
        "https://redirect.url".to_string(),
    )
    .with_http_client(Arc::new(MockAppleClient {
        token_status: 400,
        token_body: json!({"error": "invalid_grant"}),
    }));

    let form_data = crate::provider::TokenExchangeForm {
        client_id: "client_id",
        client_secret: Some("secret"),
        code: "code",
        grant_type: Some("authorization_code"),
        redirect_uri: "https://redirect.url",
        code_verifier: None,
    };

    let err = provider
        .get_user_from_form(&form_data, None)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        crate::error::ConnectError::ProviderApiError { .. }
    ));
}

#[tokio::test]
async fn test_apple_missing_id_token() {
    let provider = AppleProvider::new(
        "client_id".to_string(),
        "team_id".to_string(),
        "key_id".to_string(),
        "private_key".to_string(),
        "https://redirect.url".to_string(),
    )
    .with_http_client(Arc::new(MockAppleClient {
        token_status: 200,
        token_body: json!({
            "access_token": "mock_token" // missing id_token
        }),
    }));

    let form_data = crate::provider::TokenExchangeForm {
        client_id: "client_id",
        client_secret: Some("secret"),
        code: "code",
        grant_type: Some("authorization_code"),
        redirect_uri: "https://redirect.url",
        code_verifier: None,
    };

    let err = provider
        .get_user_from_form(&form_data, None)
        .await
        .unwrap_err();
    assert!(matches!(err, crate::error::ConnectError::Token(msg) if msg.contains("id_token")));
}

#[tokio::test]
async fn test_apple_missing_access_token() {
    let provider = AppleProvider::new(
        "client_id".to_string(),
        "team_id".to_string(),
        "key_id".to_string(),
        "private_key".to_string(),
        "https://redirect.url".to_string(),
    )
    .with_http_client(Arc::new(MockAppleClient {
        token_status: 200,
        token_body: json!({
            "id_token": "mock_id_token" // missing access_token
        }),
    }));

    let form_data = crate::provider::TokenExchangeForm {
        client_id: "client_id",
        client_secret: Some("secret"),
        code: "code",
        grant_type: Some("authorization_code"),
        redirect_uri: "https://redirect.url",
        code_verifier: None,
    };

    let err = provider
        .get_user_from_form(&form_data, None)
        .await
        .unwrap_err();
    assert!(matches!(err, crate::error::ConnectError::Token(msg) if msg.contains("access_token")));
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_apple_id_token_valid() {
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

    let exp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600;

    let claims = serde_json::json!({
        "iss": "https://appleid.apple.com",
        "aud": "client_id",
        "exp": exp,
        "sub": "apple_sub_123",
        "email": "apple@example.com",
        "nonce": "test_nonce"
    });

    let id_token = jsonwebtoken::encode(&header, &claims, &priv_key).unwrap();

    struct ValidAppleClient {
        id_token: String,
        n: String,
        e: String,
    }
    #[async_trait]
    impl HttpClient for ValidAppleClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
            if req.url.contains("token") {
                Ok(HttpResponse {
                    status: 200,
                    body: serde_json::json!({
                        "access_token": "mock_access_token",
                        "id_token": self.id_token
                    }),
                })
            } else if req.url.contains("keys") {
                Ok(HttpResponse {
                    status: 200,
                    body: serde_json::json!({
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
                    body: serde_json::json!({}),
                })
            }
        }
    }

    let provider = AppleProvider::new(
        "client_id".to_string(),
        "team_id".to_string(),
        "key_id".to_string(),
        "private_key".to_string(),
        "https://redirect.url".to_string(),
    )
    .with_http_client(std::sync::Arc::new(ValidAppleClient {
        id_token,
        n: n_val.to_string(),
        e: e_val.to_string(),
    }));

    let form_data = crate::provider::TokenExchangeForm {
        client_id: "client_id",
        client_secret: Some("secret"),
        code: "code",
        grant_type: Some("authorization_code"),
        redirect_uri: "https://redirect.url",
        code_verifier: None,
    };

    let user = provider
        .get_user_from_form(&form_data, Some("test_nonce"))
        .await
        .unwrap();

    assert_eq!(user.id, "apple_sub_123");
    assert_eq!(user.email.as_deref(), Some("apple@example.com"));

    let err = provider
        .get_user_from_form(&form_data, Some("wrong_nonce"))
        .await
        .unwrap_err();
    assert!(
        matches!(err, crate::error::ConnectError::Provider(msg) if msg.contains("nonce mismatch"))
    );
}

#[tokio::test]
async fn test_apple_refresh_token_success() {
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

    let priv_key = jsonwebtoken::EncodingKey::from_rsa_pem(pem).unwrap();
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some("valid_kid".to_string());
    let exp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600;
    let claims = serde_json::json!({
        "iss": "https://appleid.apple.com", "aud": "client_id", "exp": exp,
        "sub": "apple_sub_refreshed", "email": "apple@example.com"
    });
    let id_token = jsonwebtoken::encode(&header, &claims, &priv_key).unwrap();

    struct MockRefreshClient {
        id_token: String,
    }
    #[async_trait]
    impl HttpClient for MockRefreshClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
            if req.url.contains("token") {
                Ok(HttpResponse {
                    status: 200,
                    body: serde_json::json!({
                        "access_token": self.id_token,
                        "refresh_token": "new_refresh",
                        "expires_in": 3600
                    }),
                })
            } else if req.url.contains("keys") {
                Ok(HttpResponse {
                    status: 200,
                    body: serde_json::json!({
                        "keys": [{
                            "kid": "valid_kid", "kty": "RSA", "alg": "RS256",
                            "n": "1CeQidbquhWurg9nVZ4-X-EGJusTeaIhcSXiEh0jCHy_HEELTsD6Vu91aJoorlMNH-0Gunq5qy3vs6B0C79NN0seD1C-vMMTm1RV0DxSJ1MGyx26gzc2be4nRLayt59AcrY6Je5_4NwkX9InJdJGHDtWG7MHyb5ic4zil1XrCvbOQoLoGi-nUe0yHSfIG6CAmAH-ZO0_aTwDFRC8-chDNkmo80IYlNSvWNGx9MMwQtWkaQVS1Oc_YhuwDuBCeQhsouSEHCXImIKdRxghVihmNjTkNm3rJllBTjxyXvhGRIsavO3F6HfC5ceZ2bamKD5aJoGHIhxI_Y6RRZyDUb7yuQ",
                            "e": "AQAB"
                        }]
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

    let ec_pem = "-----BEGIN PRIVATE KEY-----\n\
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgkdn4ngP0MJj/+G/Z\n\
0FgfmUYbc26Oidgl0NZoUXoMm6KhRANCAARcJ2gzcG1e8qufjKrOWQSmC4OoQkAU\n\
k/Tz7c8S43tqF0VK/mNC462881k2cryVtuV5FkH1XoPACJzJUQ5igUZV\n\
-----END PRIVATE KEY-----";

    let provider = AppleProvider::new(
        "client_id".to_string(),
        "team_id".to_string(),
        "key_id".to_string(),
        ec_pem.to_string(),
        "https://redirect.url".to_string(),
    )
    .with_http_client(std::sync::Arc::new(MockRefreshClient { id_token }));

    let user = provider.refresh_token("old_refresh").await.unwrap();
    assert_eq!(user.id, "apple_sub_refreshed");
    use secrecy::ExposeSecret;
    assert_eq!(user.refresh_token.unwrap().expose_secret(), "new_refresh");
}

#[tokio::test]
async fn test_apple_refresh_token_error() {
    struct MockRefreshErrorClient;
    #[async_trait]
    impl HttpClient for MockRefreshErrorClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
            if req.url.contains("token") {
                Ok(HttpResponse {
                    status: 400,
                    body: serde_json::json!({
                        "error": "invalid_grant",
                        "error_description": "refresh token is invalid"
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

    let ec_pem = "-----BEGIN PRIVATE KEY-----\n\
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgkdn4ngP0MJj/+G/Z\n\
0FgfmUYbc26Oidgl0NZoUXoMm6KhRANCAARcJ2gzcG1e8qufjKrOWQSmC4OoQkAU\n\
k/Tz7c8S43tqF0VK/mNC462881k2cryVtuV5FkH1XoPACJzJUQ5igUZV\n\
-----END PRIVATE KEY-----";

    let provider = AppleProvider::new(
        "client_id".to_string(),
        "team_id".to_string(),
        "key_id".to_string(),
        ec_pem.to_string(),
        "https://redirect.url".to_string(),
    )
    .with_http_client(std::sync::Arc::new(MockRefreshErrorClient));

    let res = provider.refresh_token("old_refresh").await;
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(matches!(
        err,
        crate::error::ConnectError::ProviderApiError { .. }
    ));
}

#[tokio::test]
async fn test_apple_id_token_invalid_algorithm() {
    let secret = b"super_secret_key_123456789012345";
    let priv_key = jsonwebtoken::EncodingKey::from_secret(secret);
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    header.kid = Some("valid_kid".to_string());

    let claims = serde_json::json!({
        "iss": "https://appleid.apple.com",
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
            } else if req.url.contains("keys") {
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

    let provider = AppleProvider::new(
        "client_id".to_string(),
        "team_id".to_string(),
        "key_id".to_string(),
        "private_key".to_string(),
        "https://redirect.url".to_string(),
    )
    .with_http_client(std::sync::Arc::new(MockClient { id_token }));

    let form_data = crate::provider::TokenExchangeForm {
        client_id: "client_id",
        client_secret: Some("secret"),
        code: "code",
        grant_type: Some("authorization_code"),
        redirect_uri: "https://redirect.url",
        code_verifier: None,
    };

    let err = provider
        .get_user_from_form(&form_data, Some("test_nonce"))
        .await
        .unwrap_err();

    assert!(
        matches!(err, crate::error::ConnectError::Provider(msg) if msg.contains("Unsupported algorithm"))
    );
}
