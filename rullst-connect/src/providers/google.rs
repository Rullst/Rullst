use crate::client::HttpClientExt;
use crate::provider::Provider;
use crate::user::ConnectUser;
use async_trait::async_trait;
use serde_json::Value;

pub struct GoogleProvider {
    pub(crate) client_id: String,
    pub(crate) client_secret: secrecy::SecretString,
    pub(crate) redirect_url: String,
    pub(crate) http_client: ::std::sync::Arc<dyn crate::client::HttpClient>,
    pub(crate) scopes: String,
    pub(crate) state: Option<String>,
    pub(crate) pkce_challenge: Option<String>,
}

#[derive(serde::Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    id_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

impl GoogleProvider {
    pub fn new(
        client_id: String,
        client_secret: secrecy::SecretString,
        redirect_url: String,
    ) -> Self {
        assert!(
            !client_id.is_empty(),
            "Socialite Error: client_id cannot be empty"
        );
        assert!(
            !secrecy::ExposeSecret::expose_secret(&client_secret).is_empty(),
            "Socialite Error: client_secret cannot be empty"
        );
        assert!(
            redirect_url.starts_with("http"),
            "Socialite Error: redirect_url must be a valid HTTP/HTTPS URL"
        );

        Self {
            client_id,
            client_secret,
            redirect_url,
            http_client: crate::client::DEFAULT_HTTP_CLIENT.clone(),
            scopes: "openid profile email".to_string(),
            state: None,
            pkce_challenge: None,
        }
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

    pub fn with_http_client(
        mut self,
        client: ::std::sync::Arc<dyn crate::client::HttpClient>,
    ) -> Self {
        self.http_client = client;
        self
    }

    #[cfg(feature = "retry")]
    pub fn with_retry(mut self, max_retries: u32) -> Self {
        self.http_client =
            ::std::sync::Arc::new(crate::client::ReqwestClient::new_with_retry(max_retries));
        self
    }

    async fn get_jwks(
        &self,
    ) -> Result<std::sync::Arc<jsonwebtoken::jwk::JwkSet>, crate::error::ConnectError> {
        crate::provider::fetch_and_cache_jwks(
            "https://www.googleapis.com/oauth2/v3/certs",
            self.http_client.as_ref(),
        )
        .await
    }

    async fn get_user_from_form(
        &self,
        form_data: &crate::provider::TokenExchangeForm<'_>,
        expected_nonce: Option<&str>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        // Exchange code for token
        let token_res = self
            .http_client
            .post("https://oauth2.googleapis.com/token")
            .form(form_data)
            .send()
            .await?
            .error_for_status()?
            .json::<GoogleTokenResponse>()
            .await?;

        let access_token = token_res.access_token;

        let mut user = if let Some(id_token) = &token_res.id_token {
            // Secure OIDC: Verify the signature of Google's id_token
            let header = jsonwebtoken::decode_header(id_token).map_err(|e| {
                crate::error::ConnectError::Provider(format!(
                    "Failed to decode Google id_token header: {}",
                    e
                ))
            })?;

            if let Some(kid) = header.kid.as_ref() {
                let jwks = self.get_jwks().await?;
                let jwk = jwks.find(kid).ok_or_else(|| {
                    crate::error::ConnectError::Provider(format!(
                        "Google JWK with key ID '{}' not found",
                        kid
                    ))
                })?;
                let decoding_key = jsonwebtoken::DecodingKey::from_jwk(jwk).map_err(|e| {
                    crate::error::ConnectError::Provider(format!(
                        "Failed to build Google decoding key: {}",
                        e
                    ))
                })?;

                let alg = match header.alg {
                    jsonwebtoken::Algorithm::RS256 => jsonwebtoken::Algorithm::RS256,
                    _ => {
                        return Err(crate::error::ConnectError::Provider(
                            "Unsupported algorithm in id_token header".to_string(),
                        ));
                    }
                };
                let mut validation = jsonwebtoken::Validation::new(alg);
                validation.set_audience(&[&self.client_id]);
                validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);
                validation.validate_exp = true;
                if expected_nonce.is_some() {
                    validation.set_required_spec_claims(&["nonce"]);
                }

                let token_data =
                    jsonwebtoken::decode::<Value>(id_token, &decoding_key, &validation).map_err(
                        |e| {
                            crate::error::ConnectError::Provider(format!(
                                "Google id_token validation failed: {}",
                                e
                            ))
                        },
                    )?;

                let p = token_data.claims;

                if let Some(nonce) = expected_nonce {
                    let token_nonce = p["nonce"].as_str().unwrap_or("");
                    if !crate::provider::verify_nonce(token_nonce, nonce) {
                        return Err(crate::error::ConnectError::Provider(
                            "Google id_token nonce mismatch".to_owned(),
                        ));
                    }
                }

                ConnectUser {
                    id: p["sub"].as_str().map(String::from).ok_or_else(|| {
                        crate::error::ConnectError::Provider(
                            "Missing sub claim in Google id_token".to_owned(),
                        )
                    })?,
                    name: p["name"].as_str().map(String::from).unwrap_or_default(),
                    email: p["email"].as_str().map(String::from),
                    avatar_url: p["picture"]
                        .as_str()
                        .map(|s: &str| s.replace("=s96-c", "=s400-c")),
                    email_verified: p["email_verified"].as_bool(),
                    raw_data: p,
                    access_token: access_token.into(),
                    refresh_token: None,
                    expires_in: None,
                }
            } else {
                return Err(crate::error::ConnectError::Provider(
                    "Missing 'kid' header in Google id_token".to_owned(),
                ));
            }
        } else {
            self.get_user_from_token(&access_token).await?
        };

        user.refresh_token = token_res.refresh_token.map(secrecy::SecretString::from);
        user.expires_in = token_res.expires_in;
        Ok(user)
    }
}

#[async_trait]
impl Provider for GoogleProvider {
    crate::impl_standard_redirect_url!("https://accounts.google.com/o/oauth2/v2/auth");

    async fn get_user(
        &self,
        params: crate::provider::ExchangeParams<'_>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
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

    async fn get_user_from_token(
        &self,
        access_token: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        // Fetch user profile
        let user_res = self
            .http_client
            .get("https://www.googleapis.com/oauth2/v3/userinfo")
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
            name: user_res["name"]
                .as_str()
                .map(String::from)
                .unwrap_or_default(),
            email: user_res["email"].as_str().map(String::from),
            avatar_url: user_res["picture"]
                .as_str()
                .map(|s: &str| s.replace("=s96-c", "=s400-c")),
            email_verified: user_res["email_verified"].as_bool(),
            raw_data: user_res,
            access_token: secrecy::SecretString::from(access_token.to_owned()),
            refresh_token: None,
            expires_in: None,
        })
    }

    async fn revoke_token(&self, token: &str) -> Result<(), crate::error::ConnectError> {
        self.http_client
            .post("https://oauth2.googleapis.com/revoke")
            .form(&[("token", token)])
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    fn token_url(&self) -> String {
        "https://oauth2.googleapis.com/token".to_string()
    }

    crate::impl_standard_refresh_token!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Provider;

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

    use crate::client::{HttpClient, HttpRequest, HttpResponse};
    use serde_json::json;
    use std::sync::Arc;

    struct MockGoogleClient {
        token_status: u16,
        token_body: serde_json::Value,
        user_status: u16,
        user_body: serde_json::Value,
    }

    #[async_trait]
    impl HttpClient for MockGoogleClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
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

        assert!(
            matches!(err, crate::error::ConnectError::Provider(msg) if msg.contains("not found"))
        );
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
}
