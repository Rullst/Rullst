# Rullst Connect 🦀

> **v12 development notice:** This README documents the unreleased v12 source.
> Use a path dependency from this checkout until an immutable v12 RC exists on
> crates.io. The version below is the planned first RC, not a published claim.

[![Crates.io](https://img.shields.io/crates/v/rullst-connect.svg?style=for-the-badge&logo=rust)](https://crates.io/crates/rullst-connect)
[![Downloads](https://img.shields.io/crates/d/rullst-connect.svg?style=for-the-badge)](https://crates.io/crates/rullst-connect)
[![Documentation](https://img.shields.io/docsrs/rullst-connect?style=for-the-badge&logo=docs.rs)](https://docs.rs/rullst-connect)
[![Build](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=for-the-badge&logo=github)](https://github.com/Rullst/Rullst/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/rullst-connect?style=for-the-badge)](https://opensource.org/licenses/MIT)

**Rullst Connect** is an elegant, async-first, and Developer Experience (DX) focused OAuth2 authentication library for Rust. It simplifies the integration of social logins into your Rust web applications, providing a standardized interface across multiple providers.

## 🛡️ Security Engineering

Rullst Connect uses layered tests and repository security checks. CI badges report the
state of those checks for the referenced commit; they are not an absolute security guarantee.

| Security Audit | Status | Description |
| :--- | :---: | :--- |
| **OpenSSF Scorecard** | [![OpenSSF Scorecard](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fapi.scorecard.dev%2Fprojects%2Fgithub.com%2FRullst%2FRullst&query=%24.score&label=OpenSSF%20Scorecard&style=flat-square)](https://scorecard.dev/viewer/?uri=github.com/Rullst/Rullst) | Current public supply-chain practice score; not a security certification |
| **Codecov** | [![Framework library coverage](https://codecov.io/github/Rullst/Rullst/branch/main/graph/badge.svg?component=framework_libraries)](https://codecov.io/gh/Rullst/Rullst) | Blocking 90% target for the measured framework-library scope; repository aggregate remains separately visible |
| **OpenSSF Best Practices** | [![OpenSSF Best Practices](https://www.bestpractices.dev/projects/13360/badge)](https://www.bestpractices.dev/projects/13360) | Current project badge from the official programme |
| **Release Provenance** | [![Release](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/release.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/release.yml) | Provenance attestations for release artifacts; no SLSA level is claimed here |
| **On-demand fuzzing** | [![Fuzzing](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/fuzzing.yml?branch=main&style=flat-square&label=Fuzzing)](https://github.com/Rullst/Rullst/actions/workflows/fuzzing.yml) | Manual time-bounded targets; no continuous OSS-Fuzz claim |
| **Property tests** | [![Proptest](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/proptest.yml?branch=main&style=flat-square&label=Proptest)](https://github.com/Rullst/Rullst/actions/workflows/proptest.yml) | Scheduled/manual bounded invariant evidence |
| **Miri research matrix** | [![Miri](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/miri.yml?branch=main&style=flat-square&label=Miri)](https://github.com/Rullst/Rullst/actions/workflows/miri.yml) | Manual, partly non-blocking evidence for the targets actually exercised |
| **Kani research harnesses** | [![Kani](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/kani.yml?branch=main&style=flat-square&label=Kani)](https://github.com/Rullst/Rullst/actions/workflows/kani.yml) | Manual bounded formal evidence, not proof of the complete protocol surface |
| **CodeQL SAST** | [![CodeQL](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/codeql.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/codeql.yml) | Advanced semantic code analysis |
| **Cargo Deny** | [![Cargo Deny](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/cargo-deny.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/cargo-deny.yml) | Banning unmaintained/vulnerable crates |
| **Cargo Audit** | [![Cargo Audit](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/audit.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/audit.yml) | Continuous scanning for crate vulnerabilities |
| **Benchmark CI** | [![Benchmark](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/bench.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/bench.yml) | Continuous performance regression testing |
| **Cargo SemVer** | [![Semver Checks](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/semver.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/semver.yml) | Strict SemVer API breakage checks |
| **Cargo Machete** | [![Machete](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/machete.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/machete.yml) | Detecting unused and bloated dependencies |
| **Spellcheck CI** | [![Spellcheck](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/spellcheck.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/spellcheck.yml) | Automated typo detection across docs and code |
| **Mutation Testing** | [![Mutants](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/mutants.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/mutants.yml) | Mutation testing for test suite robustness |
| **Secret Scanning** | [![Trufflehog](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/trufflehog.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/trufflehog.yml) | Automated CI prevention of leaked credentials |
| **Unsafe Policy** | [![Unsafe](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/unsafe-policy.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/unsafe-policy.yml) | Audits unsafe usage within the workflow's declared scope |
| **Panic Policy** | [![Panics](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/zero-panics.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/zero-panics.yml) | Graceful error handling across the framework |

## ✨ Features

- 🚀 **Async & Fast**: Built on top of `tokio` and `reqwest`.
- 🧩 **Standardized**: All providers return a unified `ConnectUser` struct.
- 🛡️ **Type-Safe**: Robust error handling using `thiserror` (`ConnectError`).
- 🔌 **Framework-neutral core**: Native callback extractors exist for Axum and
  Actix; `AuthCallback` remains a plain deserializable type for other hosts.
- 🔐 **Managed callback transaction**: The optional Axum/tower-sessions path
  generates, stores, expires, validates, and consumes state + PKCE + OIDC nonce.
- 🔄 **Bounded automatic refresh**: A user-bound, redacting static coordinator
  detects token expiry, prevents overlapping refresh calls and reuses a valid
  result for waiting callers.
- 🚪 **Typed remote revocation**: Access and refresh tokens are distinct API
  operations. Google, GitHub, Discord, Apple, Auth0 and Cognito have bounded
  protocol adapters; unsupported providers fail explicitly and offline
  credentials remain network-free.
- 🔐 **OIDC Security**: Strict discovery validation plus isolated JWKS caches with TTL, refresh on unknown `kid`, and bounded stale-if-error behavior.
- 🏢 **Explicit Corporate Proxy**: First-class HTTP(S) proxy clients, including bounded Basic proxy authentication without credentials in the endpoint URL.
- 📺 **Device Flow**: Native RFC 8628 support for headless CLI and Smart TV auth.
- 🛠️ **Testing**: Typed network-free provider fallbacks plus an explicitly
  mounted, loopback-only signed OIDC fixture with one-shot codes, PKCE, nonce,
  EdDSA ID tokens and JWKS.

> 📚 **Important Documents:**
> - [CHANGELOG.md](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md): See what's new.
> - [ISSUES](https://github.com/Rullst/Rullst/issues): Any issue? Please report.
> - [AUDIT.md](https://github.com/Rullst/Rullst/blob/main/AUDIT.md): Repository audit record; current workflow evidence remains authoritative.

## 📦 Supported Providers

Official support for 11 core providers:

1. **Google**
2. **GitHub**
3. **Microsoft / Azure AD**
4. **Apple** (Sign in with Apple)
5. **Auth0**
6. **AWS Cognito**
7. **Facebook**
8. **X (Twitter)** (Strict PKCE requirement)
9. **Discord**
10. **LinkedIn**
11. **OIDC (OpenID Connect Custom Provider)**

Remote token revocation is deliberately narrower than login support. Use
`Provider::revoke_token` for an access token and
`Provider::revoke_refresh_token` for a refresh token; Auth0/Cognito accept only
the latter through these adapters, while GitHub accepts only the former. A
successful provider response does not clear application cookies, sessions,
cached identity, or durable token records—the host must commit that local
logout lifecycle. Provider revocation endpoints are intentionally idempotent in
several ecosystems, so HTTP success is protocol acceptance, not proof that the
token was valid immediately before the call.

## 🛠️ Installation

Add the package to your `Cargo.toml`. Native callback extractors are implemented
for Axum and Actix. Other hosts can deserialize the framework-neutral
`AuthCallback`; the compatibility `leptos` feature adds no Leptos runtime or
extractor.

After the RC is published, install its exact version with:
```bash
cargo add rullst-connect@12.0.0-rc.1
```

For the recommended Axum flow:

```toml
rullst-connect = { version = "12.0.0-rc.1", features = ["axum-session"] }
tower-sessions = "0.15"
```

Or manually add it to your `Cargo.toml`:
```toml
[dependencies]
rullst-connect = "12.0.0-rc.1"
tokio = { version = "1.52", features = ["full"] }
```

## 🚀 Quick Start

### 1. Initialize the Provider
Choose your provider and pass your credentials and callback URL:

```rust
use rullst_connect::prelude::*;

let github = GithubProvider::try_new(
    "YOUR_CLIENT_ID",
    SecretString::from("YOUR_CLIENT_SECRET".to_string()),
    "http://localhost:3000/auth/github/callback",
)?;
```

`try_new` validates the callback URL without panicking. HTTPS is required except for
the exact loopback hosts `localhost`, `127.0.0.1`, and `::1`; names such as
`localhost.evil` are rejected. The older infallible `new` constructor is deprecated and
returns a disabled, fail-closed provider when configuration is invalid.

### Credential modes and offline tests

Every provider exposes `credential_mode() -> CredentialMode`. An empty credential or a
value beginning with `mock_` selects `CredentialMode::Mock` and installs a transport that
never accesses the network. Functional mock identities are available only in unit tests
or when the Cargo feature `mock` is explicitly enabled; otherwise token operations return
`ConnectError::Offline`. This prevents missing production secrets from silently becoming
a working authentication bypass.

```toml
[dev-dependencies]
rullst-connect = { version = "12.0.0-rc.1", features = ["mock"] }
```

Mock-mode redirects use the reserved `example.invalid` domain and mock profiles use the
reserved `example.invalid` email domain.

### Signed local OIDC fixture

With the `axum` feature, applications can mount a real local protocol fixture:

```rust,no_run
use rullst_connect::mock_idp::{MockIdpConfig, mock_router_with_config};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let issuer = "http://127.0.0.1:8080";
let config = MockIdpConfig::try_new(
    issuer,
    "local-test-client",
    "local-test-secret",
    "http://127.0.0.1:3000/auth/callback",
)?;
let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
axum::serve(listener, mock_router_with_config(config)).await?;
# Ok(())
# }
```

The fixture exercises discovery, exact client/callback binding, expiring
one-shot codes, optional nonce, S256 PKCE, EdDSA ID-token verification, JWKS and
bearer-protected userinfo. The signing seed and credentials are predictable test
material. Keep the listener on loopback; this is not a login UI, consent server,
refresh-token service, federation implementation or conformance suite. See the
[local OIDC testing tutorial](https://rullst.github.io/Rullst/book/tutorials/48-local-oidc-testing.html).

### Explicit corporate proxy

Live providers can receive a first-class proxy-aware transport without relying
on ambient `HTTP_PROXY` state:

```rust
use rullst_connect::client::ReqwestClient;
use std::sync::Arc;

let proxy = ReqwestClient::try_with_proxy_basic_auth(
    "https://proxy.corp.example:8443",
    "proxy-user",
    proxy_password,
)?;
let github = github.with_http_client(Arc::new(proxy));
```

Proxy URLs are limited to an HTTP(S) scheme and authority, with no embedded
credentials, path, query, or fragment. Authenticated non-loopback proxies must
use HTTPS. The configured client uses only that explicit proxy; PAC/WPAD,
SOCKS, proxy mTLS identity and deployment certification remain outside this
bounded transport.

### 2. Start a Server-Bound Authorization

The recommended Axum path generates state and PKCE, stores their private
counterparts in `tower-sessions` for ten minutes, and returns only the redirect
URL:

```rust
use axum::response::Redirect;
use rullst_connect::extractors::begin_oauth_session;
use tower_sessions::Session;

let authorization = begin_oauth_session(&session, &github).await?;
return Ok(Redirect::temporary(authorization.url()));
```

Use `begin_oidc_session` instead for Google, Apple, or a custom OIDC provider.
It adds and stores an OIDC nonce as well.

### 3. Consume the Callback and Get the User

`AuthSession` consumes the challenge before checking its expiry and state. It
then supplies the exact PKCE verifier and optional OIDC nonce to the provider:

```rust
use rullst_connect::extractors::AuthSession;

let params = auth_session.exchange_params()?;
let user = github.get_user(params).await?;
let public_profile = user.universal_profile();
```

Only one managed challenge is active per browser session. Starting another
login deliberately invalidates the earlier tab. The application must configure
a durable production session store, Secure/HttpOnly/SameSite cookies, TLS,
registered redirect URLs, account-linking policy, and post-login session
rotation. See the
[server-bound OAuth/OIDC tutorial](https://rullst.github.io/Rullst/book/tutorials/42-server-bound-oauth-sessions.html).

### Safe profile serialization

`ConnectUser` contains live credentials. Its Serde representation deliberately
omits `access_token` and `refresh_token`; do not depend on serializing it as a
token store. Use the normalized, credential-free projection for responses,
sessions that only need identity data, or database mapping:

```rust
let user = github.get_user(params).await?;
let profile: rullst_connect::UniversalProfile = user.universal_profile();
let public_json = serde_json::to_string(&profile)?;
```

`UniversalProfile` contains only `id`, `name`, `email`, `email_verified`, and
`avatar_url`. Persist provider tokens separately in an encrypted secret store
with an application-defined rotation and revocation lifecycle.

### 🛡️ Manual State Handling

Non-Axum hosts may use the framework-neutral primitives directly. The host must
atomically take the expected value from a short-lived server-side store before
comparison; a reusable cookie value is not equivalent.

```rust
use rullst_connect::pkce::generate_oauth_state;

let state = generate_oauth_state();
store_one_time_state(&state).await?; // application-provided durable operation
let url = github.redirect_url_with_state(&state);

let expected = take_one_time_state().await?; // atomically removes it
callback.verify_state(&expected)?;
```

### 🔄 Refreshing Tokens

If the callback result contains a refresh token and `expires_in`, a bounded
process-local coordinator can detect expiration and serialize concurrent
refresh calls:

```rust,no_run
use rullst_connect::{AutoRefreshingSession, ConnectError, ConnectUser};
use secrecy::ExposeSecret as _;

async fn authorized_call(
    github: &rullst_connect::providers::GithubProvider,
    user: &ConnectUser,
    token_received_at: u64,
) -> Result<(), ConnectError> {
    let session = AutoRefreshingSession::from_user_at(
        github,
        user,
        token_received_at,
    )?;
    let lease = session.access_token().await?;
    send_token_to_the_authorized_api(lease.access_token().expose_secret()).await?;
    Ok(())
}
```

The coordinator uses a 60-second early-refresh window, prevents overlapping
provider calls, lets waiters reuse a successful refresh, retains a provider that
does not rotate its refresh token, adopts a validated rotation and binds every
response to the original provider user. `state_snapshot` exists for an
application-owned encrypted credential store. Cross-process leases,
retry/backoff, local-session logout/reconciliation, reauthentication and
revocation for providers outside the named adapter set remain application policy;
providers without refresh support fail explicitly.

### 🔒 Manual PKCE Support

Provider adapters expose PKCE (Proof Key for Code Exchange) where supported by the provider protocol. Some providers such as **X (Twitter) v2** require it; applications must preserve and validate the verifier/state for the complete authorization transaction.

```rust
use rullst_connect::pkce::generate_pkce;

// 1. Generate challenge and verifier
let (code_verifier, code_challenge) = generate_pkce();

// 2. Save `code_verifier` in the user's session or a secure HttpOnly cookie!

// 3. Get the URL with PKCE natively using the builder pattern
let auth_url = provider.with_pkce(&code_challenge).redirect_url();

// 4. In the callback route, fetch the user using the saved verifier:
let params = rullst_connect::provider::ExchangeParams {
    auth_code: &code,
    code_verifier: Some(&code_verifier),
    ..Default::default()
};
let user = provider.get_user(params).await?;
```

### Custom OIDC discovery

```rust
let provider = OidcProvider::discover(
    "https://identity.example.com",
    "client-id",
    "client-secret",
    "https://app.example.com/auth/callback",
).await?;
```

Discovery requires the returned issuer to match the requested issuer. Discovered token,
authorization, userinfo, and JWKS endpoints must use HTTPS. HTTP is accepted only when
both the issuer and endpoint use the same exact loopback origin. JWKS entries are refreshed
after their TTL and immediately when a token presents an unknown `kid`; stale keys are
used after a refresh error only within a bounded age and only when the requested `kid`
already exists in the cached set.

## 🧑‍💻 Full Example with Axum

You can find a complete working server using the **Axum** framework in the examples directory. Just run:

```bash
cargo run --example axum_server
```

## 📦 Releasing a New Version

`rullst-connect` is released only as part of the synchronized Rullst monorepo
train. Do not run `cargo release` or publish this crate manually. The protected,
tag-only `release.yml` workflow verifies and publishes every crate in
topological order after an exact `vX.Y.Z[-pre]` tag is approved. Follow the
[root release guide](https://github.com/Rullst/Rullst/blob/main/RELEASE_GUIDE.md)
and machine-readable `.github/release-order.json`.

## 🤝 Contributing

Feel free to open Issues and submit Pull Requests! Want to add a new provider? It's easy! Just implement the `Provider` trait.

## 📄 License

This project is licensed under the [MIT License](https://github.com/Rullst/Rullst/blob/main/LICENSE).
