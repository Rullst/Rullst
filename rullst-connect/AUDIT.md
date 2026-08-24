# rullst-connect security review guide

This file is an inventory and reproduction guide for the current `rullst-connect` source. It is
not a certification, penetration-test report, vulnerability-free statement, or evidence that a
particular release is suitable for every production environment.

The former v10.0.2 audit used static `PASS` labels, 10/10 scores, fixed test counts, and a
"fully production-ready" conclusion. Those statements were point-in-time opinions and are not
valid evidence for v12 or later, so they are intentionally not carried forward here.

## Current crate boundary

`rullst-connect` provides OAuth2/social-login adapters and OpenID Connect helpers. Queue
transports and application WebSockets/SSE are outside this crate today; Kafka, RabbitMQ, and
Redis Streams adapters remain roadmap work.

The review surface includes:

- provider configuration and callback URL validation;
- OAuth state and PKCE helpers;
- OIDC discovery, issuer validation, token claims, and JWKS rotation;
- provider HTTP response parsing and size bounds;
- deterministic, network-free credential modes;
- optional Axum, Actix, Leptos, retry, session, and mock integrations.

Application session policy, authorization after login, tenant membership, TLS termination,
secret storage, IdP configuration, redirect registration, and deployment monitoring remain the
application/operator's responsibility.

## Implemented controls to inspect

| Control | Primary implementation | Important boundary |
|---|---|---|
| Fallible provider construction | `src/macros.rs`, `src/configuration.rs`, specialized providers | Prefer `try_new`; deprecated `new` constructors fail closed but cannot report configuration errors directly. |
| Offline credential mode | `src/client/offline.rs`, `src/configuration.rs` | Empty or `mock_*` credentials never use the built-in network client. Functional mock identities require tests or the explicit `mock` feature so missing production secrets do not create a working login. |
| Redirect/issuer parsing | `src/configuration.rs` | HTTPS is required except for exact loopback hosts. Lookalike names such as `localhost.evil` are rejected. |
| OIDC discovery validation | `src/providers/oidc/discovery.rs` | The discovered issuer must exactly match the requested issuer. Discovered endpoints require HTTPS, except for the issuer's exact loopback origin. |
| Rotation-aware JWKS | `src/provider/jwks.rs` | Caches have a TTL, refresh on unknown `kid`, and use bounded stale material only for a key already present when refresh fails. |
| State/nonce comparison | `src/extractors.rs`, `src/provider/mod.rs` | Constant-time comparison is one control; the application must still generate, store, expire, and consume transaction state correctly. |
| Response-size limit | `src/client/reqwest_client.rs` | The built-in client enforces its configured bound. A custom `HttpClient` is a separate trust boundary and must enforce equivalent limits. |
| Secret wrappers | provider/user types using `secrecy::SecretString` | Debug redaction reduces accidental disclosure; it is not a complete secret-management or memory-erasure guarantee for every copy and dependency. |

## Reproduce evidence for a commit

Run these commands from the workspace root and record the commit SHA, Rust toolchain, enabled
features, command output, and date with any review artifact:

```bash
cargo test -p rullst-connect --all-features
cargo clippy -p rullst-connect --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo tree -p rullst-connect -e normal
```

Dependency scanners are separate tools and may require installation and advisory-database
network access. Do not state that they passed unless their output for the same commit is
available. Temporary advisory exceptions, their scope, owner, compensating controls, and expiry
are tracked in [`../docs/src/security-advisory-exceptions.md`](../docs/src/security-advisory-exceptions.md).

Relevant regression tests currently live alongside configuration/JWKS modules and in
`tests/integration_tests.rs`. Test counts are deliberately omitted because they change as the
suite evolves.

## Review checklist

- Verify every enabled provider's registered redirect URI at the IdP as well as in application
  configuration.
- Require and consume OAuth `state`; use PKCE where supported and bind `nonce` to OIDC
  transactions.
- Confirm expected issuer, audience, accepted signing algorithms, clock policy, and JWKS
  freshness for the deployment.
- Keep the `mock` feature out of production authentication builds unless mock identities are an
  explicit, isolated requirement.
- Treat injected `HttpClient` implementations as security-sensitive code and test their TLS,
  redirect, timeout, response-size, proxy, and retry policies.
- Run dependency and source scanners without silently ignoring findings; document any necessary
  exception with a deadline.
- Perform deployment-specific integration, abuse, and penetration testing before exposing an
  authentication flow to real users.

## Historical note

The removed v10.0.2 scorecard described code paths and file layouts that have since changed
(including constructor assertions and non-expiring shared JWKS caches). Git history remains the
appropriate place to inspect that historical snapshot; it must not be reused as a current release
attestation.
