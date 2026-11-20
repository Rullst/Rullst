### Security

- The `rullst-connect` Axum examples (`axum_server`, described as a complete
  server, and `axum_example`) redirected without a verified OAuth `state`
  (login CSRF: a callback with an attacker's code was accepted), sent no PKCE
  or nonce, and `axum_server` wrote the provider's profile name into HTML
  unescaped (XSS). Both now use `begin_login`/`finish_login` with a
  `MemoryChallengeStore` (state, PKCE and, for Google, the OIDC nonce, with a
  single-use challenge cookie) and escape every profile field. They need the
  `axum` feature: `cargo run -p rullst-connect --example axum_server
  --features axum,mock`. `axum_server`'s offline tests now run in
  `cargo test` and check that a callback without its challenge, with a
  swapped state or replayed is refused.
