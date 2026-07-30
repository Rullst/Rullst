use crate::client::{HttpClient, HttpClientExt};
use crate::error::ConnectError;
use crate::provider::Provider;
use crate::user::ConnectUser;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct OidcProvider {
    pub(crate) client_id: String,
    pub(crate) client_secret: secrecy::SecretString,
    pub(crate) redirect_url: String,
    pub(crate) http_client: Arc<dyn HttpClient>,
    pub(crate) scopes: String,
    pub(crate) state: Option<String>,
    pub(crate) pkce_challenge: Option<String>,

    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub(crate) jwks_uri: String,
    pub issuer: String,
}

impl OidcProvider {
    /// Discovers the OIDC configuration from the issuer URL and creates a new provider.
    pub async fn discover(
        issuer_url: &str,
        client_id: String,
        client_secret: String,
        redirect_url: String,
    ) -> Result<Self, ConnectError> {
        let client: Arc<dyn HttpClient> = crate::client::DEFAULT_HTTP_CLIENT.clone();
        Self::discover_with_client(issuer_url, client_id, client_secret, redirect_url, client).await
    }

    /// Internal method that performs OIDC discovery using a provided HTTP client.
    /// This exists to enable injecting mock clients in tests.
    pub(crate) async fn discover_with_client(
        issuer_url: &str,
        client_id: String,
        client_secret: String,
        redirect_url: String,
        client: Arc<dyn HttpClient>,
    ) -> Result<Self, ConnectError> {
        if !issuer_url.starts_with("https://")
            && !issuer_url.starts_with("http://127.0.0.1")
            && !issuer_url.starts_with("http://localhost")
        {
            return Err(crate::error::ConnectError::Provider(
                "OIDC Error: issuer_url must be HTTPS (or localhost)".to_string(),
            ));
        }
        if !redirect_url.starts_with("https://")
            && !redirect_url.starts_with("http://127.0.0.1")
            && !redirect_url.starts_with("http://localhost")
        {
            return Err(crate::error::ConnectError::Provider(
                "OIDC Error: redirect_url must be HTTPS (or localhost)".to_string(),
            ));
        }
        if client_id.is_empty() {
            return Err(crate::error::ConnectError::Provider(
                "OIDC Error: client_id cannot be empty".to_string(),
            ));
        }
        if client_secret.is_empty() {
            return Err(crate::error::ConnectError::Provider(
                "OIDC Error: client_secret cannot be empty".to_string(),
            ));
        }

        let well_known_url = if issuer_url.ends_with('/') {
            format!("{}.well-known/openid-configuration", issuer_url)
        } else {
            format!("{}/.well-known/openid-configuration", issuer_url)
        };

        let res = client
            .get(&well_known_url)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        let authorization_endpoint = res["authorization_endpoint"]
            .as_str()
            .ok_or_else(|| {
                crate::error::ConnectError::Provider(
                    "Missing authorization_endpoint in OIDC config".to_string(),
                )
            })?
            .to_string();

        let token_endpoint = res["token_endpoint"]
            .as_str()
            .ok_or_else(|| {
                crate::error::ConnectError::Provider(
                    "Missing token_endpoint in OIDC config".to_string(),
                )
            })?
            .to_string();

        let userinfo_endpoint = res["userinfo_endpoint"]
            .as_str()
            .ok_or_else(|| {
                crate::error::ConnectError::Provider(
                    "Missing userinfo_endpoint in OIDC config".to_string(),
                )
            })?
            .to_string();

        let jwks_uri = res["jwks_uri"]
            .as_str()
            .ok_or_else(|| {
                crate::error::ConnectError::Provider("Missing jwks_uri in OIDC config".to_string())
            })?
            .to_string();

        let issuer = res["issuer"]
            .as_str()
            .ok_or_else(|| {
                crate::error::ConnectError::Provider("Missing issuer in OIDC config".to_string())
            })?
            .to_string();

        Ok(Self {
            client_id,
            client_secret: client_secret.into(),
            redirect_url,
            http_client: client,
            scopes: "openid profile email".to_string(),
            state: None,
            pkce_challenge: None,
            authorization_endpoint,
            token_endpoint,
            userinfo_endpoint,
            jwks_uri,
            issuer,
        })
    }

    pub fn with_scopes(mut self, scopes: &[&str]) -> Self {
        self.scopes = scopes.join(" ");
        self
    }

    pub fn with_state(mut self, state: &str) -> Self {
        self.state = Some(state.to_owned());
        self
    }

    pub fn with_pkce(mut self, challenge: &str) -> Self {
        self.pkce_challenge = Some(challenge.to_owned());
        self
    }

    pub fn with_http_client(mut self, client: Arc<dyn HttpClient>) -> Self {
        self.http_client = client;
        self
    }

    async fn get_jwks(&self) -> Result<std::sync::Arc<jsonwebtoken::jwk::JwkSet>, ConnectError> {
        crate::provider::fetch_and_cache_jwks(&self.jwks_uri, self.http_client.as_ref()).await
    }

    #[tracing::instrument(skip(self, form_data))]
    async fn get_user_from_form(
        &self,
        form_data: &crate::provider::TokenExchangeForm<'_>,
        expected_nonce: Option<&str>,
    ) -> Result<ConnectUser, ConnectError> {
        let token_res = self
            .http_client
            .post(self.token_url())
            .form(form_data)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        let access_token = token_res["access_token"]
            .as_str()
            .ok_or_else(|| ConnectError::Token("Failed to get access_token".to_string()))?;

        let mut user = if let Some(id_token) = token_res["id_token"].as_str() {
            // Cryptographic OIDC Signature Validation
            let header = jsonwebtoken::decode_header(id_token).map_err(|e| {
                crate::error::ConnectError::Provider(format!(
                    "Failed to decode OIDC id_token header: {}",
                    e
                ))
            })?;

            if let Some(kid) = header.kid.as_ref() {
                let jwks = self.get_jwks().await?;
                let jwk = jwks.find(kid).ok_or_else(|| {
                    crate::error::ConnectError::Provider(format!(
                        "OIDC JWK with key ID '{}' not found",
                        kid
                    ))
                })?;
                let decoding_key = jsonwebtoken::DecodingKey::from_jwk(jwk).map_err(|e| {
                    crate::error::ConnectError::Provider(format!(
                        "Failed to build OIDC decoding key from JWK: {}",
                        e
                    ))
                })?;
                let alg = match header.alg {
                    jsonwebtoken::Algorithm::RS256
                    | jsonwebtoken::Algorithm::RS384
                    | jsonwebtoken::Algorithm::RS512
                    | jsonwebtoken::Algorithm::ES256
                    | jsonwebtoken::Algorithm::ES384
                    | jsonwebtoken::Algorithm::EdDSA => header.alg,
                    _ => {
                        return Err(crate::error::ConnectError::Provider(
                            "OIDC token header specifies an insecure or symmetric algorithm"
                                .to_string(),
                        ));
                    }
                };
                let mut validation = jsonwebtoken::Validation::new(alg);
                validation.set_audience(&[&self.client_id]);
                validation.set_issuer(&[&self.issuer]);
                validation.validate_exp = true;
                if expected_nonce.is_some() {
                    validation.set_required_spec_claims(&["nonce"]);
                }

                let token_data =
                    jsonwebtoken::decode::<Value>(id_token, &decoding_key, &validation).map_err(
                        |e| {
                            crate::error::ConnectError::Provider(format!(
                                "OIDC id_token signature or claims validation failed: {}",
                                e
                            ))
                        },
                    )?;
                let payload = token_data.claims;

                if let Some(nonce) = expected_nonce {
                    let token_nonce = payload["nonce"].as_str().unwrap_or("");
                    if !crate::provider::verify_nonce(token_nonce, nonce) {
                        return Err(crate::error::ConnectError::Provider(
                            "OIDC id_token nonce mismatch".to_owned(),
                        ));
                    }
                }

                ConnectUser {
                    id: payload["sub"].as_str().map(String::from).ok_or_else(|| {
                        crate::error::ConnectError::Provider("Missing sub in id_token".to_owned())
                    })?,
                    name: payload["name"].as_str().map(String::from).ok_or_else(|| {
                        crate::error::ConnectError::Provider("Missing name in id_token".to_owned())
                    })?,
                    email: payload["email"].as_str().map(String::from),
                    avatar_url: payload["picture"].as_str().map(String::from),
                    email_verified: payload["email_verified"].as_bool(),
                    raw_data: payload,
                    access_token: secrecy::SecretString::from(access_token.to_owned()),
                    refresh_token: None,
                    expires_in: None,
                }
            } else {
                return Err(crate::error::ConnectError::Provider(
                    "Missing 'kid' header in OIDC id_token".to_owned(),
                ));
            }
        } else {
            use crate::provider::Provider;
            self.get_user_from_token(access_token).await?
        };

        user.refresh_token = token_res["refresh_token"]
            .as_str()
            .map(|s| secrecy::SecretString::from(s.to_string()));
        user.expires_in = token_res["expires_in"]
            .as_u64()
            .or_else(|| token_res["expires_in"].as_i64().map(|v| v as u64));

        Ok(user)
    }
}

#[async_trait]
impl Provider for OidcProvider {
    crate::impl_standard_redirect_url!("{}");

    #[tracing::instrument(skip(self, params))]
    async fn get_user(
        &self,
        params: crate::provider::ExchangeParams<'_>,
    ) -> Result<ConnectUser, ConnectError> {
        let form_data = crate::provider::TokenExchangeForm {
            client_id: self.client_id.as_str(),
            client_secret: Some(secrecy::ExposeSecret::expose_secret(&self.client_secret)),
            code: params.auth_code,
            grant_type: Some("authorization_code"),
            redirect_uri: self.redirect_url.as_str(),
            code_verifier: params.code_verifier,
        };
        self.get_user_from_form(&form_data, params.expected_nonce)
            .await
    }

    #[tracing::instrument(skip(self, access_token))]
    async fn get_user_from_token(&self, access_token: &str) -> Result<ConnectUser, ConnectError> {
        let user_res = self
            .http_client
            .get(&self.userinfo_endpoint)
            .bearer_auth(access_token)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        Ok(ConnectUser {
            id: user_res["sub"].as_str().map(String::from).ok_or_else(|| {
                crate::error::ConnectError::Provider("Missing sub in userinfo".to_owned())
            })?,
            name: user_res["name"].as_str().map(String::from).ok_or_else(|| {
                crate::error::ConnectError::Provider("Missing name in userinfo".to_owned())
            })?,
            email: user_res["email"].as_str().map(String::from),
            avatar_url: user_res["picture"].as_str().map(String::from),
            email_verified: user_res["email_verified"].as_bool(),
            raw_data: user_res,
            access_token: secrecy::SecretString::from(access_token.to_owned()),
            refresh_token: None,
            expires_in: None,
        })
    }

    fn token_url(&self) -> String {
        self.token_endpoint.clone()
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<ConnectUser, ConnectError> {
        let form_data = crate::provider::TokenExchangeForm {
            client_id: self.client_id.as_str(),
            client_secret: Some(secrecy::ExposeSecret::expose_secret(&self.client_secret)),
            code: refresh_token,
            grant_type: Some("refresh_token"),
            redirect_uri: self.redirect_url.as_str(),
            code_verifier: None,
        };
        self.get_user_from_form(&form_data, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{HttpRequest, HttpResponse};
    use serde_json::json;
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
        assert!(
            matches!(err, crate::error::ConnectError::Provider(msg) if msg.contains("issuer_url must be HTTPS (or localhost)"))
        );

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
        assert!(
            matches!(err, crate::error::ConnectError::Provider(msg) if msg.contains("redirect_url must be HTTPS (or localhost)"))
        );

        let err = OidcProvider::discover_with_client(
            "https://issuer",
            "".to_string(),
            "secret".to_string(),
            "https://redirect".to_string(),
            mock_client.clone(),
        )
        .await
        .err()
        .expect("expected error");
        assert!(
            matches!(err, crate::error::ConnectError::Provider(msg) if msg.contains("client_id cannot be empty"))
        );

        let err = OidcProvider::discover_with_client(
            "https://issuer",
            "id".to_string(),
            "".to_string(),
            "https://redirect".to_string(),
            mock_client.clone(),
        )
        .await
        .err()
        .expect("expected error");
        assert!(
            matches!(err, crate::error::ConnectError::Provider(msg) if msg.contains("client_secret cannot be empty"))
        );
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

        assert!(
            matches!(err, crate::error::ConnectError::Provider(msg) if msg.contains("not found"))
        );
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
}

#[cfg(kani)]
mod kani_proofs {
    use subtle::ConstantTimeEq;

    #[kani::proof]
    fn verify_constant_time_eq_safety() {
        let len: usize = kani::any();
        kani::assume(len <= 32);

        let a: [u8; 32] = kani::any();
        let b: [u8; 32] = kani::any();

        let a_slice = &a[..len];
        let b_slice = &b[..len];

        // This proves mathematically that comparing two arbitrary byte slices
        // of the exact same length via subtle::ConstantTimeEq will NEVER panic,
        // crash, or trigger undefined behavior, regardless of the memory layout.
        let _ = a_slice.ct_eq(b_slice);
    }
}
