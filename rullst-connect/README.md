# Rullst Connect 🦀

[![Crates.io](https://img.shields.io/crates/v/rullst-connect.svg?style=for-the-badge&logo=rust)](https://crates.io/crates/rullst-connect)
[![Downloads](https://img.shields.io/crates/d/rullst-connect.svg?style=for-the-badge)](https://crates.io/crates/rullst-connect)
[![Documentation](https://img.shields.io/docsrs/rullst-connect?style=for-the-badge&logo=docs.rs)](https://docs.rs/rullst-connect)
[![Build](https://img.shields.io/github/actions/workflow/status/Rullst/Rullst/ci.yml?style=for-the-badge&logo=github)](https://github.com/Rullst/Rullst/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/rullst-connect?style=for-the-badge)](https://opensource.org/licenses/MIT)
</div>

**Rullst Connect** is an elegant, async-first, and Developer Experience (DX) focused OAuth2 authentication library for Rust. It simplifies the integration of social logins into your Rust web applications, providing a standardized interface across multiple providers.

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

- 🚀 **Async & Fast**: Built on top of `tokio` and `reqwest`.
- 🧩 **Standardized**: All providers return a unified `ConnectUser` struct.
- 🛡️ **Type-Safe**: Robust error handling using `thiserror` (`ConnectError`).
- 🔌 **Framework Agnostic**: Works seamlessly with Rullst, Axum, Actix, Leptos, Dioxus, or any other framework.
- 🔐 **OIDC Security**: Strict discovery validation plus isolated JWKS caches with TTL, refresh on unknown `kid`, and bounded stale-if-error behavior.
- 📺 **Device Flow**: Native RFC 8628 support for headless CLI and Smart TV auth.
- 🛠️ **Testing**: Typed, network-free provider fallbacks plus an explicitly mounted embedded Mock IdP for local tests.

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

Add the package to your `Cargo.toml`. If you use **Rullst**, **Axum**, **Actix**, or **Leptos**, you can enable their specific features for native Extractor support!

You can either run:
```bash
cargo add rullst-connect
```

Or manually add it to your `Cargo.toml`:
```toml
[dependencies]
rullst-connect = "12.0.0"
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
rullst-connect = { version = "12.0.0", features = ["mock"] }
```

Mock-mode redirects use the reserved `example.invalid` domain and mock profiles use the
reserved `example.invalid` email domain.

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

To prevent Cross-Site Request Forgery (CSRF) attacks, you should generate a secure random string, save it in a session/cookie, and pass it to the provider.

```rust
// 1. Generate a random state string and save it in the session
let state = "random_secure_string";

// 2. Get the authorization URL with the state parameter using the builder
let url = github.with_state(state).redirect_url();
// return Redirect::temporary(&url);

// 3. In the callback route, verify if the query param `state` matches your session!
// If you are using the optional `axum` or `actix` features, you can use `verify_state`:
// params.verify_state(&state_from_session)?;
```

### 🔄 Refreshing Tokens

If an access token expires, you can seamlessly renew it without asking the user to login again by using their `refresh_token`:

```rust
let refreshed_user = github.refresh_token("existing_refresh_token_string").await?;
// Tokens are wrapped in `secrecy::SecretString` to prevent accidental log leakage ([REDACTED]).
// When you need to send it to an API, expose it explicitly:
use secrecy::ExposeSecret;
let raw_token = refreshed_user.access_token.expose_secret();
println!("Successfully refreshed token securely!");
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

This project uses `cargo-release` to automate version bumps, README synchronization, and CHANGELOG management.
The publish workflow in `.github/workflows/publish.yml` runs when a `vX.Y.Z` tag is pushed, and it can also be triggered manually from GitHub Actions.

To release a new version, simply run:

```bash
# install it first if you haven't: cargo install cargo-release
cargo release patch --execute  # for v1.0.x patches
cargo release minor --execute  # for v1.x.0 features
cargo release major --execute  # for vX.0.0 breaking changes
```

This will automatically bump versions, tag the release, and push to GitHub, triggering the crates.io publish workflow.

For the exact release checklist and what to do next time, see [RELEASE_GUIDE.md](https://github.com/Rullst/Rullst/blob/main/RELEASE_GUIDE.md).

## 🤝 Contributing

Feel free to open Issues and submit Pull Requests! Want to add a new provider? It's easy! Just implement the `Provider` trait.

## 📄 License

This project is licensed under the [MIT License](https://github.com/Rullst/Rullst/blob/main/LICENSE).
