### Deprecated

- The `oauth_state` session-key fallback of `rullst-connect`'s `AuthSession`
  and `AuthSessionForm`: a state that the application stored itself under
  that key before 12.0, which has no expiry, PKCE verifier or OIDC nonce. It
  still works and is kept through 13.x, but each callback that uses it now
  logs a `tracing` warning (without the state). Start the login with
  `begin_oauth_session` or `begin_oidc_session`, or use the store-backed
  `challenge::begin_login` and `finish_login`.
