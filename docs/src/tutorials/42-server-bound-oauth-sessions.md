# 42. Server-Bound OAuth/OIDC Sessions

Rullst Connect can manage the browser-to-provider callback challenge for an
Axum application. The bounded flow generates state and PKCE for OAuth 2.0,
adds nonce for OpenID Connect, stores the private values in `tower-sessions`,
and consumes them before validating the callback.

This removes security-sensitive plumbing from ordinary handlers. It does not
configure the application's session store, cookie, TLS, account-linking, or
authorization policy.

## Enable the session feature

For `12.1.0` (see [publication status](../v12.md)):

```toml
[dependencies]
rullst-connect = { version = "12.1.0", features = ["axum-session"] }
tower-sessions = "0.15"
```

Applications should use one immutable compatible version. Add a
`SessionManagerLayer` to the Axum router. `MemoryStore` is useful
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

`SameSite::Lax` permits the ordinary top-level OAuth callback, a cross-site
`GET` redirect, while reducing cross-site cookie exposure. It does not cover a
provider that returns the callback as a cross-site `POST`; see
[Sign in with Apple](#sign-in-with-apple-form-post-callback). Production still requires HTTPS, a durable shared
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

Use the OIDC variant for Google or a discovered custom OIDC provider:

```rust,ignore
let authorization = begin_oidc_session(&session, &oidc_provider).await?;
Ok(Redirect::temporary(authorization.url()))
```

This stores another random value and sends it as `nonce`. The provider adapter
receives that same expected nonce later and validates it against the signed ID
token in the adapters whose documented contract includes ID-token validation.
The query-string `AuthSession` callback does not support Apple; see the next
section.

## Sign in with Apple: form POST callback

`AppleProvider` always requests `response_mode=form_post`, which Apple requires
when the `name` or `email` scope is requested. Apple then returns `code`,
`state`, `id_token` and, on the first sign-in only, `user` in an
`application/x-www-form-urlencoded` POST from `appleid.apple.com`. Two parts of
the query-string flow do not fit that request:

- `AuthSession` reads the callback from the query string only, so the POST has
  no state to compare and the extraction fails;
- the POST is cross-site, and browsers send a cookie on a cross-site POST only
  when it is `SameSite=None; Secure`. The `SameSite::Lax` session cookie shown
  above is not sent, so the callback cannot find its stored challenge.

Do not relax the application's authenticated session cookie to
`SameSite=None` to work around this. Keep the Apple challenge in a dedicated,
short-lived store reached through its own `SameSite=None; Secure; HttpOnly`
cookie scoped to the Apple routes. With a 12.x release, validate the posted form
with the framework-neutral primitives:

```rust,ignore
use rullst_connect::extractors::AuthCallback;
use rullst_connect::pkce::{generate_oauth_state, generate_pkce};
use rullst_connect::prelude::*;
use rullst_connect::provider::ExchangeParams;

// Start: generate the same values the managed flow would store.
let state = generate_oauth_state();
let nonce = generate_oauth_state();
let (code_verifier, code_challenge) = generate_pkce();
let mut url = url::Url::parse(
    &apple.redirect_url_with_pkce_and_state(&code_challenge, &state),
)?;
url.query_pairs_mut().append_pair("nonce", &nonce);
// Application-provided: store the three values for ten minutes under a random
// ID and set `apple_challenge=<ID>; Path=/auth/apple; Max-Age=600; Secure;
// HttpOnly; SameSite=None` on the redirect response.
store_apple_challenge(&state, &nonce, &code_verifier).await?;

// Callback: `POST /auth/apple/callback` with a small body limit.
async fn apple_callback(
    axum::Form(callback): axum::Form<AuthCallback>,
) -> Result<ConnectUser, ConnectError> {
    // Application-provided: atomically remove the challenge named by the
    // `apple_challenge` cookie, failing if it is missing or expired.
    let challenge = take_apple_challenge().await?;
    callback.verify_state(&challenge.state)?;
    if let Some(error) = &callback.error {
        return Err(ConnectError::Provider(format!("Apple returned {error}")));
    }
    let code = callback
        .code
        .as_deref()
        .ok_or_else(|| ConnectError::Token("missing code".to_string()))?;
    apple
        .get_user(ExchangeParams {
            auth_code: code,
            code_verifier: Some(&challenge.code_verifier),
            expected_nonce: Some(&challenge.nonce),
        })
        .await
}
```

`AuthCallback` ignores the extra `id_token` and `user` fields. `get_user`
redeems the code and verifies the returned ID token's signature, issuer,
audience, expiry and nonce. The unsigned `user` JSON is the only place Apple
sends the user's name; parse it separately if needed and treat it as
unverified. After a successful callback, rotate or create the application's own
authenticated session as usual.

### Managed form POST callback

The unpublished v13 development source adds `AuthSessionForm`, the form POST
counterpart of `AuthSession`. It reads an `application/x-www-form-urlencoded`
`POST` body of at most 16 KiB, ignores `id_token` and `user`, and consumes the
same challenge that `begin_oidc_session` stored, with the same expiry,
constant-time state comparison and single use. A non-`POST` request, another
content type or an oversized body is rejected before the challenge is touched.

```rust
use rullst_connect::prelude::*;

async fn apple_callback(
    callback: AuthSessionForm,
    apple: &AppleProvider,
) -> Result<ConnectUser, ConnectError> {
    apple.get_user(callback.exchange_params()?).await
}
```

The cookie requirement does not change: the session that holds the challenge
must be `SameSite=None; Secure`. Give the Apple start and callback routes their
own session layer rather than relaxing the application session:

```rust,ignore
use tower_sessions::{cookie::SameSite, MemoryStore, SessionManagerLayer};

let apple_challenges = SessionManagerLayer::new(MemoryStore::default())
    .with_name("apple_oauth_challenge")
    .with_path("/auth/apple")
    .with_http_only(true)
    .with_secure(true)
    .with_same_site(SameSite::None);

let apple_routes = Router::new()
    .route("/auth/apple/start", get(start_apple)) // begin_oidc_session
    .route("/auth/apple/callback", post(apple_callback)) // AuthSessionForm
    .layer(apple_challenges);
```

An inner session layer replaces the outer `Session` for those routes, so the
callback cannot also write the application session. Hand the verified identity
over with an application-owned one-time step, for example a random single-use
handoff ID that expires within a minute and is redeemed by a same-site route
under the application session layer.

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
use rullst_connect::prelude::ExposeSecret as _;

# async fn call_authorized_endpoint(_: &str) -> Result<(), ConnectError> {
#     Ok(())
# }

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
user ID and changes state only after full validation.

## Persist refresh state without storing plaintext tokens

Rullst supplies a storage-neutral authenticated envelope. The application still
chooses its database/file/secret manager, but the stored token record need not
invent its own cryptographic format:

```rust
use rullst_connect::{
    AutoRefreshingSession, EncryptedTokenSnapshot, Provider,
    TokenSnapshotBinding, TokenSnapshotError, TokenSnapshotKey,
};

async fn seal_current_state<P: Provider + ?Sized>(
    session: &AutoRefreshingSession<'_, P>,
    key_bytes: [u8; 32],
    local_account_id: &str,
) -> Result<EncryptedTokenSnapshot, TokenSnapshotError> {
    let state = session.state_snapshot().await;
    let binding = TokenSnapshotBinding::try_new("github", local_account_id)?;
    let key = TokenSnapshotKey::try_new("oauth-primary-2026", key_bytes)?;
    EncryptedTokenSnapshot::seal(&state, &key, &binding)
}
```

Write only `EncryptedTokenSnapshot::as_str()` to durable storage. On restart:

1. parse the stored string with `EncryptedTokenSnapshot::try_from_envelope`;
2. read its non-secret `key_id()` and select that 32-byte key in a secret
   manager;
3. rebuild the same binding from trusted provider and local-account state;
4. call `open`, then pass the restored state to `AutoRefreshingSession::new`.

The AES-256-GCM authentication tag covers the envelope version, key ID,
provider and local account, so a copied record fails for another owner. The
payload is bounded and revalidated after decryption. Debug output is redacted,
and the envelope itself does not implement `Display` or Serde. The application
must still commit each new generation transactionally, rotate/retain keys,
authorize the account and revoke local state. Multi-process deployments also
need a distributed compare-and-set/lease; retry/backoff, reauthentication and
replay of the original API request are deliberately not inferred.

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
- the form POST variant accepts only a `POST` with a form body of at most
  16 KiB and leaves the challenge untouched when it rejects the request shape;
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
   Native or mobile clients do not sign in by sending a provider access token
   to `get_user_from_token`: a userinfo response does not prove the token was
   issued to this application's `client_id`. Verify their ID token with
   `verify_id_token` (Google, `OidcProvider`) and a server-issued nonce.
6. Denial, timeout, provider outage and abandoned-login recovery have bounded
   user-visible behavior without logging credentials.

The local Rullst regressions prove generation, round-trip, mismatch, missing
state, expiry, replacement, replay, typed exchange parameters and redaction.
They are not provider certification or deployment evidence.
