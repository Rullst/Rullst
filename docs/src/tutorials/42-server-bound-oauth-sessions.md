# 42. Server-Bound OAuth/OIDC Sessions

Rullst Connect manages the browser-to-provider callback challenge: it
generates state and PKCE for OAuth 2.0, adds a nonce for OpenID Connect, keeps
those private values on the server and consumes them exactly once before
validating the callback. It offers two places to keep them:

- a **login challenge store** (v13, from 13.0.0-alpha.3) in the application's
  own database, with only an opaque id in a short-lived cookie. A Rullst
  application signs users in with its own encrypted session cookie, so this is
  the path for it, and `cargo rullst make:oauth google|github|microsoft|apple`
  generates it;
- a **`tower-sessions` session** (`axum-session` feature), for an Axum
  application that already runs a `SessionManagerLayer`.

Either way this removes security-sensitive plumbing from ordinary handlers. It
does not choose the application's account-linking or authorization policy.

## Store-backed login

### Generate it

In a project with the `cargo rullst auth`, SaaS or LMS account model:

```bash
cargo rullst make:oauth google      # or github, microsoft, apple
cargo rullst db:migrate
```

The command adds `/auth/<provider>` and `/auth/<provider>/callback` (behind
the password login's per-client rate limit), a "Sign in with …" link on the
login page, an `oauth_identities` table (provider and subject, unique) and the
`oauth`, `oauth-<database>` and application `oauth-mock` features. The
generated callback signs the account in with `make_login_cookie_with_epoch`,
like the password login. A provider email links or creates an account only
when the provider verified it; an email that already belongs to an account is
refused by default, because registration does not prove that the person who
registered it owns the address. See
[`cargo rullst make:oauth`](../cli_reference.md#cargo-rullst-makeoauth-v13-prerelease-preview)
for the settings and the linking policy.

Run the command once per provider: the first one creates the shared flow
(`src/controllers/oauth/mod.rs`), the table and its migration; each further
provider adds only its module, routes, login link and settings, so Google and
GitHub (or all four) share one `oauth_identities` table, one migration and the
same linking policy. A provider identity signs in to one account; linking a
second provider to an existing account follows `EXISTING_ACCOUNT_POLICY`.

What each provider's identity is:

| Provider | Protocol | Subject in `oauth_identities` | Email that may link or create an account |
| :--- | :--- | :--- | :--- |
| Google | OpenID Connect | ID token `sub` | ID token `email` with `email_verified: true` |
| GitHub | OAuth 2.0 (not OIDC) | numeric account `id` of `GET /user`, never the login name, which users can rename | the `primary` address of `GET /user/emails`, only when GitHub marks it `verified` |
| Microsoft | OpenID Connect, tenant chosen by `MICROSOFT_TENANT` | `<tid>:<oid>`, the directory and object IDs (`sub` is pairwise: it changes with the app registration) | the `email` claim only with the optional claim `xms_edov: true` |
| Apple | OpenID Connect, callback by form POST | ID token `sub`, stable for the developer team | ID token `email` with `email_verified`; it may be a private relay address |

**Microsoft.** `MICROSOFT_TENANT` is required with the credentials and has no
default, because it decides who can create accounts: `common` (any Entra ID
organization and personal Microsoft accounts), `organizations` (any
organization), `consumers` (personal accounts) or one directory (tenant) ID.
Rullst Connect verifies the ID token against the tenant's keys and requires an
issuer `https://login.microsoftonline.com/<tid>/v2.0` whose `tid` the tenant
choice admits. Microsoft does not verify the `email` claim: an organization's
administrators can give a user any address, including one of another
company. Treating it as verified would let any tenant administrator sign in to
an account here (the "nOAuth" pattern). The generated code therefore trusts it
only with `xms_edov: true`, Microsoft's statement that the address's domain is
verified for the user's organization (or that the account is a personal one).
Add the optional ID-token claims `email` and `xms_edov` under **Token
configuration** in the app registration; without them a Microsoft identity
cannot create or link an account, and only identities linked before can sign
in. Changing `EXISTING_ACCOUNT_POLICY` does not change this check.

**Apple.** Apple returns the callback as a cross-site form POST
(`response_mode=form_post`). Browsers send no `SameSite=Lax` cookie with it, so
the generated start uses `ChallengeCookie::for_cross_site_post`
(`SameSite=None; Secure`), and the command lists `/auth/apple/callback` in
`csrf_signed_webhook_paths` of `Rullst.toml`: Apple cannot send this site's
CSRF token, so instead the handler accepts the POST only with the `state` of
the login challenge that this browser's challenge cookie names, which another
site cannot know. The client secret is a five-minute ES256 JWT that Rullst
Connect signs for each code exchange from `APPLE_TEAM_ID`, `APPLE_KEY_ID` and
the `.p8` key in `APPLE_PRIVATE_KEY` (process environment or a secret store;
newlines may be written as `\n`; the command never writes it to a file).
`AppleProvider::verify_signing_key` stops the application at startup when the
key cannot sign. Apple sends the user's name only on the first authorization,
in the unsigned `user` form field; the generated code uses it as the new
account's display name and for nothing else. An Apple email may be a private
relay address (`…@privaterelay.appleid.com`): register the domain of
`MAIL_FROM` with Apple's Private Email Relay service, or account emails never
arrive. Apple accepts only HTTPS return URLs on a registered domain, never
`localhost`, so develop with the mock credentials.

To try a provider without registering an application, run with mock
credentials and the offline provider, which sends the browser straight back
to the callback (a same-site GET, Apple included) and signs in
`mock@example.invalid`:

```bash
GOOGLE_CLIENT_ID=mock_google GOOGLE_CLIENT_SECRET=mock_google \
  cargo run --features oauth-mock
```

The command prints the mock settings of the provider it added, such as
`MICROSOFT_TENANT=common MICROSOFT_CLIENT_ID=mock_microsoft
MICROSOFT_CLIENT_SECRET=mock_microsoft`. Without `--features oauth-mock` the
application stops at startup with an error that names the feature, and
staging and production refuse mock credentials.

### Or write the two handlers

The store implements `ChallengeStore`: `MemoryChallengeStore` for tests and one
process, or `SqlChallengeStore` over the application's own SQLx pool with the
`sqlite`, `postgres` or `mysql` feature of `rullst-connect` (`oauth-sqlite`,
`oauth-postgres` or `oauth-mysql` on the `rullst` facade, where the module is
`rullst::connect::challenge`). `SqlChallengeStore::open` creates its table,
`rullst_connect_login_challenges`, when it is missing.

```rust
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use rullst_connect::challenge::{
    ChallengeCookie, LoginOptions, MemoryChallengeStore, begin_login, finish_login,
};
use rullst_connect::extractors::AuthCallback;
use rullst_connect::providers::GoogleProvider;

/// A redirect that also sets `cookies`.
fn redirect(target: &str, cookies: &[String]) -> Response {
    let mut response = Redirect::to(target).into_response();
    for cookie in cookies {
        if let Ok(value) = HeaderValue::from_str(cookie) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
    }
    response
}

/// `GET /auth/google`
async fn start(
    store: &MemoryChallengeStore,
    google: &GoogleProvider,
    cookie: &ChallengeCookie,
) -> Response {
    match begin_login(store, google, LoginOptions::oidc("google")).await {
        Ok(login) => redirect(login.url(), &[login.set_cookie(cookie)]),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

/// `GET /auth/google/callback`
async fn callback(
    store: &MemoryChallengeStore,
    google: &GoogleProvider,
    cookie: &ChallengeCookie,
    headers: &HeaderMap,
    params: &AuthCallback,
) -> Response {
    let id = ChallengeCookie::read(
        headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok()),
    );
    match finish_login(store, google, "google", id.as_deref(), params).await {
        Ok(login) if login.user().email_verified == Some(true) => {
            // Map `login.user().id` (the provider subject) to an account and
            // add the application's session cookie here.
            redirect("/dashboard", &[cookie.clear()])
        }
        Ok(_) | Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}
```

`ChallengeCookie::new("/auth/google", secure)` scopes the cookie to the login
routes; pass `secure` the way Rullst Core decides it,
`Environment::requires_secure_defaults()`. The cookie is named
`rullst_oauth_challenge`, so Core also adds `Secure` to it on every HTTPS
request.

### Store contract

- `begin_login` stores the provider name, state, PKCE verifier, nonce (OIDC
  only) and an optional same-site return page (`ReturnPath`: an absolute path,
  never `//host` or `/\host`), and returns a fresh `ChallengeId`: 256 random
  bits, 43 base64url characters. The challenge lives ten minutes, or less
  with `LoginOptions::with_lifetime`.
- The browser holds only the id, in an `HttpOnly`, `SameSite=Lax` cookie of at
  most ten minutes. `SameSite=Lax` still sends it with the provider's
  top-level `GET` redirect. For a provider that POSTs the callback from its own
  site (Sign in with Apple), `ChallengeCookie::for_cross_site_post` makes it
  `SameSite=None; Secure`, the only cookie a browser attaches to that POST.
  Several cookies of that name with different values are refused rather than
  guessed.
- Stores key rows by a SHA-256 digest of the id, never the id. Consumption is
  one atomic removal: `DELETE … RETURNING` on SQLite and PostgreSQL,
  `SELECT … FOR UPDATE` then `DELETE` in one transaction on MySQL, a mutex in
  memory. Of several concurrent callbacks with one id, exactly one finds it.
- `finish_login` reports a missing cookie, then an unknown, altered, used or
  expired id, before it reads the callback; from there the challenge is
  consumed whatever follows. It compares the state in constant time before it
  trusts a provider `error`, then exchanges the code with the stored verifier
  and nonce. Errors are typed (`LoginError`, `ChallengeError`) and carry no
  state, code, verifier, nonce or id.
- Each store has an entry limit: a save at the limit first deletes expired
  challenges, then refuses with `ChallengeError::CapacityExceeded`
  (`purge_expired` deletes them on demand). The limit is exact on SQLite and in
  memory; on PostgreSQL and MySQL concurrent saves can pass it by at most
  their number.

Concurrent login tabs each have their own challenge, unlike the one active
challenge of a `tower-sessions` session.

## Alternative: `tower-sessions`

The rest of this tutorial covers the `axum-session` feature.

## Enable the session feature

Pin the v13 prerelease exactly (Cargo never selects a prerelease on its own;
the v12 line reached end of life, see the [release record](../v12.md)):

```toml
[dependencies]
rullst-connect = { version = "=13.0.0-alpha.2", features = ["axum-session"] }
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

The v13 prerelease (13.0.0-alpha.1) adds `AuthSessionForm`, the form POST
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
tabs. Such an application should use the [store-backed login](#store-backed-login),
which keys each attempt by its own opaque id with the same expiry, atomic
consumption, constant-time comparison and redaction properties.

The generic `tower-sessions` store interface does not expose a distributed
compare-and-delete. Two requests that already loaded the same record can still
race even though each removal is saved immediately. Provider authorization
codes are themselves single-use, but account creation/linking and authenticated
session establishment must still be idempotent. Deployments requiring a strict
distributed callback claim should use the store-backed login, whose SQL stores
consume a challenge atomically.

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
state, expiry, replacement, replay, typed exchange parameters and redaction,
and, for the store-backed login, concurrent single consumption on SQLite (and
on owned PostgreSQL and MySQL servers in CI). They are not provider
certification or deployment evidence.
