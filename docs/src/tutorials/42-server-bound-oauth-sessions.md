# 42. Server-Bound OAuth/OIDC Sessions

Rullst Connect can manage the browser-to-provider callback challenge for an
Axum application. The bounded flow generates state and PKCE for OAuth 2.0,
adds nonce for OpenID Connect, stores the private values in `tower-sessions`,
and consumes them before validating the callback.

This removes security-sensitive plumbing from ordinary handlers. It does not
configure the application's session store, cookie, TLS, account-linking, or
authorization policy.

## Enable the session feature

For the unreleased workspace:

```toml
[dependencies]
rullst-connect = { path = "../Rullst/rullst-connect", features = ["axum-session"] }
tower-sessions = "0.15"
```

Published applications should replace the path with one immutable compatible
version. Add a `SessionManagerLayer` to the Axum router. `MemoryStore` is useful
for local examples and tests, but it is process-local and is not a production
durability or horizontal-scaling strategy.

```rust,ignore
use tower_sessions::{cookie::SameSite, MemoryStore, SessionManagerLayer};

let sessions = SessionManagerLayer::new(MemoryStore::default())
    .with_http_only(true)
    .with_same_site(SameSite::Lax)
    .with_secure(true);

let app = app.layer(sessions);
```

`SameSite::Lax` permits the ordinary top-level OAuth callback while reducing
cross-site cookie exposure. Production still requires HTTPS, a durable shared
store where multiple instances are used, bounded store retention, protected
keys and an explicit reverse-proxy policy.

## Start OAuth 2.0 with state and PKCE

Use this path for providers such as GitHub where the application is using an
OAuth authorization-code flow without an ID token:

```rust
use axum::response::Redirect;
use rullst_connect::prelude::*;
use tower_sessions::Session;

async fn start_github(
    session: Session,
    github: &GithubProvider,
) -> Result<Redirect, ConnectError> {
    let authorization = begin_oauth_session(&session, github).await?;
    Ok(Redirect::temporary(authorization.url()))
}
```

The returned URL contains the random state and the SHA-256 PKCE challenge. The
64-character verifier is serialized only in the server-side session record.
`OAuthAuthorization` deliberately redacts its URL from `Debug` output.

## Start OpenID Connect with nonce

Use the OIDC variant for Google, Apple, or a discovered custom OIDC provider:

```rust,ignore
let authorization = begin_oidc_session(&session, &oidc_provider).await?;
Ok(Redirect::temporary(authorization.url()))
```

This stores another random value and sends it as `nonce`. The provider adapter
receives that same expected nonce later and validates it against the signed ID
token in the adapters whose documented contract includes ID-token validation.

## Consume the callback

Mount `AuthSession` directly as an Axum extractor. Extraction parses the real
query, removes and immediately saves the stored challenge, rejects expiry or a
constant-time state mismatch, and makes a later sequential replay fail:

```rust
use rullst_connect::prelude::*;
use tower_sessions::Session;

async fn github_callback(
    session: Session,
    callback: AuthSession,
    github: &GithubProvider,
) -> Result<UniversalProfile, ConnectError> {
    let user = github.get_user(callback.exchange_params()?).await?;

    // Rotate the browser session before establishing authenticated identity.
    session
        .cycle_id()
        .await
        .map_err(|error| ConnectError::Session(error.to_string()))?;
    session
        .insert("authenticated_user_id", &user.id)
        .await
        .map_err(|error| ConnectError::Session(error.to_string()))?;

    Ok(user.universal_profile())
}
```

Do not serialize `ConnectUser` as a credential store. Its public serialization
already omits provider tokens, while `UniversalProfile` is the narrower
credential-free identity projection. If an application needs provider refresh
tokens, place them in a dedicated encrypted store with explicit rotation and
revocation policy.

For a provider that returned both a refresh token and `expires_in`, construct a
bounded process-local coordinator at the trusted time the token response was
received:

```rust,no_run
use rullst_connect::{AutoRefreshingSession, ConnectError, ConnectUser};
use secrecy::ExposeSecret as _;

async fn provider_request(
    github: &rullst_connect::providers::GithubProvider,
    user: &ConnectUser,
    token_received_at: u64,
) -> Result<(), ConnectError> {
    let tokens = AutoRefreshingSession::from_user_at(
        github,
        user,
        token_received_at,
    )?;
    let lease = tokens.access_token().await?;
    call_authorized_endpoint(lease.access_token().expose_secret()).await?;
    Ok(())
}
```

`AutoRefreshingSession<P>` checks a bounded early-expiration window and holds
one async process-local refresh gate, so provider refresh calls cannot overlap
and waiters reuse the first valid result. It keeps the old refresh credential if
the provider does not rotate, adopts a valid rotation, requires the same provider
user ID and changes state only after full validation. Persist `state_snapshot()` through a dedicated
encrypted credential store. Multi-process deployments still need an
application-owned distributed lease; retry/backoff, revocation, reauthentication
and replay of the original API request are deliberately not inferred.

## Lifecycle and failure semantics

The managed contract is intentionally small:

- a challenge expires ten minutes after it is created;
- there is one active challenge per browser session;
- starting a second flow replaces the first, so the older browser tab fails;
- authorization URLs must use HTTPS or exact loopback HTTP, contain no URL
  credentials/fragment, and preserve exactly one generated state and S256 PKCE
  tuple without a preconfigured nonce;
- the challenge is removed and saved before state, nonce or PKCE-dependent
  exchange;
- missing, mismatched, expired and later sequential callbacks fail closed;
- provider error text is bounded before it becomes a typed error;
- callback codes, state, nonce, verifier and authorization URLs are redacted
  from the managed types' `Debug` output.

One active challenge makes replay and lifecycle behavior unambiguous, but it is
not the best UX for applications that intentionally support concurrent login
tabs. Such an application should build a bounded transaction store keyed by an
opaque flow identifier and retain the same expiry, atomic consume, constant-time
comparison and redaction properties.

The generic `tower-sessions` store interface does not expose a distributed
compare-and-delete. Two requests that already loaded the same record can still
race even though each removal is saved immediately. Provider authorization
codes are themselves single-use, but account creation/linking and authenticated
session establishment must still be idempotent. Deployments requiring a strict
distributed callback claim should use an application-owned atomic challenge
store.

## What the application still must prove

Before release, test the exact deployed provider and browser path:

1. The registered redirect URI exactly matches the application route and uses
   HTTPS outside an exact loopback development host.
2. The session cookie remains Secure and HttpOnly, uses an intentional
   SameSite policy, and is rotated after successful authentication.
3. Every application instance sees the same durable session store, or routing
   is deliberately constrained without pretending failover works.
4. Issuer, audience, signature, expiry and nonce checks pass and fail against
   the provider's real or restricted environment.
5. Account creation/linking cannot attach an attacker-controlled provider
   identity to an existing local account.
6. Denial, timeout, provider outage and abandoned-login recovery have bounded
   user-visible behavior without logging credentials.

The local Rullst regressions prove generation, round-trip, mismatch, missing
state, expiry, replacement, replay, typed exchange parameters and redaction.
They are not provider certification or deployment evidence.
