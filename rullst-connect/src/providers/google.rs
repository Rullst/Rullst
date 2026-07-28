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
        MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCxbATI3BfP+iG3\n\
        oFVmomIagpUVFbJ56yBKAlhwzgbBZL9jjmISRl1ti4+V7A5GFXPwkTu19OZVUidG\n\
        YsMCgAR5MH6Setk+NovSmhmYKIIZts05ryVRc9skmtgVtCUaHAw4F9k6l4lyNq3f\n\
        G4lG2qSZqrr9sa70F8kZQ7DNnes/zfkkKGpH+g6ZSlwLoQ9H0EHSl2u8Fhn+2Wwq\n\
        GCY0kjp20rU19BOIivi2Pj0iJJpUIjZzIjEp/aTNa7mP1h+oq1xNSWpCQ7bpYJx0\n\
        GM7Y0tGJXRKf5AsY+VAnSgOZCMP9JQ73Gew2a0ilt2Qfon6esofTBb0VBbv3rXtS\n\
        nqpu7tcFAgMBAAECggEACJNjzNCknwMkX5OSpS4ap1Th/12n3ZBBpIol+7fdqDnm\n\
        LVn7wA3iJLIe+9xn2TfevYVLmVEv0/ZvWw3ZhrSo4rG3IH3rI8BvtDuKGqp0lWka\n\
        VNw0JdJ/iG6qnKvzMiaiaZCviY87D8f4UgUq9r+JPrs7pBkTT42ZxQyaTmoAbbpC\n\
        gE5afzy801AdqmzrlrxNzZJpmfS0CsLKKQub49LDlh84h6z7X7Iv2Lbq2gxvEcgt\n\
        h5uyIduciICtCZYtkaNbC1lnj/ecBSoAWKOSOCP1ISg65LjZwvhiVB4jgZUhrXsp\n\
        9FscocRWe92HunCeEt0IB+Q+JMkO/e9rFeSfMiIYkQKBgQDvCPrGM2IaKCfkVpXs\n\
        zZcYKNoukL7ID/H+LWByoQHF1O6tBI5VAuduEB5K/J4SrqZcWC1PdiscEdRQh5Ik\n\
        Q2Jrpg2djzCxg6rDI88piOm9+UBH09YjxIITYI9q74prsWC+8dJbiGruOWEZp4On\n\
        VXuE7OE9aUcNkkpRV/gOcvwGsQKBgQC+A5eax4nJfCZ9Zroypd8sN/7i/bJbkr+s\n\
        38jRtJreJRahOTccO3I9yDo0idVlLoubFlokl55gjwu4IOBYbY3mg8Ka8shZe5v8\n\
        x6MZPlwDXt4OTR2QEJcN3QQgon3wpG7gbzjRR8syi4fDESe1kVVTRnXIS6kKfBAl\n\
        SXLHpM6SlQKBgG9LbQevkPPA0qIcNn4lUz5qdvvLZSjdU70W/5sfoCWueNqSDntC\n\
        eOLkGlarvCXSr567Z41h5bySCJreJItB3Kdmj1xW+UMNnQpyt9gM6VgMn4NR/Jh2\n\
        vGGtSdluYrK1yefdzCXWJIN6r900A7Z7tKE1ccIYLH8DKBsrrFF99B5hAoGAJqsi\n\
        ehwrXTaHurNiJxZ8cUo/87+/QUV+/lZYTtzbO2P+0/aJ0ZQDbrFFrxVxuPKc9IW6\n\
        +IFmeK4Dq4f9P+GjpAqiWtgXj6ZJG0shVOzM2t6+f9iPsJa/ttGImn+W85by/XeE\n\
        74oVvwaILVlbZGbcH2NR9aW4E+slegEVe619YHUCgYBwOo5Czo4tDLTI0AYwx/on\n\
        a1JhoEeS1o3f/BfneofQqOFFeATUmU52tidm8G4wSGfkCKRtj3w8JP+gpSXJ6v6G\n\
        imOfWTISM8OrQHS4RmPK+mRor4a7Pf930DCF6W2PRXZgYdBw7Gs6TnClpy5RXslE\n\
        LQR3iiL0OLIZDwiYlfBWLA==\n\
        -----END PRIVATE KEY-----";

        let n_val = "sWwEyNwXz_oht6BVZqJiGoKVFRWyeesgSgJYcM4GwWS_Y45iEkZdbYuPlewORhVz8JE7tfTmVVInRmLDAoAEeTB-knrZPjaL0poZmCiCGbbNOa8lUXPbJJrYFbQlGhwMOBfZOpeJcjat3xuJRtqkmaq6_bGu9BfJGUOwzZ3rP835JChqR_oOmUpcC6EPR9BB0pdrvBYZ_tlsKhgmNJI6dtK1NfQTiIr4tj49IiSaVCI2cyIxKf2kzWu5j9YfqKtcTUlqQkO26WCcdBjO2NLRiV0Sn-QLGPlQJ0oDmQjD_SUO9xnsNmtIpbdkH6J-nrKH0wW9FQW79617Up6qbu7XBQ";
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
