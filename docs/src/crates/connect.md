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
- 🔐 **OIDC Security**: Strict discovery validation plus isolated JWKS caches with TTL, refresh on unknown `kid`, and bounded stale-if-error behavior.
- 📺 **Device Flow**: Native RFC 8628 support for headless CLI and Smart TV auth.
- 🛠️ **Testing**: Empty or `mock_*` credentials select a deterministic
  offline transport, and `mock_idp` supplies a local protocol fixture.

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

### 2. Redirect the User
Get the authorization URL and redirect your user:

```rust
let url = github.redirect_url();
// Example in Axum: return Redirect::temporary(&url);
```

### 3. Handle the Callback & Get User
When the user returns to your callback URL with a `code` query parameter, exchange it for a `ConnectUser`:

```rust
let params = rullst_connect::provider::ExchangeParams {
    auth_code: code,
    ..Default::default()
};
match github.get_user(params).await {
    Ok(user) => {
        println!("Welcome, {}!", user.name);
        println!("Email: {:?}", user.email);
        println!("Avatar: {:?}", user.avatar_url);
    }
    Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get user".to_string()),
}
```

### 🛡️ CSRF Protection (State Parameter)

To bind the authorization request to its callback, generate a high-entropy
state, store it in a short-lived server-side session, and consume it after the
callback comparison.

```rust
use rullst_connect::pkce::generate_oauth_state;

// 1. Generate and store this exact value in the server-side session.
let state = generate_oauth_state();

// 2. Get the authorization URL with the state parameter using the builder
let url = github.with_state(&state).redirect_url();
// return Redirect::temporary(&url);

// 3. With the optional Axum/Actix extractor, validate before token exchange:
callback.verify_state(&state_from_session)?;
session.consume_oauth_state()?;
```

### 🔄 Refreshing Tokens

If the provider issued a refresh token and still accepts it, the provider
adapter can request refreshed credentials. Rotation, revocation, secure storage,
and reauthentication policy remain application concerns:

```rust
let refreshed_user = github.refresh_token("existing_refresh_token_string").await?;
// Tokens are wrapped in `secrecy::SecretString` to prevent accidental log leakage ([REDACTED]).
// When you need to send it to an API, expose it explicitly:
use secrecy::ExposeSecret;
let raw_token = refreshed_user.access_token.expose_secret();
send_token_to_the_authorized_api(raw_token).await?;
```

### 🔒 PKCE Support (v9.0.0+)

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
Follow the [v12 release guide](https://github.com/Rullst/Rullst/blob/dev/RELEASE_GUIDE.md)
and require all candidate-SHA gates before creating a release tag.

## 🤝 Contributing

Feel free to open Issues and submit Pull Requests! Want to add a new provider? It's easy! Just implement the `Provider` trait.

## 📄 License

This project is licensed under the [MIT License](https://github.com/Rullst/Rullst/blob/main/LICENSE).
