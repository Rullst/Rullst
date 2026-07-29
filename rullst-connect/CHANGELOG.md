# Changelog

> [!NOTE]
> Starting from Rullst version 12.0.0, all changelogs will be written exclusively in the main `CHANGELOG.md` at the root of the Rullst repository. This file is kept for historical purposes only.
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Maintenance
- **Dependabot Bumps**: Updated GitHub Actions workflows (`github/codeql-action` to `v4.37.3`, `ossf/scorecard-action` to `v2.4.4`, `actions/configure-pages` to `v6.0.0`, `actions/upload-pages-artifact` to `v5.0.0`, and `EmbarkStudios/cargo-deny-action` to `v2.1.1`). Also bumped `base64` crate from `0.22` to `0.23`.

## [11.0.0] - 2026-07-15

### Breaking Changes
- **HTTPS Enforcement**: All `redirect_url` and `issuer_url` declarations now strictly require `https://`. An exception has been added for development environments running on `http://localhost` and `http://127.0.0.1`. Applications using plain `http://` in production or tests will fail during provider initialization.

### Performance
- **JWKS Cache Optimization**: Improved `JWKS_CACHE` to store `Arc<JwkSet>` instead of the full JSON structure. This eliminates deep cloning during cache hits and reduces memory allocations under high concurrent OIDC authentication load.
- **Client Allocations**: Reduced heap allocations (`String`) during `HttpRequest` construction by leveraging `Cow<'static, str>` for HTTP methods, and optimized error allocations in the fallback path.

### Security
- **Timing Attack Mitigation**: Secured `verify_nonce` and `verify_state` against potential length-based timing attacks by hashing inputs using `SHA256` before applying constant-time comparisons.
- **Strict JSON Serialization Errors**: Replaced `unwrap_or_default()` in HTTP client with strict error handling (`map_err`) to avoid accidentally sending empty bodies on payload serialization failures.

### Fixed
- **Mock Provider Panics**: Removed `unimplemented!()` panics in `SimpleProvider` and `FailingUserProvider` mock structs, replacing them with explicit `ConnectError` returns to improve stability in testing environments.

### Added
- **Test Coverage**: Added test coverage for `HttpClientExt` helper methods (`.get()` and `.post()`), `basic_auth` requests, fallback HTTP mapping, JWKS caching flow, and `with_http_client`/`with_retry` builder methods.
- **Mock IDP Coverage**: Added tests for valid authorization and token exchange paths in the end-to-end `mock_idp` utilities.

## [10.0.5] - 2026-07-05

### Security
- **CI/CD Credential Protection**: Added a new TruffleHog GitHub Actions workflow to block leaked secrets and API keys in Pull Requests before they are merged.
- **JWT Algorithm Confusion Mitigation**: Hardened the generic `OidcProvider`, `AppleProvider`, and `GoogleProvider`'s `id_token` validation to strictly enforce asymmetric cryptographic algorithms (e.g., `RS256`, `ES256`). This patches a critical vulnerability where an attacker could bypass signature checks by providing a symmetric algorithm (like `HS256`) or `none` in the token header.
- **SLSA Level 3 Build Provenance**: Added official GitHub Actions workflow steps to generate verifiable SLSA Level 3 build provenance attestations for crates published to crates.io. Added the SLSA Level 3 badge to the README.

### Added
- **Extractors Edge Case Testing**: Added explicit test case ensuring `AuthCallback::verify_state` returns `ConnectError::InvalidState` when the `state` parameter is completely missing.
- **Secret Deserialization Testing**: Added test coverage for `Option<SecretString>` custom serde deserialization.
- **Prelude Testing**: Added compilation tests for the library's `prelude` module to ensure all re-exports remain accessible.
- **Mock Implementations**: Filled out `get_user` and `get_user_from_token` in `DummyProvider` with valid mock data to prevent accidental panics in consumer tests.

### Maintenance
- **Dependabot Bumps**: Updated multiple GitHub Actions workflows to their latest stable versions (`actions/checkout@v7.0.0`, `codecov/codecov-action@v7.0.0`, `actions/cache@v6.1.0`, and `github/codeql-action@v4.36.3`).

## [10.0.4] - 2026-06-28

### Fixed
- **Mutation Testing Resilience**: Eliminated all 19 surviving `cargo-mutants` mutants by hardening the test suite:
  - **`src/client.rs`**: Added precise boundary tests for the 2 MiB response body size limit (`test_body_size_limit_exceeded`, `test_body_size_limit_exact_boundary_succeeds`), verifying both that over-limit bodies are rejected and that exact-limit bodies are accepted. Added `test_retry_branch_headers_are_forwarded` to kill the `delete !` mutant on the `!req.headers.is_empty()` guard in the retry branch. Strengthened `test_reqwest_client_new_with_retry_is_distinct_from_default` to use heap address comparison instead of a trivial existence check.
  - **`src/extractors.rs`**: Added explicit assertions to `test_verify_state` covering prefix-length mismatches (`== vs !=` mutant), same-length / different-content mismatches (`&& vs ||` mutant), and the `Ok(())` replacement mutant. Added `test_axum_extractor_values_are_from_query` and expanded `test_actix_extractor` to assert that real query-string values are returned, killing the `Ok(Default::default())` mutant on both framework extractors. Added `test_axum_session_extractor_carries_real_values` and `test_axum_session_extractor_length_mismatch_rejected` to cover the `AuthSession` replacement and logical-operator mutants.
  - **`src/providers/auth0.rs`** and **`src/providers/google.rs`**: Strengthened `test_auth0_with_retry` and `test_google_with_retry` to additionally assert that the new `http_client` is not pointer-equal to the global `DEFAULT_HTTP_CLIENT`, killing the `replace with_retry -> Self with Default::default()` mutant that previously escaped the weaker `ptr_eq` check.
- **`cargo fmt` Compliance**: Fixed formatting regressions introduced by the previous release, ensuring `cargo fmt -- --check` passes on CI.

## [10.0.3] - 2026-06-28

### Added
- **Property-based Testing**: Added `proptest` to development dependencies and introduced property-based tests for PKCE challenge generation.
- **Provider Tests**: Added missing test `test_exchange_and_get_user_fetch_user_fails` to properly cover partial token exchange failures where network succeeds but user-fetching fails.

### Changed
- **Performance Optimization**: Replaced instance-level `OnceCell` JWKS caches in `AppleProvider`, `GoogleProvider`, and `OidcProvider` with a global `LazyLock<RwLock<HashMap>>`. This eliminates redundant public key fetches on every request and prevents rate-limiting issues when instantiating providers dynamically in web frameworks like Axum.

## [10.0.2] - 2026-06-24

### Added
- **Quality & Security Pipeline**:
  - Replaced `cargo-audit` with `cargo-deny` for comprehensive vulnerability, license, and ban checks.
  - Added `cargo-llvm-cov` for accurate, LLVM-based code coverage.
  - Integrated `cargo-mutants` for mutation testing to ensure robust test assertions.
  - Configured GitHub CodeQL for automated taint analysis and static application security testing (SAST).
  - Configured `Miri` in CI to automatically test for Undefined Behavior and memory safety.
  - Configured `cargo-fuzz` CI infrastructure with a manual trigger and a strict 6-hour timeout to prevent runner exhaustion.
  - Added `cargo-semver-checks` workflow to enforce SemVer compliance in PRs and prevent accidental breaking changes.
  - Added `cargo-machete` workflow to enforce removal of unused dependencies.
  - Added `cargo-spellcheck` workflow to catch spelling and grammar errors in documentation.
  - Added `cargo-kani` workflow to automate mathematical proofs via Kani Rust Verifier.
  - Configured GitHub Dependabot for automated weekly updates on crates and GitHub Actions.
  - Added `criterion`, `loom`, and Kani verifier as development dependencies for microbenchmarking, concurrent async testing, and formal verification.
  - Refactored `provider_bench` to use the `Criterion` framework.
- **Test Coverage (94%)**:
  - Achieved ~94% line coverage globally.
  - Added exhaustive unit tests for network failure paths (HTTP 400 Bad Request, invalid JSON) and missing fields (missing `id_token`, `access_token`) across providers.
  - Added tests for `Provider` trait default methods (`request_device_code`, `refresh_token`, `revoke_token`, `poll_device_token`).
- **OIDC Provider**: Implemented the missing `refresh_token` trait method.

## [10.0.1] - 2026-06-19

### Changed
- **Dependencies**: Bumped `secrecy` requirement to `0.10` and `actions/checkout` to `v7` in GitHub Actions workflows.

## [10.0.0] - 2026-06-18

### [BREAKING CHANGE]
- **`build_oauth_params` Signature Change**: The signature of the public helper function `rullst_connect::provider::build_oauth_params` has been updated to accept `base_url` as `&str` and `scopes` as `&str` (previously `String` and `&[String]`). This eliminates cascading heap allocations during URL generation. Developers implementing custom providers using this helper must update their calls to pass string references. The primary user-facing builder API (`with_scopes`) is completely unaffected.

- **`HttpRequest` Form Type**: The `form` field in `rullst_connect::client::HttpRequest` has been changed from `Vec<(String, String)>` to `Option<String>`. This breaking change allows providers to serialize form data directly into a single pre-allocated string (using structures like `TokenExchangeForm`), completely eliminating dynamic vector and string allocations during token exchanges. Custom HTTP client implementations must be updated to accept the pre-serialized string.

- **Token Type Safety (`secrecy` Crate)**: All provider constructors now require `secrecy::SecretString` for the `client_secret` parameter, instead of `String`. Furthermore, the `access_token` and `refresh_token` fields inside the `ConnectUser` universal profile struct have been changed to `SecretString`. Developers interacting with these tokens must now call `.expose_secret()` or `.expose_secret().as_str()` to read the raw string values. This is an architectural boundary introduced to prevent accidental credential logging.

### Performance
- **Zero-Allocation JSON Parsing**: The HTTP client now parses API responses directly from the byte buffer using `serde_json::from_slice`, completely bypassing the redundant UTF-8 validation pass and the intermediate `String` allocation that `String::from_utf8` previously caused.
- **Zero-Allocation Token Exchanges**: Introduced `TokenExchangeForm` across all OAuth providers. This static struct uses `serde` to efficiently serialize form data directly into the HTTP request body without intermediate `Vec` or `String` allocations on the hot path.
- **Atomic Session Extraction**: Optimized the Axum `AuthCallback` extractor to use `session.remove()` directly instead of sequentially calling `session.get()` followed by `session.remove()`. This cuts backend I/O operations (e.g., Redis or database calls) in half during the OAuth callback phase.
- **Global Connection Pooling**: Centralized HTTP client instantiation by introducing a globally shared `reqwest::Client` (via `LazyLock`) in `src/client.rs`. This prevents each provider from creating its own isolated connection pool, dramatically reducing memory overhead, DNS lookups, and thread spawning in multi-provider environments.
- **URL Serialization**: Optimized `redirect_url` string allocations across all providers by preventing unnecessary `format!()` concatenations and leveraging `url::form_urlencoded::Serializer::for_suffix` to safely append queries directly to the base URL without duplicate memory allocations.
- **Pre-joined Scopes**: Provider scopes are now pre-joined into a single `String` at provider initialization time rather than being dynamically allocated and joined as a `Vec<String>` on every authentication request, drastically reducing allocations in the hot path.
- **OIDC ID Token Fast-Path**: Optimized `OidcProvider` token exchange to properly utilize `get_user_from_form` (which parses and cryptographically validates the returned `id_token`) instead of blindly falling back to the generic token exchange. This saves a full network roundtrip to the `/userinfo` endpoint for OIDC providers that embed claims directly in the `id_token`.
- **Lazy JWKS Fetching (`OidcProvider`)**: Optimized `OidcProvider::discover` to no longer eagerly fetch the JSON Web Key Set (JWKS) during provider initialization. It now stores the `jwks_uri` and leverages `tokio::sync::OnceCell` to fetch the public keys lazily upon the first actual token validation. This eliminates a blocking external network call from application startup, significantly decreasing initial boot and discovery latency.
- **Request Body Allocation**: Removed an unnecessary `String` clone during internal HTTP requests by fully consuming the `HttpRequest` struct when assigning request bodies, slightly improving runtime performance during token exchanges.
- **URL Capacity Pre-allocation**: Optimized OAuth2 URL extension methods (`redirect_url_with_state`, `redirect_url_with_pkce`, etc.) to use `.reserve()` and pre-allocate string capacity. This prevents dynamic `String` re-allocations on the heap when appending query parameters to the base URL returned by providers.
- **PKCE Verifier Allocation**: Optimized PKCE `code_verifier` random string generation by pre-allocating an exact 64-byte `String::with_capacity` and utilizing `.extend()`. This guarantees a single allocation, removing hidden `Vec` re-allocation steps previously caused by `.collect()` on unknown-length random iterators.
- **HeaderMap Memory Optimization**: Refactored internal `HttpRequest` to utilize `reqwest::header::HeaderMap` rather than `Vec<(String, String)>`. This shifts the `builder.header()` pattern to accept `&str` instead of `impl Into<String>`, eliminating unnecessary `String` heap allocations and avoiding duplicated strings for static headers like `Accept` and `User-Agent`.
- **Header Parsing Fast-Path**: Replaced `.to_str()` and `.parse()` with direct ASCII byte manipulation (`.as_bytes().iter().try_fold(...)`) when parsing the `Content-Length` header. This completely eliminates UTF-8 validation overhead and `String` allocations for this common numeric header, squeezing out extra CPU cycles during the HTTP response parsing hot-path.
- **Compile-Time Scope Concatenation**: Replaced runtime `["a", "b"].join(" ")` array allocations with the `concat!` macro in the `define_provider!` macro. This shifts the concatenation of default OAuth2 scopes entirely to compile-time, converting them into a single static string literal and eliminating dynamic heap allocations and iteration overhead every time a provider is initialized.

### Security
- **JWT Algorithm Confusion Bypass**: Patched a critical security vulnerability in `GoogleProvider`, `AppleProvider`, and `OidcProvider` where the `id_token` signature verification blindly trusted the `alg` specified in the unverified JWT header. The validation logic now strictly enforces asymmetric cryptographic algorithms (e.g. `RS256`), completely mitigating attacks where a malicious actor injects symmetric algorithms (like `HS256`) to bypass signature checks using known public keys.
- **OOM Protection (Body Capacity Cap)**: Added a hard cap (`min(MAX_BODY_SIZE)`) to the pre-allocated `Vec::with_capacity` buffer derived from the `Content-Length` header. This prevents an attacker from spoofing a massive `Content-Length` (e.g. 10GB) and crashing the application with an Out-of-Memory (OOM) panic before the body stream is even read.
- **Provider API Error Sanitization**: Implemented truncation (maximum 512 characters) for error messages parsed from provider APIs in `HttpClient`. This prevents potential sensitive information exposure and prevents log spam / DoS if an upstream provider or gateway returns massive payloads (like a 100KB HTML Cloudflare error page) inside an HTTP failure.
- **OIDC Nonce Timing Attacks**: Hardened OpenID Connect (OIDC), Google, and Apple providers against timing side-channel attacks during ID token validation. The `nonce` claim is now verified using constant-time comparison via `subtle::ConstantTimeEq` instead of standard string equality. Added strict enforcement requiring the `nonce` claim to be present in the JWT when expected.
- **Constant-Time Panic (DoS) Mitigations**: Fixed multiple Denial of Service vulnerabilities where attackers could crash the application by sending tokens or `state` strings of a different length than expected. The `subtle::ConstantTimeEq` trait panics on slices of unequal length. A strict pre-validation length check was added before executing constant-time equality in `AuthCallback::verify_state`, the Axum `AuthSession` extractor, and the OIDC `nonce` validation logic for Google, Apple, and generic OIDC providers.
- **Log Leakage & Token Exposure Prevention**: Deeply integrated the `secrecy` crate into the core library architecture. Highly sensitive fields like `client_secret`, `access_token`, and `refresh_token` are no longer raw `String` types; they are now strongly typed as `secrecy::SecretString`. This guarantees that tokens cannot be accidentally leaked via `dbg!()`, `println!()`, standard application logs, or panic traces (they display as `[REDACTED]`). It also enforces safe memory-zeroing when the variables are dropped.
- **Short-Lived Apple Client Secrets (Defense in Depth)**: Hardened `AppleProvider`'s dynamic `client_secret` JWT generator. The expiration (`exp`) claim for the internally generated authentication token was drastically reduced from 30 days (2,592,000 seconds) to just 5 minutes (300 seconds). Since a new JWT is generated strictly at the moment of each token exchange, this minimizes the attack surface to a negligible window if a token is ever intercepted.
- **SSRF & Open Redirect Mitigation (`OidcProvider`)**: Hardened the OIDC discovery process by explicitly enforcing URL scheme validation (`http`/`https`) on both the `issuer_url` and `redirect_url` provided by the developer at runtime. This mitigates Server-Side Request Forgery (SSRF) and Local File Inclusion vulnerabilities where an attacker could theoretically provide an internal metadata IP (`http://169.254.169.254`) or local file path (`file:///`) dynamically.

## [9.0.1] - 2026-06-18

### Security
- **Hardcoded Credentials**: Removed hardcoded dummy credentials from Axum examples (`axum_example.rs` and `axum_server.rs`), replacing them with environment variable reads (`std::env::var`) to promote secure configuration practices.

### Performance & Maintenance
- **Form Allocation Optimization**: Replaced array slicing `.form(&[...])` with standard arrays `.form([...])` across all providers to reduce unnecessary borrowing overhead.
- **Header Construction**: Optimized `HeaderMap` construction in `ReqwestClient` to safely use `try_from` without panicking on invalid headers.
- **Redundant Clones**: Removed unnecessary `.clone()` calls on the access token in the `GoogleProvider` token exchange.
- **Macro Hygiene**: Fixed macro hygiene issue by properly scoping `#[allow(dead_code)]` in the `define_provider` macro to avoid triggering `unused_attributes` warnings in recent Rust compilers.

### Added
- **Unit Tests**: Added robust tests for `ConnectUser` serialization and deserialization.
- **Unit Tests**: Added tests for the `Connect::driver` dynamic provider factory method.
- **Unit Tests**: Added endpoints tests for the `mock_router` discovery and userinfo handlers in `mock_idp.rs`.

## [9.0.0] - 2026-06-17

### Changed (Breaking)
- **Provider API Unification**: Replaced the redundant `get_user(auth_code: &str)` and `get_user_with_pkce(...)` methods on the `Provider` trait with a single, unified `get_user(params: ExchangeParams)` method. This future-proofs the API and handles `auth_code`, `code_verifier`, and `expected_nonce` in one struct.

### Security
- **OIDC Nonce Validation**: Implemented cryptographic validation for the `nonce` claim in `GoogleProvider`, `AppleProvider`, and `OidcProvider`. The `ExchangeParams` struct now accepts an optional `expected_nonce` to prevent replay attacks during OpenID Connect flows.
- **PKCE Enforcement**: Fixed an issue where the `code_verifier` was correctly accepted by `get_user_with_pkce` but incorrectly omitted from the final token exchange POST request across multiple providers (Cognito, LinkedIn, GitHub, OIDC, Apple, Discord, Microsoft, Facebook).

### Added
- **Unit Tests**: Added comprehensive mock client testing for the `GithubProvider` to verify mapping of GitHub profile attributes.
- **Unit Tests**: Added initialization tests for `ReqwestClient::new_with_retry` under the `retry` feature flag.
- **Unit Tests**: Added missing coverage for the generic `fetch_access_token` and `fetch_refresh_token` helper methods in `src/provider.rs`.

### Performance
- **Optimized JSON Parsing**: Refactored the `GoogleProvider` token exchange to deserialize directly into a strongly-typed struct (`GoogleTokenResponse`) instead of the generic `serde_json::Value`, avoiding unnecessary allocations.

## [8.0.0] - 2026-06-15

### Changed (Breaking)
- **Pruning**: Removed 25 low-usage/unmaintained providers to focus the library completely on the 11 most robust, heavily-used core providers: Google, GitHub, Microsoft, Apple, Auth0, Cognito, Facebook, X, Discord, LinkedIn, and OIDC.
- **Security (PKCE)**: Upgraded the `Provider` trait to uniformly require PKCE natively on all providers. Implemented `get_user_with_pkce` enforcing authorization code interception attack mitigations.
- **Architecture**: Renamed the primary entry point `Socialite` to `Connect`.
- **API Optimization**: Changed `RequestBuilder::form` to accept `IntoIterator` to avoid unnecessary memory allocations. If you were passing slices via `.form(&[...])`, you must now pass arrays or vectors directly `.form([...])`.

### Changed
- **Security**: Replaced `debug_assert!` with `assert!` in Google provider to ensure configuration verification runs in release builds.
- **Security**: Upgraded CSRF `state` parameter validation in `AuthCallback` and `AuthSession` to use constant-time comparison via the `subtle` crate, mitigating timing attacks.
- **Internal**: Removed unused `dead_code` macro allowance in `macros.rs` test module.
- **Testing**: Added tests covering `ResponseWrapper::json` deserialization error paths.
- **Testing**: Added missing tests for default trait methods in `HttpClientExt` (`get` and `post`).

### Added
- **Architecture**: Renamed `Socialite` to `Connect` and implemented `Connect::driver`, a dynamic provider factory method allowing initialization of standard OAuth providers by name via string matching.
- **DRY Refactoring**: Introduced `impl_standard_redirect_url!` and `impl_standard_refresh_token!` helper macros, eliminating hundreds of lines of duplicate boilerplate across 30+ providers without introducing breaking changes to the `Provider` trait interface.
- **Testing**: Added unit tests for `fetch_refresh_token` error flows and the `exchange_and_get_user` integration helper.

### Added
- **Architecture**: Implemented `Socialite::driver`, a dynamic provider factory method allowing initialization of standard OAuth providers by name via string matching.
- **DRY Refactoring**: Introduced `impl_standard_redirect_url!` and `impl_standard_refresh_token!` helper macros, eliminating hundreds of lines of duplicate boilerplate across 30+ providers without introducing breaking changes to the `Provider` trait interface.
- **Testing**: Added unit tests for `fetch_refresh_token` error flows and the `exchange_and_get_user` integration helper.

## [7.0.2] - 2026-06-08

### Security
- **Strict CSRF Enforcement**: Axum Session extractor now strictly requires the `state` parameter in callback queries, returning a `BAD_REQUEST` error if missing, eliminating the bypass vector (FINDING-01).
- **Secure Convenience Methods**: URL-encoded the `state` and `code_challenge` parameters in default trait helper methods of `Provider` to prevent parameter injection (FINDING-02).
- **Safety Assertions**: Promoted macros credential verification guards from `debug_assert!` to standard `assert!`, ensuring validation runs in production/release builds (FINDING-03).
- **Apple Token Retrieval Hardening**: Required `access_token` presence in Apple provider token exchange rather than falling back to an empty string (FINDING-04).
- **GitHub Device Flow Validation**: Enforced presence of `device_code`, `user_code`, and `verification_uri` in GitHub device authorization flow response (FINDING-05).
- **Mock IDP Warning Documentation**: Added warnings to mock identity provider discovery and token handlers explicitly marking `alg: none` as unsafe for production (FINDING-06).

## [7.0.1] - 2026-06-06

### Security & Performance
- **Header Optimization**: Migrated 30+ providers from manual `format!()` string concatenation and Base64 encoding to native `.bearer_auth()` and `.basic_auth()` methods for robust `Authorization` header construction.
- **Async Test Hardening**: Replaced `std::sync::Mutex` with `tokio::sync::Mutex` in test mocks to prevent async lock contention. Replaced unsafe `.unwrap()` with descriptive `.expect()` and idiomatic error handling in tests and docs.

### Security
- **Token Exposure Prevention**: Migrated VK (VKontakte), Facebook, and Instagram providers to use `Authorization: Bearer <token>` HTTP headers instead of exposing `access_token` in URL query parameters, preventing leakage in server logs and proxy caches.
- **Client Secret Protection**: Migrated VK (VKontakte) token exchange from `GET` to `POST` request with a form body, protecting `client_secret` from URL exposure.
- **OIDC Signature Validation Enforcement**: Removed silent fallback to `/userinfo` in Google and generic OIDC providers when `id_token` verification/signature fails, ensuring immediate failure for invalid tokens.
- **Apple Key ID Hardening**: Refactored Apple id_token verification to prevent defaulting Key ID (`kid`) to an empty string, mitigating potential Key Confusion Attacks.
- **DoS/OOM Prevention (HttpClient)**: Imposed a 2MB maximum size limit when reading HTTP response bodies chunk-by-chunk in the default client to protect against Slowloris-style buffer attacks. Clamped `max_retries` to a maximum of 10.

### Resilience & Error Handling
- **Strict Error Handling (Phantom Users Mitigation)**: Extended the elimination of silent failures across all remaining 21 providers (including GitHub, GitLab, Apple, Google, Facebook, etc.). Crucial user identifier fields (like `id` or `sub`) no longer default to empty strings `""` or `0`, returning an explicit `ConnectError::Provider` error if essential user data is missing.
- **Graceful JWT Fallback**: Refactored `OidcProvider`'s `id_token` Key ID (`kid`) extraction to use `.and_then()` instead of `.unwrap_or_default()`, ensuring that missing `kid` headers cleanly fall back to the `/userinfo` endpoint instead of searching for an empty key.

### Added
- **Unit Tests**: Added integration/unit tests for `mock_idp.rs` to guarantee the local mock router properly simulates OAuth2 failure flows (e.g., `invalid_grant` for invalid authorization codes).
- **Unit Tests**: Added comprehensive tests in `src/error.rs` validating `.into()` conversion from standard library and third-party errors (`reqwest::Error`, `serde_json::Error`) into `ConnectError`.

## [7.0.0] - 2026-06-01

### Security & Architecture
- **Dependency Shielding (Blindagem de Dependências)**: Fully isolated the public API from external dependency leaks to protect downstream users from unexpected breaking changes:
  - Refactored `ConnectError` to hold stringified errors (`String`) instead of raw third-party types (`reqwest::Error`, `jsonwebtoken::errors::Error`, `base64::DecodeError`, etc.).
  - Implemented manual `From` conversions for internal convenience of the `?` operator without exposing raw types to consumers.
  - Restricted the visibility of the `jwks` field in `OidcProvider` from `pub` to `pub(crate)` to avoid leaking `jsonwebtoken::jwk::JwkSet` to consumers.

### Security
- **Cryptographic JWT Validation — Apple Provider**: `AppleProvider` now lazily fetches Apple's public JWKS from `https://appleid.apple.com/auth/keys` (cached via `tokio::sync::OnceCell`) and cryptographically verifies the `id_token` signature and claims on every login. Removes the previous unverified base64-decode approach.
- **Cryptographic JWT Validation — Google Provider**: `GoogleProvider` now lazily fetches Google's JWKS from `https://www.googleapis.com/oauth2/v3/certs` and validates the `id_token` signature. If validation fails (e.g. network error, unknown key), it falls back gracefully to the secure `/userinfo` API endpoint.

### Performance
- **Zero-Allocation Option Mapping**: Replaced `.as_str().unwrap_or("").to_string()` with `.as_str().map(String::from).unwrap_or_default()` across all 34 providers. This eliminates unnecessary heap allocations when a field is absent.
- **Smart Scope Serialization**: Optimized `build_oauth_params` in `src/provider.rs` to skip the `join(" ")` heap allocation entirely when only one scope is requested.
- **No-Clone Secret Generation**: Refactored `AppleClaims` to use lifetime references `AppleClaims<'a>` instead of owned `String`s, removing unnecessary clones during Apple client secret generation.

### Added
- **Mockable `CognitoProvider`**: Refactored to use `Arc<dyn HttpClient>` and exposed `with_http_client` builder method, enabling custom/mock HTTP clients for testing without a real Cognito server.
- **Mockable `OktaProvider`**: Same refactoring as above. Now fully testable offline with any custom client.
- **Mockable `AppleProvider`**: Refactored from `reqwest::Client` to `Arc<dyn HttpClient>` and exposed `with_http_client`, enabling mock-based unit testing.
- **Unit Tests — `RequestBuilder` & `ResponseWrapper`**: Comprehensive tests added to `src/client.rs` covering all builder methods and validating `error_for_status` behavior for standard OAuth error payloads, `message` fields, unknown JSON shapes, and plain-text bodies.
- **Unit Tests — `CognitoProvider` & `OktaProvider`**: Redirect URL unit tests added, verifying correct domain and parameter construction.
- **Unit Tests — `AppleProvider`**: Redirect URL and invalid token handling unit tests added.
- **Unit Tests — `GoogleProvider`**: Redirect URL unit test added.
- **Unit Tests — `OidcProvider::discover` edge cases**: Verifies correct `.well-known` URL construction with and without trailing slashes, and validates descriptive errors on missing OIDC configuration fields.

### Changed
- `ResponseWrapper` now derives `Debug` to support standard Rust test assertion patterns.
- `GoogleProvider` is now defined explicitly instead of via the `define_provider!` macro to support the additional `jwks` `OnceCell` field.



### Changed
- **Dependency Updates**: 
  - `leptos_router` `0.7` → `0.8`

--- 

## [6.1.3] - 2026-05-31

### Changed
- **Dependency Updates**: Updated all direct dependencies to use flexible versioning for automatic patch/minor updates:
  - `async-trait` `0.1.89` → `0.1`
  - `base64` `0.22.1` → `0.22`
  - `jsonwebtoken` `10.4.0` → `10`
  - `rand` `0.10.1` → `0.10`
  - `reqwest` `0.13.4` → `0.13` (maintained for compatibility with reqwest-middleware)
  - `serde` `1.0.228` → `1`
  - `serde_json` `1.0.150` → `1`
  - `sha2` `0.11.0` → `0.11`
  - `thiserror` `2.0.18` → `2`
  - `tokio` `1.52.3` → `1`
  - `url` `2.5.8` → `2.5`
  - `axum` `0.8.9` → `0.8`
  - `actix-web` `4.13.0` → `4`
  - `leptos_router` `0.6` → `0.7`
  - `serde_urlencoded` `0.7.1` → `0.7`
  - `reqwest-middleware` `0.5.2` → `0.5`
  - `reqwest-retry` `0.9.1` → `0.9`
  - `tower-sessions` `0.15.0` → `0.15`
  - `wiremock` `0.6.5` → `0.6` (dev)

## [6.1.2] - 2026-05-31

### Changed
- **Dependency Updates**: All direct dependencies updated to their latest stable versions:
  - `rand` `0.8` → `0.10.1`
  - `reqwest` `0.13.3` → `0.13.4`
  - `actix-web` `4.9.0` → `4.13.0`
  - `tower-sessions` `0.13.0` → `0.15.0`
  - `wiremock` `0.6.2` → `0.6.5` (dev)

### Internal
- **rand 0.10 Migration** (`src/pkce.rs`): Updated PKCE code to the new `rand 0.10` API — replaced `distributions::Alphanumeric` with `distr::Alphanumeric`, `thread_rng()` with `rng()`, and imported `RngExt` trait for `sample_iter` support.

## [6.1.1] - 2026-05-30

### Fixed
- **Formatting Cleanup**: Reflowed several Rust modules to satisfy `cargo fmt -- --check` and keep CI green.
- **Publish Workflow**: Corrected the crates.io tag filter and added manual workflow dispatch for releases.
- **Release Documentation**: Added a dedicated release guide and aligned the README with the publish flow.

## [6.1.0] - 2026-05-30

### Added
- **OIDC Fast-Path Provider (`OidcProvider`):** Added a generic provider that automatically downloads the OpenID configuration via `.well-known/openid-configuration` and sets up endpoints instantly.
- **Enterprise-Grade Observability:** Native integration with `tracing` crate. Highly detailed spans during token exchanges and profile fetching.
- **Strict Profile Normalization:** Enforced strict typing for `UniversalProfile` (`ConnectUser`), ensuring fields like `email_verified` exist consistently across all 35+ providers.
- **Automated CSRF Protection (`AuthSession`):** Added an Axum Session extractor (behind `axum-session` feature flag) with `tower-sessions` to automatically generate, store, and validate OAuth `state` securely.
- **Native Apple Secret Generation:** `AppleProvider` now dynamically generates the JWT `client_secret` on the fly using a `.p8` Private Key, eliminating tedious script generation for developers.
- **Local Mock IdP:** Added a built-in mock identity provider router (behind `mock-idp` feature flag) powered by Axum, enabling full E2E testing without internet access or rate limits.
- **Device Authorization Flow (RFC 8628):** Support for headless CLI tools and Smart TVs with new `request_device_code()` and `poll_device_token()` methods, fully implemented for GitHub.
- **Cryptographic OIDC Signature Validation:** `OidcProvider` now automatically fetches the provider's JWKS Public Keys and cryptographically verifies the RSA signature of the `id_token` for maximum enterprise security.
- **Unified Provider Errors**: Replaced panic/opaque errors with `ProviderApiError`. HTTP 400s now gracefully parse OAuth 2.0 standard JSON errors (`error`, `error_description`).
- **ORM Integration**: Added the `IntoDatabaseUser` helper trait to easily transform the universal profile into your database models.

### Changed
- **Avatar Standardization**: Improved avatar resolution for Discord (upscaled to 1024px), Google (upscaled to 400px), and X/Twitter (stripped `_normal` suffix for original quality).

### Fixed
- **Mock IdP Build:** Added the missing `base64::Engine` import in `src/mock_idp.rs` so the mock identity provider builds cleanly with the current `base64` API.
- **GitHub Integration Test:** Updated the token error assertion to match the actual `ConnectError::ProviderApiError` shape returned by the client.
- **Clippy Cleanup:** Moved the `src/extractors.rs` test module to the end of the file to satisfy `clippy::items_after_test_module`.
- **Twitch Provider Safety:** Removed the remaining production `unwrap()` in `src/providers/twitch.rs` and now return a proper error when the user payload is empty.
- **Rustdoc Hygiene:** Fixed the bare URL warning in `src/providers/cognito.rs` by formatting it as a proper rustdoc link.

## [5.2.3] - 2026-05-29

### Fixed
- **Formatting**: Fixed trailing blank lines left over by `cargo fix` to appease the strict `-D warnings` on `cargo fmt`.

## [5.2.2] - 2026-05-29

### Fixed
- **Clean Code**: Removed unused `url::form_urlencoded` imports left over from the v5.2.0 refactor, fixing `-D warnings` on the `clippy` CI check.

## [5.2.1] - 2026-05-29

### Fixed
- **CI/CD**: Fixed GitHub Actions permission issue for `rustsec/audit-check` that caused workflow failures.
- **Formatting**: Fixed `cargo fmt` errors on `src/lib.rs`, `src/macros.rs`, and integration tests.

## [5.2.0] - 2026-05-29

### Added
- **Leptos Support**: Integrated Leptos support! By enabling the `leptos` feature, the `AuthCallback` extractor now seamlessly implements `leptos_router::Params`.
- **HTTP Client Agnostic**: The provider traits and builder methods now allow passing a custom `HttpClient` through `.with_http_client(...)`.
- **HTTP Proxy Support**: With the agnostic HTTP client interface, users can now provide a proxy-configured client to navigate locked-down environments easily.

### Refactored
- **URL Generation Boilerplate**: Cleaned up the codebase by removing massive code duplication across all 35 providers for URL generation (`client_id`, `redirect_uri`, `scope`, `state`, `pkce` logic is now unified).

## [5.1.0] - 2026-05-28

### Added
- **Automatic CSRF Validation**: The `AuthCallback` extractor now includes a `verify_state(&self, session_state: &str)` method to automatically secure OAuth flows against CSRF attacks.
- **Refresh Token Support**: Added `token_url()` and `refresh_token(token: &str)` methods to the `Provider` trait and implemented them across all 35 providers, allowing developers to automatically renew expired tokens natively.

### Fixed
- **Testing Dependencies**: Added `serde_urlencoded` to `dev-dependencies` to fix a major compilation bug running the test suite on `main`.

### Security
- Maintained zero `unsafe` footprint while ensuring standard parameter passing for URL token generation and token revocation across providers.

## [5.0.2] - 2026-05-27

### Performance
- **Optimized String allocations**: Replaced `String::new()` with `String::with_capacity(256)` in all 33 providers' `redirect_url` methods, reducing unnecessary reallocations and improving performance.

### Developer Experience
- **Fixed README example**: Corrected compilation bug in the README.md code example (line 103) where the `Err` branch was incorrectly placed inside the `Ok` block.

### Maintenance
- **Removed dead code**: Deleted `src/utils.rs` file which was no longer used after the PR-29 refactoring that inlined URL parameter serialization logic directly into providers.

### Internal
- **Code cleanup**: Removed unused module imports and references to the deleted `utils.rs` module from `lib.rs`.

### Compatibility
- **Breaking Changes**: None
- **API Changes**: None
- **Migration Guide**: No migration required - fully backward compatible with v5.0.1

## [5.0.1] - 2026-05-27

### Added
- **Tokens returned on User**: `ConnectUser` now contains `access_token`, `refresh_token`, and `expires_in` fields so you can interact with the provider's API immediately.
- **Frontend/Mobile Integrations**: Added `get_user_from_token(access_token)` to all providers. This allows your backend to securely fetch the user profile when the OAuth flow is handled natively on the frontend (e.g. mobile apps, React, Vue).
- **Framework Integrations (Axum & Actix)**: Added `axum` and `actix-web` optional features in `Cargo.toml`. Provides native extractors (`AuthCallback`) for seamless URL parsing in route handlers.
- **Token Revocation**: New `revoke_token` method on the `Provider` trait for direct logout at the provider level (reference implementation added for Google).
- **Mocking Tools (TDD)**: Added `MockProvider` to the library to facilitate offline unit testing.
- **Continuous Integration**: Added GitHub Actions (Publish to Crates.io) to automate new version deployments via Tags.
- **Native OIDC Support**: OIDC providers (like Google and Apple) now feature a "Fast Path" that decodes the `id_token` directly via base64, extracting name, email, and photo instantly without making a secondary HTTP request! A massive performance boost.
- **PKCE Support (Proof Key for Code Exchange)**: All providers now have native support for modern PKCE security via the `.with_pkce(code_challenge)` builder method.
- **Prelude Module (`rullst_connect::prelude::*`)**: Added a prelude module for unified imports (ideal for developers and AI assistants).

### Changed
- **Architectural Macros**: All providers now use the internal `define_provider!` macro, which centralizes constructors, state, PKCE, scopes, and reduces hundreds of lines of boilerplate.
- All dependencies have been updated to their latest compatible versions.
- Cleaned up compiler warnings related to unused variables across providers.

## [0.4.0] - Previous stable version
- Initial open-source release with 33 OAuth2 providers supported.
- Standardized `ConnectUser` and async support.
