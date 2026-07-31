use crate::client::HttpClientExt;
use crate::provider::Provider;
use crate::user::ConnectUser;
use async_trait::async_trait;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct AppleProvider {
    client_id: String,
    team_id: String,
    key_id: String,
    private_key_pem: String,
    redirect_url: String,
    http_client: ::std::sync::Arc<dyn crate::client::HttpClient>,
    scopes: String,
    state: Option<String>,
    pkce_challenge: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppleClaims<'a> {
    iss: &'a str,
    iat: u64,
    exp: u64,
    aud: &'a str,
    sub: &'a str,
}

impl AppleProvider {
    /// Apple requires a Team ID, a Key ID, and the contents of a .p8 Private Key file
    /// to dynamically generate the client_secret JWT on every login.
    pub fn new(
        client_id: String,
        team_id: String,
        key_id: String,
        private_key_pem: String,
        redirect_url: String,
    ) -> Self {
        Self {
            client_id,
            team_id,
            key_id,
            private_key_pem,
            redirect_url,
            http_client: crate::client::DEFAULT_HTTP_CLIENT.clone(),
            scopes: "name email".to_string(),
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

    fn generate_client_secret(&self) -> Result<String, crate::error::ConnectError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let claims = AppleClaims {
            iss: &self.team_id,
            iat: now,
            exp: now + 300, // 5 minutes expiration (short-lived credential)
            aud: "https://appleid.apple.com",
            sub: &self.client_id,
        };

        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.key_id.clone());

        let encoding_key = EncodingKey::from_ec_pem(self.private_key_pem.as_bytes())?;
        let token = encode(&header, &claims, &encoding_key)?;

        Ok(token)
    }

    async fn get_jwks(
        &self,
    ) -> Result<std::sync::Arc<jsonwebtoken::jwk::JwkSet>, crate::error::ConnectError> {
        crate::provider::fetch_and_cache_jwks(
            "https://appleid.apple.com/auth/keys",
            self.http_client.as_ref(),
        )
        .await
    }

    async fn get_user_from_form(
        &self,
        form_data: &crate::provider::TokenExchangeForm<'_>,
        expected_nonce: Option<&str>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let token_res = self
            .http_client
            .post("https://appleid.apple.com/auth/token")
            .form(form_data)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        // Apple returns user data inside an "id_token" (JWT)
        let id_token_str = token_res["id_token"].as_str().ok_or_else(|| {
            crate::error::ConnectError::Token("Failed to get id_token from Apple".to_string())
        })?;
        let access_token = token_res["access_token"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| {
                crate::error::ConnectError::Token(
                    "Failed to get access_token from Apple".to_string(),
                )
            })?;

        let mut user = self
            .decode_apple_id_token(id_token_str, expected_nonce)
            .await?;
        user.access_token = access_token.into();
        user.refresh_token = token_res["refresh_token"]
            .as_str()
            .map(|s| secrecy::SecretString::from(s.to_string()));
        user.expires_in = token_res["expires_in"]
            .as_u64()
            .or_else(|| token_res["expires_in"].as_i64().map(|v| v as u64));
        Ok(user)
    }

    async fn decode_apple_id_token(
        &self,
        id_token_str: &str,
        expected_nonce: Option<&str>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let mut payload: Option<Value> = None;

        if let Ok(header) = jsonwebtoken::decode_header(id_token_str)
            && let Some(kid) = header.kid.as_ref()
            && let Ok(jwks) = self.get_jwks().await
            && let Some(jwk) = jwks.find(kid)
            && let Ok(decoding_key) = jsonwebtoken::DecodingKey::from_jwk(jwk)
        {
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
            validation.set_issuer(&["https://appleid.apple.com"]);
            validation.validate_exp = true;
            if expected_nonce.is_some() {
                validation.set_required_spec_claims(&["nonce"]);
            }

            if let Ok(token_data) =
                jsonwebtoken::decode::<Value>(id_token_str, &decoding_key, &validation)
            {
                payload = Some(token_data.claims);
            }
        }

        let payload = match payload {
            Some(p) => p,
            None => {
                return Err(crate::error::ConnectError::Provider(
                    "Failed to verify Apple id_token signature or claims".to_string(),
                ));
            }
        };

        if let Some(nonce) = expected_nonce {
            let token_nonce = payload["nonce"].as_str().unwrap_or("");
            if !crate::provider::verify_nonce(token_nonce, nonce) {
                return Err(crate::error::ConnectError::Provider(
                    "Apple id_token nonce mismatch".to_owned(),
                ));
            }
        }

        Ok(ConnectUser {
            id: payload["sub"].as_str().map(String::from).ok_or_else(|| {
                crate::error::ConnectError::Provider("Missing sub in Apple id_token".to_string())
            })?,
            name: String::with_capacity(256), // Developer needs to extract this from the form_post on first login
            email: payload["email"].as_str().map(String::from),
            avatar_url: None, // Apple does not provide avatars
            email_verified: None,
            raw_data: payload,
            access_token: id_token_str.to_string().into(),
            refresh_token: None,
            expires_in: None,
        })
    }
}

#[async_trait]
impl Provider for AppleProvider {
    fn redirect_url(&self) -> String {
        let mut params = crate::provider::build_oauth_params(
            "https://appleid.apple.com/auth/authorize",
            &self.client_id,
            &self.redirect_url,
            &self.scopes,
            self.state.as_deref(),
            self.pkce_challenge.as_deref(),
        );
        params.append_pair("response_type", "code");
        params.append_pair("response_mode", "form_post");
        params.finish()
    }

    async fn get_user(
        &self,
        params: crate::provider::ExchangeParams<'_>,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let client_secret = self.generate_client_secret()?;
        let form_data = crate::provider::TokenExchangeForm {
            client_id: self.client_id.as_str(),
            client_secret: Some(client_secret.as_str()),
            code: params.auth_code,
            grant_type: Some("authorization_code"),
            redirect_uri: self.redirect_url.as_str(),
            code_verifier: params.code_verifier,
        };
        self.get_user_from_form(&form_data, params.expected_nonce)
            .await
    }

    /// For Apple, `access_token` parameter should actually be the `id_token` JWT string.
    async fn get_user_from_token(
        &self,
        id_token_str: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        self.decode_apple_id_token(id_token_str, None).await
    }

    fn token_url(&self) -> String {
        "https://appleid.apple.com/auth/token".to_string()
    }

    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<ConnectUser, crate::error::ConnectError> {
        let client_secret = self.generate_client_secret()?;

        let token_res = self
            .http_client
            .post(self.token_url())
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", client_secret.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;

        if let Some(err) = token_res["error"].as_str() {
            let err_desc = token_res["error_description"].as_str().unwrap_or_default();
            return Err(crate::error::ConnectError::Token(format!(
                "Provider returned error: {} - {}",
                err, err_desc
            )));
        }

        let access_token = token_res["access_token"].as_str().ok_or_else(|| {
            crate::error::ConnectError::Token(
                "Failed to get access_token during refresh".to_string(),
            )
        })?;

        let mut user = self.get_user_from_token(access_token).await?;
        user.refresh_token = token_res["refresh_token"]
            .as_str()
            .map(|s| secrecy::SecretString::from(s.to_string()));
        user.expires_in = token_res["expires_in"]
            .as_u64()
            .or_else(|| token_res["expires_in"].as_i64().map(|v| v as u64));
        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                assert!(msg.contains("Failed to verify Apple id_token"));
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

    use crate::client::{HttpClient, HttpRequest, HttpResponse};
    use serde_json::json;
    use std::sync::Arc;

    struct MockAppleClient {
        token_status: u16,
        token_body: serde_json::Value,
    }

    #[async_trait]
    impl HttpClient for MockAppleClient {
        async fn execute(
            &self,
            req: HttpRequest,
        ) -> Result<HttpResponse, crate::error::ConnectError> {
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
        // We use a valid-looking PEM just so it parses, but then the HTTP request fails.
        // Or we can just call get_user_from_form directly which is private to the module.
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
        assert!(
            matches!(err, crate::error::ConnectError::Token(msg) if msg.contains("access_token"))
        );
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
}
