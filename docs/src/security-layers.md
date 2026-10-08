# Which security layer to use, and when

Rullst has two places for security code, and some of their controls look
alike:

- **`rullst-core`** (`rullst::security`) owns the **runtime baseline**: secure
  headers with a CSP nonce, CORS, a small WAF, double-submit CSRF, optional
  response PII masking, trusted-proxy client resolution, exact machine
  endpoints and tenant context. `Server` mounts the baseline for you in staging
  and production.
- **`rullst-security`** (`rullst::security_runtime`, or the `rullst_security`
  crate that generated applications already depend on) is a toolbox of
  **opt-in** layers and helpers: RASP, response DLP, configurable headers,
  honeypots, CSWSH, JSON schema guards, rate limiters, the login jail, MFA,
  RBAC, audit chains, field encryption, SRI and more. Nothing in it protects
  traffic until you mount or call it.

This page tells you which one to pick for each job, gives one recommended
production stack and states what each layer does **not** protect against. It
summarizes the code in `rullst-core/src/security/` and `rullst-security/src/`;
the [security architecture](security-architecture.md) page holds the deeper
contracts, and both crates are in the Core [maturity tier](maturity.md).

## The short answer

1. Run your application through `Server` and configure `[security]` in
   `Rullst.toml`. In staging and production that gives you headers, CORS, the
   WAF and CSRF without extra code.
2. Behind a reverse proxy, set `Server::trusted_proxies` (or
   `[security] trusted_proxies`) to your proxy networks only.
3. Add from `rullst-security` only what your application needs: the honeypot
   and response DLP for every app, the CSWSH guard on WebSocket routes, a JSON
   schema guard on JSON write routes, and the login jail, MFA, RBAC and audit
   chain inside your handlers.
4. Do not stack two layers that do the same job and expect double protection.
   The pairs below explain which one wins and what each one misses.

## What generated applications already have

| Blueprint | Mounted for you | Not mounted |
| :--- | :--- | :--- |
| Every blueprint | `Server` applies the Core baseline in **staging and production**: secure headers, CORS (only when `cors_allow_origins` is set), WAF and CSRF. PII masking stays off unless `enable_pii_masking = true`. In **development** the baseline adds only CORS (and machine-endpoint authentication when configured). | No `rullst-security` middleware layer (RASP, DLP, `SecureHeadersLayer`, honeypot, CSWSH, schema guard) is mounted by any blueprint. No global rate limit or Traffic Shield is configured. Trusted proxies are configured only when `cargo rullst deploy --platform vps` writes its Caddy address into `[security] trusted_proxies`. |
| Blank (full stack), SaaS, LMS, ERP | Core `csrf_middleware` and `headers_middleware` on the router too, so development behaves like production. | — |
| Blank JSON API | Core `headers_middleware` on the router. The example write route `POST /api/messages` is an exact machine endpoint (`Server::with_machine_endpoints`): clients send `Authorization: Bearer <API_TOKEN>` (a random value in the generated `.env`), requests with cookies are refused, and startup fails without `API_TOKEN`. | No CSRF layer on the router. In staging and production the `Server` baseline requires CSRF on every other write route, so add each new JSON write route to `machine_endpoints()` in `src/main.rs`, or send the double-submit token from a browser. |
| Blog, Portfolio | Only the `Server` baseline (staging and production). | No router-level layers in development. |
| SaaS, LMS | A Core token-bucket limit on the credential routes; the SaaS `[security]` section exempts the exact signed billing webhook path from CSRF. LMS uses `rullst_security::{UserContext, RbacGuard}`. | Login jail, MFA (add it with `make:mfa`), audit chain. |

Every generated `Cargo.toml` already lists `rullst-security`, so adding a layer
needs no new dependency.

Since 13.0 every generated project also has `src/security_tests.rs`, which
`cargo test` runs offline against the project's own `router()` wrapped in the
staging/production baseline (`apply_security_baseline`, as `Server` composes
it). It checks the security headers, that a write without the CSRF token is
refused and one with it passes, and that the WAF refuses an injection probe
but accepts the prose "Please select an option". SaaS and LMS add the sign-in
rate limit, LMS the owner check of its lesson routes, and the Blank JSON API
the bearer token of its machine endpoint. Keep these tests passing as you
change routes; they do not replace your own authorization tests.

## Recommended production composition

From the outermost layer to your handler:

| # | Layer | Owner | How you enable it |
| :---: | :--- | :--- | :--- |
| 1 | Trusted-proxy client resolution | Core | `Server::trusted_proxies` or `[security] trusted_proxies` |
| 2 | Secure headers and CSP nonce | Core baseline | Automatic in staging/production; tune `[security] csp` and `coep` |
| 3 | CORS allowlist | Core baseline | `[security] cors_allow_origins` |
| 4 | WAF | Core baseline | Automatic in staging/production |
| 5 | CSRF (and exact machine endpoints) | Core baseline | Automatic; `Server::with_machine_endpoints` for API, webhook and mTLS routes |
| 6 | Readiness and drain, Traffic Shield, per-peer rate limit | Core `Server` | `with_lifecycle`, `shield`, `rate_limit` |
| 7 | Honeypot trap paths and peer bans | `rullst-security` | `HoneypotLayer` |
| 8 | Response secret masking | `rullst-security` | `DlpResponseLayer` |
| 9 | Per-route guards | `rullst-security` | `cswsh_guard_middleware` on WebSocket routes, `json_schema_guard_middleware` on JSON write routes |
| 10 | Session, authentication, tenant and authorization | Your application | Your auth middleware, `TenantContext`, `RbacGuard` |
| 11 | Inside handlers | Both | `LoginGuard`, a `RedisRateLimiter` for credential routes, TOTP and recovery codes, `AuditChain`, `FieldEncryptor` |

`Server` composes layers 1 to 6 itself (see `rullst-core/src/server/stack.rs`);
you add 7 to 11 to your router. The [canonical production preset](security-architecture.md#canonical-production-preset)
is the full machine-readable ordering contract, including request-body
limits, request IDs and tracing.

```rust,ignore
use axum::{Extension, Router, middleware::from_fn};
use rullst::security::TrustedProxyConfig;
use rullst::{RateLimitConfig, RateLimiter, Server};
use rullst_security::{
    CswsPolicy, DlpResponseLayer, HoneypotLayer, HoneypotState, cswsh_guard_middleware,
};

/// `pages` holds the application routes, with session, authentication and
/// RBAC checks inside them; `live` holds the WebSocket upgrade routes.
async fn serve(pages: Router, live: Router) -> Result<(), Box<dyn std::error::Error>> {
    // Layer 9: reject cross-site WebSocket upgrades.
    let live = live
        .route_layer(from_fn(cswsh_guard_middleware))
        .layer(Extension(CswsPolicy::try_new(["https://app.example.com"])?));

    let app = pages
        .merge(live)
        // Layer 8: mask private keys, AWS key IDs and database URL passwords.
        .layer(DlpResponseLayer)
        // Layer 7: refuse trap paths such as /.env and ban direct scanners.
        .layer(HoneypotLayer::new(HoneypotState::default()));

    Server::new(app.into())
        // Layer 1: accept forwarded client addresses only from your proxies.
        .trusted_proxies(TrustedProxyConfig::new(["10.0.0.0/8"])?)
        // Layer 6: process-local token bucket per IPv4 address or IPv6 /64.
        .rate_limit(RateLimiter::new(RateLimitConfig::per_minute(600.0)))
        .run(3000)
        .await?;
    Ok(())
}
```

The snippet is marked `ignore`: it shows the shape and the real API names, but
the book does not compile it. Replace the example origin and proxy network with
your own. `rullst::RateLimiter` (Core token bucket) and
`rullst_security::RateLimiter` (fixed window) are different types. The WAF,
RASP and schema guards buffer at most 1–2 MiB of a body themselves; set an
explicit limit on upload routes too (for example axum's `DefaultBodyLimit`).

Leave these out unless you have a specific reason:

- **`RaspSecurityLayer`** overlaps the Core WAF and buffers the same request
  body a second time. Add it when you need its extra signatures (cloud
  metadata hosts, JNDI lookups and every non-credential header) and accept
  more false positives.
- **Core PII masking** rewrites e-mail addresses and long digit runs in every
  textual response, including ones your users are meant to see. Prefer
  redacting fields in your own serializers.
- **`SecureHeadersLayer` inside `Server`** only when you need values that
  `[security]` cannot express: the Core baseline then keeps every header the
  layer chose or omitted (see below).

## Overlapping layers compared

### Secure headers: Core baseline vs `SecureHeadersLayer`

| | Core `headers_middleware` | `rullst_security::SecureHeadersLayer` |
| :--- | :--- | :--- |
| Mounted by | `Server` in staging/production; most blueprints also mount it on the router | You, as a Tower layer |
| Configuration | `[security] csp` (template with `{NONCE}`) and `coep`; other values fixed | `SecureHeadersConfig`: every header value replaceable or omitted (`None`), CSP template or static CSP |
| Headers | HSTS (two years, preload), `X-Frame-Options: DENY`, `nosniff`, `X-XSS-Protection: 0`, `Referrer-Policy`, `Permissions-Policy`, COOP, CORP, COEP, CSP | The same set, from the config |
| Existing values | Adds each header only when the response has none; leaves all of them alone after an inner `SecureHeadersLayer` | Replaces the handler's values with its configured ones |
| `Cache-Control` | Adds `no-store` when the handler set none | Not touched |
| CSP nonce | `CspNonce` request extension | Same `CspNonce`, reused when one already exists |

Both layers reuse one `CspNonce` per request, so templates render a nonce that
matches the final header. The Core baseline is a fallback: it fills in a
security header only when the handler or an inner layer has not set it, so an
**explicit value set by the application wins over the baseline**.
`SecureHeadersLayer` marks its responses with
`rullst::security::SecurityHeadersApplied`; when the Core baseline (outermost
inside `Server` in staging and production) sees that marker it leaves every
security header to the layer, so the headers `SecureHeadersConfig` sets to
`None` stay absent. It still adds `Cache-Control: no-store` when no cache
policy was set. The trade-off: a weaker value that a handler sets on purpose
or by mistake (for example `Referrer-Policy: unsafe-url` or a CSP with
`'unsafe-inline'`) is no longer replaced, so review the headers your handlers
set. Use `[security]` settings to tune a `Server` application, and
`SecureHeadersLayer` when you need per-header values or omissions, or for a
plain Axum router that you serve without `Server`. An exact
`Referrer-Policy: no-referrer` from a handler survives both layers, normalized
to one value. `CspSecurityLayer` (in `sanitizer`) is a smaller variant: default
CSP, `X-Frame-Options`, `nosniff` and `Referrer-Policy` only; it does not set
the marker, so the Core baseline keeps those four and adds the rest.

**Not covered:** headers reduce browser-side risks (framing, MIME sniffing,
inline script injection); they do not fix an XSS bug in a page that allows
`'unsafe-inline'` or a trusted script host that serves attacker content, and
HSTS has no effect until the first HTTPS response reaches the browser.

### Request inspection: Core WAF vs RASP

| | Core `waf_middleware` | `rullst_security::RaspSecurityLayer` |
| :--- | :--- | :--- |
| Mounted by | `Server` in staging/production | You |
| Signatures | Injection structure, not keywords. SQL: a quote followed by `or`/`and` and a comparison or by a comment (`' or '1'='1`, `admin'--`), a `;` followed by a statement (`; drop table`, `; delete from`, `; select *`), `union [all] select`, and probes (`sleep(`, `benchmark(`, `waitfor delay`, `pg_sleep(`, `information_schema`, `@@version`, `xp_cmdshell`, `load_file(`, `into outfile`). Shell: `;`, `\|`, `&&`, a backtick or `$(` followed by a command name (`sh`, `bash`, `cat`, `ls`, `id`, `curl`, `wget`, `nc`, `rm`, `python`…). XSS (`<script`, `javascript:`, `onload=`, `onerror=`, `document.cookie`) and traversal (`../`, `..\`, `/etc/passwd`, `win.ini`) as substrings | More specific phrases: SQL (`union select`, `' or '1'='1`, `sleep(`, `information_schema`…), traversal, SSRF hosts (`169.254.169.254`, `metadata.google.internal`), shell (`cmd.exe`, `/bin/sh`…), JNDI (`${jndi:`…) | More specific phrases: SQL (`union select`, `' or '1'='1`, `sleep(`, `information_schema`…), traversal, SSRF hosts (`169.254.169.254`, `metadata.google.internal`), shell (`cmd.exe`, `/bin/sh`…), JNDI (`${jndi:`…) |
| Where it looks | Decoded path (traversal only), query, `Referer`, each cookie pair, `User-Agent` against a configurable blocklist (AI and SEO crawlers by default) | Full request target, every header except `Cookie` and `Authorization`, body; JSON keys and strings after decoding |
| Bodies | `text/*`, JSON, XML, URL-encoded forms; identity encoding only (others get `415`); at most 1 MiB (`413`); invalid UTF-8 gets `400` | Same media types and limits |
| Decoding | One percent-decoding pass, then lowercase with whitespace runs collapsed, checked with and without `/* … */` comments; JSON bodies are checked key by key and string by string | Raw text plus one percent-decoding pass |
| Telemetry | None | `SecurityStore` interception counters and events |

The Core WAF is a coarse baseline. Ordinary text that names SQL or shell
words ("please select an option", "delete my account", "curl the API with your
token") passes; only injection syntax around those words is refused.
Parameterized SQL is the real defense against SQL injection, and shell-free
process APIs against command injection.

**Not covered by either:** `multipart/form-data` and other binary bodies,
double-encoded or otherwise obfuscated payloads (HTML entities, Unicode
escapes outside JSON strings, and in RASP SQL comments between keywords),
payloads split across fields, injections that need no quote, `;` or
`union` (a bare numeric `1 or 1=1`), and every attack class without a listed
signature. Both still produce false positives: the Core WAF refuses text in
which a shell metacharacter precedes a command name (a Markdown cell
`| id |`, inline code such as `` `ls` ``) or a quote precedes `or` and a
comparison, and RASP refuses any field that contains `../` or `/bin/sh`.
Parameterized SQL, output encoding, typed validation and URL allowlists remain
the actual defenses.

### Response masking: Core PII masking vs DLP

Both layers inspect **responses only**; neither looks at requests. (Prompt PII
masking for LLM calls lives in `rullst-ai`.)

| | Core `pii_masking_middleware` | `rullst_security::DlpResponseLayer` |
| :--- | :--- | :--- |
| Enabled by | `[security] enable_pii_masking = true` (staging/production) | You |
| Finds | E-mail addresses; 13–19 digit card-like runs (digits with at most two spaces or hyphens between them; no Luhn check) | Complete PEM private-key blocks (PKCS#8, RSA, EC, DSA, OpenSSH, OpenPGP), `AKIA` access-key IDs, passwords in `postgres`, `postgresql`, `mysql`, `redis` and `rediss` URLs |
| Media types | `text/*` except `text/event-stream`, JSON (masked only inside string values), XML, `application/javascript` | `text/*` except `text/event-stream`, JSON, XML, YAML, `application/javascript` and its aliases |
| Bounds | Fixed-size, identity-encoded bodies up to 2 MiB; streams, unknown sizes, compressed and larger bodies pass unchanged; `HEAD`, `204` and `304` skipped | Same |
| `206 Partial Content` | A range that would change becomes `502` with `no-store`; a clean range passes | Same |
| `multipart/byteranges` | Split into parts (at most 256 parts, 8 KiB of headers per part); withheld with `502` when a textual part would change or the body cannot be split | Same |

**Not covered:** a value split across separately requested ranges, any
compressed or streamed response, other secret formats (temporary `ASIA` keys,
AWS secret keys, API tokens, JWTs, passwords outside a database URL), PII
other than e-mails and card-like numbers, and values encoded in a way the
pattern does not match. Core masking also hides legitimate phone numbers, order
numbers and e-mails. Neither layer stops a leak through logs, files or another
channel; use `redact_secrets` for log fields and keep secrets out of responses
in the first place.

### Browser and abuse controls

| Control | Owner | Reduces | Mechanism | Does not protect against |
| :--- | :--- | :--- | :--- | :--- |
| Double-submit CSRF | Core | Cross-site form and fetch writes using ambient cookies | `rullst_csrf` cookie must match `X-CSRF-Token` or the `_token` form field, compared in constant time | State changes in `GET` handlers; a sibling subdomain that can set cookies; an XSS bug that reads the token; WebSocket upgrades |
| Exact machine endpoints | Core | Webhook and API routes that must skip browser CSRF | Exact method and path, authenticated (bearer digest, signed-webhook or mTLS verifier) before the CSRF exemption applies | A weak verifier you supply; replay unless your verifier checks freshness |
| Trusted-proxy resolution | Core | Spoofed `X-Forwarded-For` choosing the client identity | Forwarded headers read only from listed proxy networks, right to left | A listed proxy that forwards client headers unchanged; any host inside a listed network |
| Tenant context | Core | Client-chosen tenant headers | Only tenant context inserted by your authentication middleware is used | Missing ownership checks in handlers |
| Rate limit (`Server::rate_limit`) | Core | Per-peer request floods | In-memory token bucket per IPv4 address or IPv6 /64, at most 100,000 keys (LRU) | Multiple instances (each has its own budget), botnets, network-level DDoS |
| `RedisRateLimiter` | `rullst-security` (`security-redis`) | Budgets shared across instances | Atomic fixed-window Lua script in Redis | Redis outages and failover semantics; call `require_distributed()` so a mock URL fails at startup |
| `rate_limit_middleware`, `RateLimiter` | `rullst-security` | Per-peer floods in one process | Fixed window, 16,384 keys, fails closed for new keys when full | Same as the Core limiter; prefer one limiter per policy |
| `LoginGuard` | `rullst-security` | Password guessing on one identity | Progressive delay and an in-memory jail per hashed identity | Distributed guessing across identities; restarts and other instances (state is local) |
| Honeypot (`HoneypotLayer`) | `rullst-security` | Automated scanners probing well-known paths | Exact trap paths; direct hits ban the socket peer for 15 minutes by default | Scanners that avoid the paths or rotate addresses; bans are per process and per exact IP |
| `deception_trap_middleware` | `rullst-security` | The same, without bans | Global registry of exact trap paths; `403` plus telemetry | Anything beyond recording the probe |
| CSWSH guard | `rullst-security` | Cross-site WebSocket hijacking with ambient cookies | Exact `Origin` allowlist, or same host/port/scheme when no allowlist is set; missing `Origin` rejected by default | Non-browser clients (they can send any `Origin`); authentication of the socket itself |
| TOTP and recovery codes | `rullst-security` | Stolen passwords | RFC 6238 TOTP, ±1 step; `verify_totp_step_after` rejects replayed steps; HMAC-verified single-use recovery codes | Real-time phishing proxies; replay unless you persist the last accepted step |
| `AuditChain` | `rullst-security` | Silent edits to stored audit records | HMAC-SHA256 chain over sequence, time, fields and predecessor | Deleting the newest records without an external checkpoint; a stolen HMAC key |
| `FieldEncryptor` | `rullst-security` | Readable sensitive columns in a stolen database copy | AES-256-GCM with AAD and key IDs | An attacker with the key or with application access; key custody is yours |
| SRI tags | `rullst-security` | A compromised CDN serving altered JS or CSS | SHA-384 `integrity` attributes | Assets that legitimately change without regenerating the tag |
| JSON schema guard | `rullst-security` | Oversized, deep, duplicate-key or off-schema JSON | Size (2 MiB) and depth (32) limits, duplicate-key rejection, optional JSON Schema 2020-12 policy | Business-rule validation; the global guard passes non-JSON content types through |

## What none of these layers do

- **Authorization.** No layer knows who owns a record. Check ownership and roles
  in every protected handler (`RbacGuard::authorize_owner_or_role`) and test
  the denial.
- **Injection-proof data access.** Use SQLx parameters or
  `sanitize_identifier`; WAF and RASP signatures are a backstop, not a parser.
- **Network-level protection.** Volumetric DDoS, TLS termination, firewalling
  and host hardening belong to your proxy, provider and operating system.
- **Shared state.** Bans, jails and most counters are per process. Multiple
  instances need the Redis limiter or your own shared store.
- **Certification.** Mounting these layers does not make an application
  OWASP-, PCI- or SOC 2-compliant.

To collect static evidence of which of these layers a project mounts (headers
and CSP, CSRF, cookie attributes, rate limiting on credential routes), run
`cargo rullst audit --report`; the [security report guide](security-report.md)
explains each check and its OWASP ASVS 5.0 Level 1 mapping. The
[external audit kit](external-audit-kit.md) packages scope, threat models, a
sample application and tooling for a third-party reviewer.

See the [`rullst-security` crate page](crates/security.md) for module details
and the [security architecture](security-architecture.md) for the deployment
checklist.
