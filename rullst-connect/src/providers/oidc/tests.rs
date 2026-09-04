use super::discovery::OidcProvider;
use crate::client::{HttpClient, HttpRequest, HttpResponse};
use crate::error::ConnectError;
use crate::provider::Provider;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

struct MockOidcClient {
    config_body: Value,
    jwks_body: Value,
    captured_urls: tokio::sync::Mutex<Vec<String>>,
}

#[async_trait]
impl HttpClient for MockOidcClient {
    async fn execute(
        &self,
        req: crate::client::HttpRequest,
    ) -> Result<crate::client::HttpResponse, crate::error::ConnectError> {
        self.captured_urls.lock().await.push(req.url.clone());
        if req.url.contains("openid-configuration") {
            Ok(crate::client::HttpResponse {
                status: 200,
                body: self.config_body.clone(),
            })
        } else if req.url.contains("jwks") {
            Ok(crate::client::HttpResponse {
                status: 200,
                body: self.jwks_body.clone(),
            })
        } else if req.url.contains("missing_id") {
            Ok(crate::client::HttpResponse {
                status: 200,
                body: json!({"name": "No ID User"}),
            })
        } else if req.url.contains("error_token") {
            Ok(crate::client::HttpResponse {
                status: 400,
                body: json!({"error": "invalid_grant"}),
            })
        } else if req.url.contains("refresh_token_test") {
            Ok(crate::client::HttpResponse {
                status: 200,
                body: json!({
                    "access_token": "new_access_token",
                    "refresh_token": "new_refresh_token",
                    "expires_in": 3600
                }),
            })
        } else if req.url.contains("token") {
            Ok(crate::client::HttpResponse {
                status: 200,
                body: json!({
                    "access_token": "mock_access_token",
                    "expires_in": 3600
                }),
            })
        } else if req.url.contains("userinfo") {
            Ok(crate::client::HttpResponse {
                status: 200,
                body: json!({
                    "sub": "user_123",
                    "name": "Test User",
                    "email": "test@example.com",
                    "picture": "https://avatar.url",
                    "email_verified": true
                }),
            })
        } else {
            Err(crate::error::ConnectError::Provider(
                "Not found".to_string(),
            ))
        }
    }
}

#[tokio::test]
async fn test_oidc_discover_success_with_slash() {
    let mock_client = Arc::new(MockOidcClient {
        config_body: json!({
            "authorization_endpoint": "https://auth.com/authorize",
            "token_endpoint": "https://auth.com/token",
            "userinfo_endpoint": "https://auth.com/userinfo",
            "jwks_uri": "https://auth.com/jwks",
            "issuer": "https://issuer.com"
        }),
        jwks_body: json!({
            "keys": []
        }),
        captured_urls: tokio::sync::Mutex::new(vec![]),
    });

    // Test with trailing slash
    let _provider = OidcProvider::discover_with_client(
        "https://issuer.com/",
        "client_id".to_string(),
        "client_secret".to_string(),
        "https://redirect.url".to_string(),
        mock_client.clone(),
    )
    .await
    .expect("OIDC discovery failed");

    let urls = mock_client.captured_urls.lock().await;
    assert_eq!(urls.len(), 1);
    assert_eq!(
        urls[0],
        "https://issuer.com/.well-known/openid-configuration"
    );
}

#[tokio::test]
async fn test_oidc_discover_success_no_slash() {
    let mock_client = Arc::new(MockOidcClient {
        config_body: json!({
            "authorization_endpoint": "https://auth.com/authorize",
            "token_endpoint": "https://auth.com/token",
            "userinfo_endpoint": "https://auth.com/userinfo",
            "jwks_uri": "https://auth.com/jwks",
            "issuer": "https://issuer.com"
        }),
        jwks_body: json!({
            "keys": []
        }),
        captured_urls: tokio::sync::Mutex::new(vec![]),
    });

    // Test without trailing slash
    let _provider = OidcProvider::discover_with_client(
        "https://issuer.com",
        "client_id".to_string(),
        "client_secret".to_string(),
        "https://redirect.url".to_string(),
        mock_client.clone(),
    )
    .await
    .expect("OIDC discovery failed");

    let urls = mock_client.captured_urls.lock().await;
    assert_eq!(urls.len(), 1);
    assert_eq!(
        urls[0],
        "https://issuer.com/.well-known/openid-configuration"
    );
}

#[tokio::test]
async fn test_oidc_discover_missing_token_endpoint() {
    let mock_client = Arc::new(MockOidcClient {
        config_body: json!({
            "authorization_endpoint": "https://auth.com/authorize",
            "userinfo_endpoint": "https://auth.com/userinfo",
            "jwks_uri": "https://auth.com/jwks",
            "issuer": "https://issuer.com"
        }),
        jwks_body: json!({
            "keys": []
        }),
        captured_urls: tokio::sync::Mutex::new(vec![]),
    });

    let res = OidcProvider::discover_with_client(
        "https://issuer.com",
        "client_id".to_string(),
        "client_secret".to_string(),
        "https://redirect.url".to_string(),
        mock_client.clone(),
    )
    .await;

    match res {
        Err(crate::error::ConnectError::Provider(msg)) => {
            assert!(msg.contains("Missing token_endpoint"));
        }
        Err(_) => panic!("Expected Provider error variant"),
        Ok(_) => panic!("Expected an error, but discover succeeded"),
    }
}

#[tokio::test]
async fn test_oidc_discover_invalid_args() {
    let mock_client = Arc::new(MockOidcClient {
        config_body: json!({}),
        jwks_body: json!({}),
        captured_urls: tokio::sync::Mutex::new(vec![]),
    });

    let err = OidcProvider::discover_with_client(
        "http://invalid_url",
        "id".to_string(),
        "secret".to_string(),
        "https://redirect".to_string(),
        mock_client.clone(),
    )
    .await
    .err()
    .expect("expected error");
    assert!(matches!(
        err,
        crate::error::ConnectError::InvalidConfiguration {
            field: "issuer_url",
            ..
        }
    ));

    let err = OidcProvider::discover_with_client(
        "https://issuer",
        "id".to_string(),
        "secret".to_string(),
        "http://invalid_redirect".to_string(),
        mock_client.clone(),
    )
    .await
    .err()
    .expect("expected error");
    assert!(matches!(
        err,
        crate::error::ConnectError::InvalidConfiguration {
            field: "redirect_url",
            ..
        }
    ));

    let provider = OidcProvider::discover_with_client(
        "https://issuer",
        "".to_string(),
        "secret".to_string(),
        "https://redirect".to_string(),
        mock_client.clone(),
    )
    .await
    .expect("empty credentials select the offline provider")
    .with_http_client(mock_client.clone());
    assert_eq!(
        provider.credential_mode(),
        crate::configuration::CredentialMode::Mock
    );
    let user = provider
        .get_user(crate::provider::ExchangeParams::default())
        .await
        .expect("offline OIDC user");
    assert_eq!(user.id, "mock-user");
    assert!(
        mock_client.captured_urls.lock().await.is_empty(),
        "a custom client must not replace mock mode's network-free transport"
    );

    let provider = OidcProvider::discover_with_client(
        "https://issuer",
        "id".to_string(),
        "".to_string(),
        "https://redirect".to_string(),
        mock_client.clone(),
    )
    .await
    .expect("empty credentials select the offline provider");
    assert_eq!(
        provider.credential_mode(),
        crate::configuration::CredentialMode::Mock
    );

    let error = OidcProvider::discover_with_client(
        "http://localhost.evil",
        "id",
        "secret",
        "https://redirect.example/callback",
        mock_client,
    )
    .await
    .err()
    .expect("lookalike localhost host must be rejected");
    assert!(matches!(
        error,
        crate::error::ConnectError::InvalidConfiguration {
            field: "issuer_url",
            ..
        }
    ));
}

#[tokio::test]
async fn test_oidc_rejects_mismatched_issuer_and_insecure_endpoint() {
    let mismatched = Arc::new(MockOidcClient {
        config_body: json!({
            "authorization_endpoint": "https://auth.com/authorize",
            "token_endpoint": "https://auth.com/token",
            "userinfo_endpoint": "https://auth.com/userinfo",
            "jwks_uri": "https://auth.com/jwks",
            "issuer": "https://attacker.example"
        }),
        jwks_body: json!({"keys": []}),
        captured_urls: tokio::sync::Mutex::new(vec![]),
    });
    let error = OidcProvider::discover_with_client(
        "https://issuer.example",
        "client",
        "secret",
        "https://app.example/callback",
        mismatched,
    )
    .await
    .err()
    .expect("issuer mismatch must fail");
    assert!(matches!(
        error,
        ConnectError::InvalidConfiguration {
            field: "issuer",
            ..
        }
    ));

    let insecure = Arc::new(MockOidcClient {
        config_body: json!({
            "authorization_endpoint": "https://auth.com/authorize",
            "token_endpoint": "http://auth.com/token",
            "userinfo_endpoint": "https://auth.com/userinfo",
            "jwks_uri": "https://auth.com/jwks",
            "issuer": "https://issuer.example"
        }),
        jwks_body: json!({"keys": []}),
        captured_urls: tokio::sync::Mutex::new(vec![]),
    });
    let error = OidcProvider::discover_with_client(
        "https://issuer.example",
        "client",
        "secret",
        "https://app.example/callback",
        insecure,
    )
    .await
    .err()
    .expect("insecure endpoint must fail");
    assert!(matches!(
        error,
        ConnectError::InvalidConfiguration {
            field: "token_endpoint",
            ..
        }
    ));
}

#[tokio::test]
async fn test_oidc_discover_success_localhost() {
    let mock_client = Arc::new(MockOidcClient {
        config_body: json!({
            "authorization_endpoint": "https://auth.com/authorize",
            "token_endpoint": "https://auth.com/token",
            "userinfo_endpoint": "https://auth.com/userinfo",
            "jwks_uri": "https://auth.com/jwks",
            "issuer": "http://localhost:8080"
        }),
        jwks_body: json!({
            "keys": []
        }),
        captured_urls: tokio::sync::Mutex::new(vec![]),
    });

    let _provider = OidcProvider::discover_with_client(
        "http://localhost:8080",
        "client_id".to_string(),
        "client_secret".to_string(),
        "http://localhost:8080/redirect".to_string(),
        mock_client.clone(),
    )
    .await
    .expect("OIDC discovery failed");
}

#[tokio::test]
async fn test_oidc_discover_success_127_0_0_1() {
    let mock_client = Arc::new(MockOidcClient {
        config_body: json!({
            "authorization_endpoint": "https://auth.com/authorize",
            "token_endpoint": "https://auth.com/token",
            "userinfo_endpoint": "https://auth.com/userinfo",
            "jwks_uri": "https://auth.com/jwks",
            "issuer": "http://127.0.0.1:8080"
        }),
        jwks_body: json!({
            "keys": []
        }),
        captured_urls: tokio::sync::Mutex::new(vec![]),
    });

    let _provider = OidcProvider::discover_with_client(
        "http://127.0.0.1:8080",
        "client_id".to_string(),
        "client_secret".to_string(),
        "http://127.0.0.1:8080/redirect".to_string(),
        mock_client.clone(),
    )
    .await
    .expect("OIDC discovery failed");
}

#[tokio::test]
async fn test_oidc_get_user_success() {
    let mock_client = Arc::new(MockOidcClient {
        config_body: json!({
            "authorization_endpoint": "https://auth.com/authorize",
            "token_endpoint": "https://auth.com/token",
            "userinfo_endpoint": "https://auth.com/userinfo",
            "jwks_uri": "https://auth.com/jwks",
            "issuer": "https://issuer.com"
        }),
        jwks_body: json!({"keys": []}),
        captured_urls: tokio::sync::Mutex::new(vec![]),
    });

    let provider = OidcProvider::discover_with_client(
        "https://issuer.com",
        "client_id".to_string(),
        "client_secret".to_string(),
        "https://redirect.url".to_string(),
        mock_client.clone(),
    )
    .await
    .unwrap();

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
async fn test_oidc_token_error() {
    let mock_client = Arc::new(MockOidcClient {
        config_body: json!({
            "authorization_endpoint": "https://auth.com/authorize",
            "token_endpoint": "https://auth.com/error_token", // use special token URL
            "userinfo_endpoint": "https://auth.com/userinfo",
            "jwks_uri": "https://auth.com/jwks",
            "issuer": "https://issuer.com"
        }),
        jwks_body: json!({"keys": []}),
        captured_urls: tokio::sync::Mutex::new(vec![]),
    });

    let provider = OidcProvider::discover_with_client(
        "https://issuer.com",
        "client_id".to_string(),
        "client_secret".to_string(),
        "https://redirect.url".to_string(),
        mock_client.clone(),
    )
    .await
    .unwrap();

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
async fn test_oidc_refresh_token_success() {
    let mock_client = Arc::new(MockOidcClient {
        config_body: json!({
            "authorization_endpoint": "https://auth.com/authorize",
            "token_endpoint": "https://auth.com/refresh_token_test", // use special refresh token URL
            "userinfo_endpoint": "https://auth.com/userinfo",
            "jwks_uri": "https://auth.com/jwks",
            "issuer": "https://issuer.com"
        }),
        jwks_body: json!({"keys": []}),
        captured_urls: tokio::sync::Mutex::new(vec![]),
    });

    let provider = OidcProvider::discover_with_client(
        "https://issuer.com",
        "client_id".to_string(),
        "client_secret".to_string(),
        "https://redirect.url".to_string(),
        mock_client.clone(),
    )
    .await
    .unwrap();

    let user = provider.refresh_token("old_refresh").await.unwrap();

    assert_eq!(user.id, "user_123");
    assert_eq!(user.name, "Test User");
    use secrecy::ExposeSecret;
    assert_eq!(
        user.refresh_token.unwrap().expose_secret(),
        "new_refresh_token"
    );
}

#[tokio::test]
async fn test_oidc_missing_id() {
    let mock_client = Arc::new(MockOidcClient {
        config_body: json!({
            "authorization_endpoint": "https://auth.com/authorize",
            "token_endpoint": "https://auth.com/token",
            "userinfo_endpoint": "https://auth.com/missing_id_userinfo", // use special missing ID url
            "jwks_uri": "https://auth.com/jwks",
            "issuer": "https://issuer.com"
        }),
        jwks_body: json!({"keys": []}),
        captured_urls: tokio::sync::Mutex::new(vec![]),
    });

    let provider = OidcProvider::discover_with_client(
        "https://issuer.com",
        "client_id".to_string(),
        "client_secret".to_string(),
        "https://redirect.url".to_string(),
        mock_client.clone(),
    )
    .await
    .unwrap();

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
async fn test_oidc_id_token_invalid_jwt() {
    struct InvalidJwtClient;
    #[async_trait]
    impl HttpClient for InvalidJwtClient {
        async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, ConnectError> {
            if req.url.contains("token_invalid_jwt") {
                Ok(HttpResponse {
                    status: 200,
                    body: json!({
                        "access_token": "mock_access_token",
                        "id_token": "invalid_jwt_format"
                    }),
                })
            } else if req.url.contains("openid-configuration") {
                Ok(HttpResponse {
                    status: 200,
                    body: json!({
                        "authorization_endpoint": "https://auth.com/authorize",
                        "token_endpoint": "https://auth.com/token_invalid_jwt",
                        "userinfo_endpoint": "https://auth.com/userinfo",
                        "jwks_uri": "https://auth.com/jwks",
                        "issuer": "https://issuer.com"
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

    let provider = OidcProvider::discover_with_client(
        "https://issuer.com",
        "client_id".to_string(),
        "client_secret".to_string(),
        "https://redirect.url".to_string(),
        Arc::new(InvalidJwtClient),
    )
    .await
    .unwrap();

    let err = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            ..Default::default()
        })
        .await
        .unwrap_err();

    assert!(
        matches!(err, crate::error::ConnectError::Provider(msg) if msg.contains("Failed to decode OIDC id_token header"))
    );
}

#[tokio::test]
async fn test_oidc_id_token_missing_kid() {
    let id_token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjMifQ.ZHVtbXk".to_string();

    struct MissingKidClient(String);
    #[async_trait]
    impl HttpClient for MissingKidClient {
        async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, ConnectError> {
            if req.url.contains("token") {
                Ok(HttpResponse {
                    status: 200,
                    body: json!({
                        "access_token": "mock_access_token",
                        "id_token": self.0
                    }),
                })
            } else if req.url.contains("openid-configuration") {
                Ok(HttpResponse {
                    status: 200,
                    body: json!({
                        "authorization_endpoint": "https://auth.com/authorize",
                        "token_endpoint": "https://auth.com/token",
                        "userinfo_endpoint": "https://auth.com/userinfo",
                        "jwks_uri": "https://auth.com/jwks",
                        "issuer": "https://issuer.com"
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

    let provider = OidcProvider::discover_with_client(
        "https://issuer.com",
        "client_id".to_string(),
        "client_secret".to_string(),
        "https://redirect.url".to_string(),
        Arc::new(MissingKidClient(id_token)),
    )
    .await
    .unwrap();

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
async fn test_oidc_id_token_kid_not_found() {
    let id_token =
        "eyJhbGciOiJIUzI1NiIsImtpZCI6Im5vbl9leGlzdGVudF9raWQifQ.eyJzdWIiOiIxMjMifQ.ZHVtbXk"
            .to_string();

    struct KidNotFoundClient(String);
    #[async_trait]
    impl HttpClient for KidNotFoundClient {
        async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, ConnectError> {
            if req.url.contains("token") {
                Ok(HttpResponse {
                    status: 200,
                    body: json!({
                        "access_token": "mock_access_token",
                        "id_token": self.0
                    }),
                })
            } else if req.url.contains("openid-configuration") {
                Ok(HttpResponse {
                    status: 200,
                    body: json!({
                        "authorization_endpoint": "https://auth.com/authorize",
                        "token_endpoint": "https://auth.com/token",
                        "userinfo_endpoint": "https://auth.com/userinfo",
                        "jwks_uri": "https://auth.com/jwks",
                        "issuer": "https://issuer.com"
                    }),
                })
            } else if req.url.contains("jwks") {
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

    let provider = OidcProvider::discover_with_client(
        "https://issuer.com",
        "client_id".to_string(),
        "client_secret".to_string(),
        "https://redirect.url".to_string(),
        Arc::new(KidNotFoundClient(id_token)),
    )
    .await
    .unwrap();

    let err = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            ..Default::default()
        })
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        crate::error::ConnectError::JwkNotFound(ref kid) if kid == "non_existent_kid"
    ));
}

#[tokio::test]
async fn test_oidc_id_token_valid() {
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

    let claims = json!({
        "iss": "https://issuer.com",
        "aud": "client_id",
        "exp": exp,
        "sub": "oidc_user_123",
        "name": "OIDC User",
        "email": "oidc@example.com",
        "picture": "https://oidc.avatar",
        "email_verified": true,
        "nonce": "test_nonce"
    });

    let id_token = jsonwebtoken::encode(&header, &claims, &priv_key).unwrap();

    struct ValidOidcClient {
        id_token: String,
        n: String,
        e: String,
    }
    #[async_trait]
    impl HttpClient for ValidOidcClient {
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
            } else if req.url.contains("openid-configuration") {
                Ok(HttpResponse {
                    status: 200,
                    body: json!({
                        "authorization_endpoint": "https://auth.com/authorize",
                        "token_endpoint": "https://auth.com/token",
                        "userinfo_endpoint": "https://auth.com/userinfo",
                        "jwks_uri": "https://auth.com/jwks",
                        "issuer": "https://issuer.com"
                    }),
                })
            } else if req.url.contains("jwks") {
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

    let provider = OidcProvider::discover_with_client(
        "https://issuer.com",
        "client_id".to_string(),
        "client_secret".to_string(),
        "https://redirect.url".to_string(),
        Arc::new(ValidOidcClient {
            id_token,
            n: n_val.to_string(),
            e: e_val.to_string(),
        }),
    )
    .await
    .unwrap();

    let user = provider
        .get_user(crate::provider::ExchangeParams {
            auth_code: "code",
            expected_nonce: Some("test_nonce"),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(user.id, "oidc_user_123");
    assert_eq!(user.name, "OIDC User");
    assert_eq!(user.email.as_deref(), Some("oidc@example.com"));

    // Test Nonce Mismatch
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
