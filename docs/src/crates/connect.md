# Rullst Connect 🦀

> **Vision preserved:** message brokers, additional queue transports, remote
> storage, and media work are retained with explicit status and recommendations in
> the [capability ledger](../capability-ledger.md#connect-real-time-queues-storage-and-data).

[![Crates.io](https://img.shields.io/crates/v/rullst-connect.svg?style=for-the-badge&logo=rust)](https://crates.io/crates/rullst-connect)
[![Downloads](https://img.shields.io/crates/d/rullst-connect.svg?style=for-the-badge)](https://crates.io/crates/rullst-connect)
[![Documentation](https://img.shields.io/docsrs/rullst-connect?style=for-the-badge&logo=docs.rs)](https://docs.rs/rullst-connect)
[![Build](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=for-the-badge&logo=github)](https://github.com/Rullst/Rullst/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/rullst-connect?style=for-the-badge)](https://opensource.org/licenses/MIT)

**Rullst Connect** is an async OAuth2/OIDC client layer with a shared
`Provider` interface and normalized `ConnectUser` output. Provider-specific
protocols, scopes, claims, and application account-linking rules still require
explicit review.

## 🛡️ Security Engineering

Rullst Connect uses layered tests and repository security checks. CI badges report the
state of those checks for the referenced commit; they are not an absolute security guarantee.

| Security Audit | Status | Description |
| :--- | :---: | :--- |
| **OSSF Scorecard** | [![Scorecard](https://img.shields.io/ossf-scorecard/github.com/Rullst/Rullst?style=flat-square&label=%20)](https://securityscorecards.dev/viewer/?uri=github.com/Rullst/Rullst) | Supply-chain security & best practices |
| **Codecov** | [![Coverage](https://img.shields.io/codecov/c/github/Rullst/Rullst?style=flat-square&label=%20)](https://codecov.io/gh/Rullst/Rullst) | Strict code coverage enforcement |
| **OpenSSF** | [![OpenSSF Best Practices](https://img.shields.io/badge/%20-passing-success.svg?style=flat-square)](https://www.bestpractices.dev/projects/13360) | Open source security standards |
| **Release Provenance** | [![Release](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/release.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/release.yml) | Provenance attestations for release artifacts; no SLSA level is claimed here |
| **Continuous Fuzzing** | [![Fuzz Testing](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/fuzzing.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/fuzzing.yml) | Fuzzing against edge cases & panics |
| **Property Testing** | [![Proptest](https://img.shields.io/badge/%20-passing-success.svg?style=flat-square)](https://crates.io/crates/proptest) | Validating complex logic against edge cases |
| **Miri UB Detection** | [![Miri](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/miri.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/miri.yml) | Detecting Undefined Behavior and memory leaks |
| **Kani Verifier** | [![Kani](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/kani.yml?style=flat-square&label=%20)](https://github.com/Rullst/Rullst/actions/workflows/kani.yml) | Automated reasoning and formal verification |
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

- 🚀 **Async HTTP**: Built on Tokio-compatible request paths and `reqwest`.
- 🧩 **Standardized**: All providers return a unified `ConnectUser` struct.
- 🛡️ **Type-Safe**: Robust error handling using `thiserror` (`ConnectError`).
- 🔌 **Framework adapters**: Core provider APIs are framework-independent;
  optional extractor features cover the integrations declared in the crate.
- 🔐 **Managed callback transaction**: The optional Axum/tower-sessions path
  generates, stores, expires, validates, and consumes state + PKCE + OIDC nonce.
- 🔐 **OIDC Security**: Strict discovery validation plus isolated JWKS caches with TTL, refresh on unknown `kid`, and bounded stale-if-error behavior.
- 🏢 **Explicit Corporate Proxy**: First-class HTTP(S) proxy clients, including bounded Basic proxy authentication without credentials in the endpoint URL.
- 📺 **Device Flow**: Native RFC 8628 support for headless CLI and Smart TV auth.
- 🛠️ **Testing**: Empty or `mock_*` credentials select a deterministic
  offline transport, while `mock_idp` supplies a loopback-only signed OIDC
  fixture with one-shot codes, PKCE, nonce, EdDSA ID tokens and JWKS.

> 📚 **Important Documents:**
> - [CHANGELOG.md](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md): See what's new.
> - [ISSUES](https://github.com/Rullst/Rullst/issues): Any issue? Please report.
> - [AUDIT.md](https://github.com/Rullst/Rullst/blob/main/AUDIT.md): Complete security, performance, and maintainability audit report.

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

## 🛠️ Installation

Add the published package to an application with `cargo add`. Inside a checkout
of the unreleased v12 workspace, examples use its path dependency instead.

You can either run:
```bash
cargo add rullst-connect
cargo add secrecy
```

For the recommended Axum session transaction, enable `axum-session` and add a
`tower-sessions` store:

```toml
rullst-connect = { path = "../Rullst/rullst-connect", features = ["axum-session"] }
tower-sessions = "0.15"
```

Or manually add it to your `Cargo.toml`:
```toml
[dependencies]
rullst-connect = { path = "../Rullst/rullst-connect" }
secrecy = "0.10"
tokio = { version = "1.52", features = ["full"] }
```

## 🚀 Quick Start

### 1. Initialize the Provider
Choose your provider and pass your credentials and callback URL:

```rust
use rullst_connect::prelude::*;

let github = GithubProvider::try_new(
    "YOUR_CLIENT_ID",
    "YOUR_CLIENT_SECRET".to_string().into(),
    "http://localhost:3000/auth/github/callback",
)?;
```

### Signed local OIDC fixture

Enable `axum` and mount `mock_idp::mock_router_with_config` only on an exact
loopback listener. `MockIdpConfig::try_new` also rejects non-loopback issuer and
callback URLs. The resulting fixture exercises discovery, an exact registered
client/callback, expiring one-shot authorization codes, optional nonce, S256
PKCE, EdDSA ID-token validation through JWKS and bearer-protected userinfo.

Its signing seed and credentials are deterministic public test material. It is
not a production identity provider, interactive login/consent UI, refresh-token
service, federation implementation or OIDC conformance suite. Follow the
[local OIDC testing tutorial](../tutorials/48-local-oidc-testing.md) for a
complete loopback setup.

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
rotation. See [Server-Bound OAuth/OIDC Sessions](../tutorials/42-server-bound-oauth-sessions.md).

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

If the provider returned `expires_in` plus a refresh token, bind the result to a
statically dispatched, process-local coordinator when the callback receives it:

```rust,no_run
use rullst_connect::{AutoRefreshingSession, ConnectError, ConnectUser};
use secrecy::ExposeSecret as _;

async fn call_provider_api(
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

The default checks 60 seconds before expiration. Refresh calls cannot overlap;
callers waiting behind a successful refresh reuse its state. A response can
replace the refresh token only after the lifetime and original provider user ID
validate. Use `access_token_at` in
deterministic workers/tests and `state_snapshot` only to update a dedicated
encrypted credential store. Persistence, cross-process refresh leases,
retry/backoff, revocation and reauthentication remain application policy. A
provider that does not support refresh continues to return a typed error.

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

## 🧑‍💻 Full Example with Axum

You can find a complete working server using the **Axum** framework in the examples directory. Just run:

```bash
cargo run --example axum_server
```

## 📦 Releasing a New Version

Connect is released only through the repository-wide, topologically ordered
release workflow. Do not publish this crate independently from a working tree.
Follow the [v12 release guide](https://github.com/Rullst/Rullst/blob/main/RELEASE_GUIDE.md)
and require all candidate-SHA gates before creating a release tag.

## 🤝 Contributing

Feel free to open Issues and submit Pull Requests! Want to add a new provider? It's easy! Just implement the `Provider` trait.

## 📄 License

This project is licensed under the [MIT License](https://github.com/Rullst/Rullst/blob/main/LICENSE).
