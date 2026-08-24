/// Defines a standard OAuth2 provider struct and its builder methods.
///
/// This macro generates the boilerplate struct definition, the `new` constructor,
/// and the `with_scopes` / `with_state` builder methods.
#[macro_export]
macro_rules! define_provider {
    ($name:ident) => {
        $crate::define_provider!($name, );
    };
    ($name:ident, $($default_scope:expr),*) => {
        pub struct $name {
            pub(crate) client_id: String,
            pub(crate) client_secret: secrecy::SecretString,
            pub(crate) redirect_url: String,
            pub(crate) http_client: ::std::sync::Arc<dyn $crate::client::HttpClient>,
            pub(crate) scopes: String,
            pub(crate) state: Option<String>,
            pub(crate) pkce_challenge: Option<String>,
            pub(crate) credential_mode: $crate::configuration::CredentialMode,
        }

        impl $name {
            /// Creates a validated provider. Empty or `mock_*` credentials select the
            /// deterministic offline fallback instead of issuing network requests.
            pub fn try_new(
                client_id: impl Into<String>,
                client_secret: secrecy::SecretString,
                redirect_url: impl Into<String>,
            ) -> Result<Self, $crate::error::ConnectError> {
                let client_id = client_id.into();
                let redirect_url = redirect_url.into();
                $crate::configuration::validate_redirect_url(&redirect_url)?;
                let credential_mode =
                    $crate::configuration::credential_mode(&client_id, &client_secret);
                let http_client = $crate::configuration::provider_http_client(
                    credential_mode,
                    stringify!($name),
                    None,
                );

                Ok(Self {
                    client_id,
                    client_secret,
                    redirect_url,
                    http_client,
                    scopes: concat!($($default_scope, " "),*).trim_end().to_string(),
                    state: None,
                    pkce_challenge: None,
                    credential_mode,
                })
            }

            /// Deprecated infallible constructor retained for source compatibility.
            /// Invalid URLs produce a disabled provider whose redirect is `about:blank`
            /// and whose network operations return a typed error.
            #[cfg_attr(
                not(test),
                deprecated(since = "12.0.0", note = "use try_new and handle ConnectError")
            )]
            pub fn new(
                client_id: impl Into<String>,
                client_secret: secrecy::SecretString,
                redirect_url: impl Into<String>,
            ) -> Self {
                let client_id = client_id.into();
                let mut redirect_url = redirect_url.into();
                let validation = $crate::configuration::validate_redirect_url(&redirect_url);
                let (credential_mode, invalid_reason) = match validation {
                    Ok(_) => (
                        $crate::configuration::credential_mode(&client_id, &client_secret),
                        None,
                    ),
                    Err(error) => {
                        redirect_url = "about:blank".to_string();
                        ($crate::configuration::CredentialMode::Invalid, Some(error.to_string()))
                    }
                };
                let http_client = $crate::configuration::provider_http_client(
                    credential_mode,
                    stringify!($name),
                    invalid_reason,
                );

                Self {
                    client_id,
                    client_secret,
                    redirect_url,
                    http_client,
                    scopes: concat!($($default_scope, " "),*).trim_end().to_string(),
                    state: None,
                    pkce_challenge: None,
                    credential_mode,
                }
            }

            /// Returns whether this instance uses live, mock, or disabled credentials.
            pub fn credential_mode(&self) -> $crate::configuration::CredentialMode {
                self.credential_mode
            }

            /// Overrides the default scopes for this provider.
            pub fn with_scopes(mut self, scopes: &[&str]) -> Self {
                self.scopes = scopes.join(" ");
                self
            }

            /// Sets the state parameter for CSRF protection.
            pub fn with_state(mut self, state: impl Into<String>) -> Self {
                self.state = Some(state.into());
                self
            }

            /// Sets the PKCE code_challenge parameter.
            pub fn with_pkce(mut self, challenge: impl Into<String>) -> Self {
                self.pkce_challenge = Some(challenge.into());
                self
            }

            /// Sets a custom HTTP client for live credentials (e.g., a proxy or test transport).
            /// Mock and invalid credentials retain their network-free/fail-closed transport.
            pub fn with_http_client(mut self, client: ::std::sync::Arc<dyn $crate::client::HttpClient>) -> Self {
                if matches!(self.credential_mode, $crate::configuration::CredentialMode::Live) {
                    self.http_client = client;
                }
                self
            }

            /// Configures the built-in HTTP client to use exponential backoff retries.
            /// This is only available when the `retry` feature is enabled.
            #[cfg(feature = "retry")]
            pub fn with_retry(mut self, max_retries: u32) -> Self {
                if matches!(self.credential_mode, $crate::configuration::CredentialMode::Live) {
                    self.http_client = ::std::sync::Arc::new($crate::client::ReqwestClient::new_with_retry(max_retries));
                }
                self
            }
        }
    };
}

#[macro_export]
macro_rules! impl_standard_redirect_url {
    ($url:expr) => {
        fn redirect_url(&self) -> String {
            if self.credential_mode.is_invalid() {
                return "about:blank".to_string();
            }
            if self.credential_mode.is_mock() {
                return $crate::configuration::mock_redirect_url(
                    ::core::any::type_name::<Self>(),
                    self.state.as_deref(),
                    self.pkce_challenge.as_deref(),
                );
            }
            let mut params = $crate::provider::build_oauth_params(
                $url,
                &self.client_id,
                &self.redirect_url,
                &self.scopes,
                self.state.as_deref(),
                self.pkce_challenge.as_deref(),
            );
            params.finish()
        }
    };
}

#[macro_export]
macro_rules! impl_standard_refresh_token {
    () => {
        fn refresh_token<'life0, 'life1, 'async_trait>(
            &'life0 self,
            refresh_token: &'life1 str,
        ) -> ::core::pin::Pin<
            ::std::boxed::Box<
                dyn ::core::future::Future<
                        Output = Result<$crate::user::ConnectUser, $crate::error::ConnectError>,
                    > + ::core::marker::Send
                    + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            'life1: 'async_trait,
            Self: 'async_trait,
        {
            ::std::boxed::Box::pin(async move {
                $crate::provider::refresh_and_get_user(
                    self,
                    self.http_client.as_ref(),
                    &self.token_url(),
                    &self.client_id,
                    &self.client_secret,
                    refresh_token,
                )
                .await
            })
        }
    };
}

#[cfg(all(test, not(miri)))]
#[allow(dead_code)]
mod tests {
    define_provider!(DummyProvider, "default_scope1", "default_scope2");

    #[test]
    fn test_macro_generated_struct_new() {
        let provider = DummyProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect_url".to_string(),
        );

        use secrecy::ExposeSecret;
        assert_eq!(provider.client_id, "client_id");
        assert_eq!(provider.client_secret.expose_secret(), "client_secret");
        assert_eq!(provider.redirect_url, "https://redirect_url");
        assert_eq!(provider.scopes, "default_scope1 default_scope2".to_string());
        assert_eq!(provider.state, None);
        assert_eq!(provider.pkce_challenge, None);
    }

    #[test]
    fn test_macro_fallible_constructor_rejects_lookalike_localhost() {
        let result = DummyProvider::try_new(
            "client_id",
            secrecy::SecretString::from("client_secret".to_string()),
            "http://localhost.evil/callback",
        );
        assert!(matches!(
            result,
            Err(crate::error::ConnectError::InvalidConfiguration {
                field: "redirect_url",
                ..
            })
        ));
    }

    #[test]
    fn test_macro_empty_credentials_select_offline_mode() {
        let provider = DummyProvider::try_new(
            "",
            secrecy::SecretString::from("client_secret".to_string()),
            "https://app.example/callback",
        )
        .expect("mock configuration should be valid");
        assert_eq!(
            provider.credential_mode(),
            crate::configuration::CredentialMode::Mock
        );
    }

    #[test]
    fn test_mock_credentials_cannot_replace_the_offline_transport() {
        let provider = DummyProvider::try_new(
            "",
            secrecy::SecretString::from("".to_string()),
            "https://app.example/callback",
        )
        .expect("mock configuration should be valid");
        let original = provider.http_client.clone();
        let provider =
            provider.with_http_client(std::sync::Arc::new(crate::client::ReqwestClient::new()));

        assert!(std::sync::Arc::ptr_eq(&provider.http_client, &original));
    }

    #[test]
    fn test_legacy_constructor_fails_closed() {
        let provider = DummyProvider::new(
            "client_id",
            secrecy::SecretString::from("client_secret".to_string()),
            "http://example.com/callback",
        );
        assert_eq!(
            provider.credential_mode(),
            crate::configuration::CredentialMode::Invalid
        );
        assert_eq!(provider.redirect_url, "about:blank");
    }

    #[test]
    fn test_macro_generated_struct_with_scopes() {
        let provider = DummyProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect_url".to_string(),
        )
        .with_scopes(&["new_scope1", "new_scope2"]);

        assert_eq!(provider.scopes, "new_scope1 new_scope2".to_string());
    }

    #[test]
    fn test_macro_generated_struct_with_state() {
        let provider = DummyProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect_url".to_string(),
        )
        .with_state("my_state");

        assert_eq!(provider.state, Some("my_state".to_string()));
    }

    #[test]
    fn test_macro_generated_struct_with_pkce() {
        let provider = DummyProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect_url".to_string(),
        )
        .with_pkce("my_pkce_challenge");

        assert_eq!(
            provider.pkce_challenge,
            Some("my_pkce_challenge".to_string())
        );
    }

    #[test]
    fn test_macro_generated_struct_with_http_client() {
        let client = std::sync::Arc::new(crate::client::ReqwestClient::new());
        let provider = DummyProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect_url".to_string(),
        )
        .with_http_client(client);

        // We can't directly check the client, but we can verify the builder method chain works
        assert_eq!(provider.client_id, "client_id");
    }

    #[test]
    #[cfg(feature = "retry")]
    fn test_macro_generated_struct_with_retry() {
        let provider = DummyProvider::new(
            "client_id".to_string(),
            secrecy::SecretString::from("client_secret".to_string()),
            "https://redirect_url".to_string(),
        )
        .with_retry(3);

        // Verifying builder method works
        assert_eq!(provider.client_id, "client_id");
    }
}
