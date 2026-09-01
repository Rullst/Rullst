# 48. Signed Local OIDC Testing

> [!IMPORTANT]
> Dependency examples use `12.0.0-rc.1`, the planned first v12 RC. Do not
> request it from crates.io before it is published; use path dependencies from
> this source checkout during development.

Rullst Connect includes an explicitly mounted local identity-provider fixture so
an application can exercise a cryptographically verified OIDC flow without a
third-party account. Unlike an in-process provider stub, this path traverses
HTTP discovery, authorization, token exchange, JWKS retrieval and ID-token
verification through the ordinary `OidcProvider` implementation.

## Enable and bind the fixture

Enable the Axum feature in development:

```toml
[dev-dependencies]
rullst-connect = { version = "12.0.0-rc.1", features = ["axum"] }
```

Mount the router only on an exact loopback listener:

```rust,no_run
use rullst_connect::mock_idp::{MockIdpConfig, mock_router_with_config};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let issuer = "http://127.0.0.1:8080";
let callback = "http://127.0.0.1:3000/auth/callback";
let config = MockIdpConfig::try_new(
    issuer,
    "academy-local-client",
    "academy-local-secret",
    callback,
)?;
let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
axum::serve(listener, mock_router_with_config(config)).await?;
# Ok(())
# }
```

`MockIdpConfig` rejects non-HTTP or non-loopback issuers and callbacks, issuer
paths, empty/oversized identifiers and control characters. The application
must still bind the actual listener to loopback; possession of a router does not
control how the host serves it.

## Configure the ordinary OIDC client

Use non-placeholder local credentials so `OidcProvider` traverses the HTTP
fixture instead of selecting the separate network-free credential fallback:

```rust,no_run
use rullst_connect::providers::OidcProvider;

# async fn provider() -> Result<OidcProvider, rullst_connect::ConnectError> {
OidcProvider::discover(
    "http://127.0.0.1:8080",
    "academy-local-client",
    "academy-local-secret",
    "http://127.0.0.1:3000/auth/callback",
).await
# }
```

Compose this provider with the server-bound session ceremony from
[Tutorial 42](42-server-bound-oauth-sessions.md). That ceremony generates and
stores state, PKCE verifier and OIDC nonce; the fixture binds the challenge and
nonce to its one-shot authorization grant. The callback then supplies the
consumed values to `OidcProvider::get_user`.

## What the test proves

The checked-in loopback regression proves that the current client can:

- validate discovery metadata on the exact issuer origin;
- preserve an exact registered client and callback;
- exchange one expiring authorization code only once;
- reject a missing, malformed or mismatched S256 PKCE verifier;
- bind the requested nonce into a signed ID token;
- select the Ed25519 public key by `kid` from JWKS;
- validate EdDSA signature, issuer, audience, expiry and nonce; and
- accept only an issued, unexpired bearer token at userinfo.

Authorization grants and access-token digests are process-local and capped at
64 records. A restart erases them. This is intentional test behavior, not a
durable identity service.

## Non-production boundary

The signing seed, client credentials and identity are deterministic public
fixtures. The router has no interactive login, consent, refresh-token, device,
federation, account-management, key-rotation or administrative lifecycle. It
has not passed an OIDC conformance suite. Never expose it publicly, reuse its
key or credentials, or use a successful fixture test as evidence that a live
provider/deployment is correctly configured.
