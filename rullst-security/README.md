# Rullst Security 🛡️⚡

`rullst-security` is the dedicated security suite for the **Rullst Framework**. It uses bounded in-memory state, established cryptographic primitives, and defense-in-depth middleware. Its WAF/RASP rules are heuristic controls and must be combined with secure application design, authentication, authorization, TLS, monitoring, and timely dependency updates.

Every control here is opt-in: `rullst-core`'s `Server` already mounts the
runtime baseline (headers, CORS, a small WAF and CSRF) in staging and
production, and nothing in this crate protects traffic until you mount the
layer or call the helper. Read
[which security layer to use, and when](https://rullst.github.io/Rullst/book/security-layers.html)
before combining the two. Each module's rustdoc states the risk it reduces,
how, its known limits and what you must still do.

---

## 🌟 Modules & Features

### 🍯 1. Rullst Honey (`rullst_security::honey`)
*Trap-path detection and local peer bans*
- **Synthetic Honeypot Traps:** Refuses requests for exact trap paths that scanners probe, such as `/.env`, `/admin.php`, `/wp-login.php` and `/.git/config`. Scanners that never request a trap path are not detected.
- **Bounded In-Memory Ban List:** Tracks verified socket peers with an explicit TTL and cardinality limit. A request checks only its own peer; expired bans are pruned in expiry order when bans are added or counted, and a full list evicts the ban that expires soonest.
- **Exact Route Matching:** Trap paths are matched as complete paths; untrusted forwarding headers are not used as ban identities.
- **Local, per-address bans:** A ban covers one exact IP address in one process. Rotating addresses (an IPv6 prefix, a botnet) avoids it, other instances do not see it, and a shared NAT address can be banned by one scanner behind it.
- **Lure-Resistant Bans:** Every trap hit is refused, but only a direct request bans its peer. A load that a page initiated (`Sec-Fetch-Site` of `same-origin`, `same-site` or `cross-site`, or `Origin`/`Referer` from a browser without fetch metadata) is recorded without a ban, so an `<img src="/.env">` on another site or in user content cannot ban visitors or a shared NAT address. These headers are client-controlled: a scanner can avoid the ban, not the refusal, by sending them.

### 🧹 2. Rullst Sanitizer (`rullst_security::sanitizer`)
*XSS risk reduction for user-supplied HTML, and CSP nonces*
- **Allowlisted HTML Sanitization:** Uses `ammonia`'s default allowlist to strip scripts, inline event handlers, unsafe attributes, and unsupported SVG/HTML instead of trying to make arbitrary markup safe. Allowed links and images can still point to external sites, and `sanitize_text` escaping is correct only for HTML text and quoted attributes.
- **Dynamic Content Security Policy (CSP):** Generates cryptographically secure base64 nonces (`nonce-<random>`) per HTTP request.
- **Clickjacking & Security Headers:** Enforces `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, and strict `Referrer-Policy`.

### 🛡️ 3. Rullst RBAC Guard (`rullst_security::rbac`)
*Role-Based Access Control & BOLA/IDOR Defense*
- **Declarative Authorization:** Inspects `UserContext` roles and fine-grained capabilities (`RbacGuard::authorize`).
- **BOLA / IDOR checks:** `RbacGuard::authorize_owner_or_role` compares the resource owner with the authenticated subject. It protects only the handlers that call it, with an owner taken from your database rather than from the request.

### 📜 4. Rullst Audit Log (`rullst_security::audit`)
*HMAC-SHA256 Tamper-Evident Trail*
- **Canonical Event Chaining:** Signs a versioned, domain-separated, length-prefixed encoding of the sequence, timestamp, event fields, and predecessor hash.
- **Offline Integrity Checks:** `verify_record` checks one record's HMAC; `verify_sequence` additionally validates genesis, monotonic sequence IDs, and predecessor continuity.
- **Extensible Sinks:** Provides `AuditLogger` trait for database ORM logging or Cloud-Native stdout/JSON sinks.

### 🔑 5. TOTP Recovery Codes (`rullst_security::recovery_codes`)

- **One-time plaintext:** 80-bit codes are returned only during enrollment and zeroized on drop.
- **Storage-safe records:** Persist only the subject-bound salted HMAC-SHA256 verifiers.
- **Single-use contract:** `consume_recovery_code` removes one verifier; database-backed callers must make compare-and-delete transactional.

### 📲 6. MFA & Login Abuse Controls

- **TOTP enrollment:** OS-random 160-bit secrets, RFC 6238 code generation,
  constant-time six-digit verification, `otpauth://` URIs, and bounded SVG QR
  generation through `build_mfa_qr_svg`.
- **TOTP replay protection:** `verify_totp_code` is stateless and accepts the
  previous, current and next 30-second step, so an observed code verifies
  again for about 90 seconds. To meet RFC 6238 section 5.2, call
  `verify_totp_step_after(secret, code, last_accepted_step)`, which accepts
  only a step newer than the last accepted one, and persist the returned step
  per secret atomically (a conditional update) before admitting the login.
  `verify_totp_step` returns the matched step without replay state.
- **Applied tarpit:** `LoginGuard::record_login_failure_and_wait` records a
  failure and awaits its progressive delay; jail state is bounded and local to
  the process. Concurrent admission shares the configured identity ceiling.
  At capacity, a new identity evicts the least recently failed counter and a
  new offender evicts the jail that expires soonest, so a flood of unrelated
  identities cannot stop a later identity from being counted and jailed. It
  can still age out older counters, so pair the jail with upstream rate limits.
- **Local rate limiter:** Fixed-window counters retain at most 16,384 identities
  with keys up to 256 bytes, reclaim expired identities on subsequent requests,
  and reject zero budgets or exhausted admission. Clones share state; this is
  not a distributed limit across application instances. Prefer one
  `RateLimiter` per policy. The legacy global `is_rate_limited` helper keeps a
  separate budget per `(key, max_requests, window)`, so policies on the same
  key neither share a count nor reset each other, but they share its capacity.
  `rate_limit_middleware` keys the verified socket peer per IPv4 address and
  per IPv6 /64 (IPv4-mapped IPv6 counts as IPv4), so rotating addresses inside
  one delegated prefix shares a budget instead of filling the identity table.
- **Shared Redis limiter:** The opt-in `redis-rate-limit` feature adds
  `RedisRateLimiter`, an atomic fixed-window Lua counter with hashed client
  keys. Empty or `mock_*` URLs select a bounded process-local test mode; call
  `require_distributed()` at production startup.

### 🔎 7. Bounded Payload, Log & Asset Guards

- **Schema Guard:** Rejects malformed JSON, recursive duplicate keys, excessive
  body size/depth, and ambiguous JSON content types. An application can also
  compile one bounded JSON Schema 2020-12 document or one explicit OpenAPI 3.1
  component into route-scoped middleware. References stay local, pattern
  matching uses the linear-time regex engine, and schema construction performs
  no filesystem or network retrieval. An empty `GET`, `HEAD` or `OPTIONS`
  body (and, for the global guard, `DELETE`) passes even when the client sends
  a JSON `Content-Type`.
- **Response DLP:** `mask_response_payload` and `DlpResponseLayer` mask
  complete PEM private-key blocks (PKCS#8 plain or encrypted, RSA, EC, DSA,
  OpenSSH and OpenPGP), AWS access-key IDs and `postgres`/`postgresql`/`mysql`/`redis`/`rediss`
  URL passwords in bounded textual responses (at most 2 MiB). The layer treats
  `text/*` (except `text/event-stream`), JSON, XML (`application/xml` and
  `+xml` types such as SOAP and Atom), YAML and `application/javascript`
  (with its `x-javascript`/`ecmascript` aliases) as textual, in any ASCII
  case; other media types pass through unchanged. A `206 Partial Content`
  response that would need masking is replaced by a `502` with
  `Cache-Control: no-store`, because a masked range no longer matches its
  `Content-Range`; a clean range passes through unchanged. A
  `multipart/byteranges` response is split into its parts and each textual,
  identity-encoded part is checked the same way; it is withheld when any part
  would need masking or when it cannot be split exactly (an invalid boundary
  or delimiter line, more than 256 parts, a part header block over 8 KiB or
  no close delimiter). Each range is inspected on its own, so a secret split
  across separately requested ranges is not recognized. Every pass is
  linear in the body length. A URL password is recognized only inside the URL
  authority: credentials must be percent-encoded, and the authority ends at
  the first `/`, `?`, `#`, whitespace, quote, `<`, `>`, backtick or control
  character, or after 2,048 bytes.
- **Log redaction:** `redact_secrets` handles repeated Bearer/assignment, PEM,
  AWS, and database patterns, including escaped quoted values and keys and
  values of JSON embedded in a JSON string (`{"body":"{\"password\":\"..\"}"}`),
  in time linear in the record length. An assignment key (`password`,
  `passwd`, `secret`, `api_key`/`api-key`, `apikey`, `token`,
  `authorization`, `cookie`, `session`, in any ASCII case) also matches as
  the final component of a compound name joined by `_`, `-` or `.`
  (`DB_PASSWORD`, `access_token`, `client-secret`, `app.db.password`) or when
  a final `key`/`id` component follows it (`SECRET_KEY`, `session_id`).
  An unquoted `Authorization`/`Proxy-Authorization` or `Cookie`/`Set-Cookie`
  value is redacted to the end of its line, so later cookies and credentials
  containing spaces are covered; only a recognized authentication scheme such
  as `Bearer`, `Basic`, `Digest` or `Token` is kept. Records over
  64 KiB are replaced wholesale by an oversized-record marker. A redacted
  record increments only the log-redaction counter; it is never counted or
  announced as a blocked HTTP response (`dlp_secrets_masked`,
  `DLP_SECRET_LEAK_PREVENTED`). The host must
  invoke it before emitting untrusted log fields; pattern matching is not a
  guarantee that arbitrary sensitive content can be recognized.
- **SRI:** Generate escaped SHA-384 tags from bytes or bounded local JS/CSS
  files with `sri_script_tag_from_file` and `sri_link_tag_from_file`.

### 🧱 Request inspection (RASP) and prompt heuristics

- **RASP:** `RaspSecurityLayer` refuses requests whose target, non-credential
  headers or bounded textual body (at most 1 MiB, identity-encoded) contain
  SQL injection, traversal, SSRF, shell or JNDI signatures, checked raw and
  after one percent-decoding pass. Cookies, `Authorization`, multipart and
  binary bodies are not inspected; double encoding and alternative syntax
  bypass it, and free text containing a signature is refused. `Server`
  already runs the overlapping Core WAF in staging and production.
- **LLM prompt filter:** `LlmFirewall` and `ai_firewall_middleware` match a
  fixed list of English override phrases, prompt-leak requests, chat-template
  tokens, Markdown image beacons and invisible characters. Rephrased,
  translated or encoded instructions and indirect injection through retrieved
  content pass; treat model output as untrusted regardless.
- **Session fingerprint (`zero_trust`):** an HMAC of `User-Agent`,
  `Accept-Language` and the client's /24 or /64. All three are
  client-controlled or shared, so a copying attacker on the same network
  passes and real users can fail after a browser or network change. The
  module name is historical; it is not a zero-trust architecture.
- **Timing guard:** pads responses to a minimum duration with jitter. Slower
  paths are not padded, and the synthetic CPU work does not match a real
  password hash; verify against a dummy hash to hide whether an account
  exists.

### 🧩 8. Deterministic Threat Sentinel

- **Explainable assessment:** Classifies caller-supplied aggregate windows
  against explicit credential-stuffing, API-scraping and
  distributed-automation thresholds; it does not claim AI attribution.
  `SentinelObservation` deserialization applies the same validation as
  `try_new`, so a zero window, zero requests or inconsistent counts are
  rejected rather than reaching the classifier.
- **Bounded proof of work:** Issues OS-random, HMAC-authenticated, subject-bound
  challenges with bounded TTL, difficulty and local cardinality.
- **One-shot verification:** Exactly one concurrent verifier consumes an active
  challenge in the current process. Distributed replay state and traffic
  identity remain application/deployment responsibilities.
- **Limits:** the classifier sees only the counts you supply, and proof of
  work slows automation without stopping an attacker with spare compute. Offer
  an accessible alternative for low-power devices.

### 💾 9. Durable Local SIEM Spool

- **Restart validation:** `DurableSiemSpool` synchronously appends normalized,
  unsigned `LiveSecurityEvent` v1 records and validates the whole bounded file
  when it is reopened.
- **Fail-closed framing:** A version header, exact length and SHA-256 digest
  detect truncation and accidental modification; byte and record ceilings stop
  unbounded growth.
- **Explicit boundary:** This is a single-process local spool. SHA-256 is not
  source authentication, and the application still owns trusted directories,
  permissions, rotation, retention, backup, delivery, retry, acknowledgement,
  dead-letter policy and any Splunk/Datadog/Elastic/Syslog adapter.

### 🔐 10. Authenticated Local SIEM Journal

- **HMAC chain:** `AuthenticatedSiemSpool` binds every normalized event to its
  one-based sequence, named key, predecessor tag and exact payload using
  domain-separated HMAC-SHA256.
- **Explicit rotation:** `SiemKeyRing` writes with one active key and verifies
  older frames with at most seven historical keys. Secret bytes live in
  zeroizing storage and are redacted from `Debug`.
- **Fail-closed recovery:** Restart rejects forged content, wrong or absent
  keys, reordered/removed interior frames, malformed encoding and quota or
  external-length violations. A trusted external checkpoint is still needed
  to detect removal of a complete valid tail.

---

## 📦 Installation

Add `rullst-security` to your `Cargo.toml`:

```toml
[dependencies]
rullst-security = "12.1.0"
```

---

## ⚡ Quickstart Code Examples

### 1. Honeypot & CSP Middleware Setup

```rust
use axum::{Router, routing::get};
use rullst_security::{HoneypotLayer, HoneypotState, CspSecurityLayer};

#[tokio::main]
async fn main() {
    let state = HoneypotState::default(); // Catches /.env, /admin.php, etc.

    let app = Router::new()
        .route("/api/data", get(|| async { "Protected Data" }))
        .layer(CspSecurityLayer::default())
        .layer(HoneypotLayer::new(state));
}
```

### 2. XSS HTML Sanitization

```rust
use rullst_security::HtmlSanitizer;

let dirty_input = "<script>alert('xss')</script><p>Clean Text</p>";
let safe_html = HtmlSanitizer::sanitize(dirty_input);

assert_eq!(safe_html, "<p>Clean Text</p>");
```

### 3. Role & Ownership Authorization (RBAC)

```rust
use rullst_security::{UserContext, RbacGuard};

let user = UserContext::new("usr_100", vec!["editor".to_string()]);

// Authorize role
let is_allowed = RbacGuard::authorize(&user, "editor");
assert!(is_allowed.is_ok());

// Authorize owner or admin
let resource_owner = "usr_100";
let is_owner = RbacGuard::authorize_owner_or_role(&user, resource_owner, "admin");
assert!(is_owner.is_ok());
```

### 4. Cryptographic Audit Log Chain

```rust
use std::sync::Arc;
use rullst_security::{AuditChain, StdoutAuditLogger};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let secret = b"my-master-hmac-secret-key-32-bytes";
    let logger = Arc::new(StdoutAuditLogger::default());
    let chain = AuditChain::try_new(secret, logger)?;

    let record = chain
        .record_event("admin_user", "UPDATE_ROLE", "user_456", "{\"role\":\"admin\"}")
        .await?;

    assert!(AuditChain::verify_record(secret, &record));
    Ok(())
}
```

A new chain starts at sequence 1 from the genesis predecessor. After a
restart, continue the persisted trail with `AuditChain::try_resume(secret,
logger, &tip)` (unpublished v13), where `tip` is the newest persisted record
and must verify with the key; otherwise `verify_sequence` rejects the retained
trail. One writer must own each persisted chain.

### 5. TOTP Enrollment QR

```rust
use rullst_security::{build_mfa_qr_svg, try_generate_mfa_secret};

fn enrollment_qr() -> Result<String, rullst_security::SecurityError> {
    let secret = try_generate_mfa_secret()?;
    build_mfa_qr_svg("My Rullst App", "alice@example.com", &secret)
}
```

Store the secret encrypted, show the QR only during a protected enrollment
ceremony, and require a verified code before enabling MFA.

### 6. Route-scoped JSON Schema enforcement

```rust
use axum::{Router, middleware, routing::post};
use rullst_security::{
    JsonSchemaPolicy, SchemaPolicyError, json_schema_guard_middleware,
};
use serde_json::json;

fn schema_routes() -> Result<Router, SchemaPolicyError> {
    let policy = JsonSchemaPolicy::from_schema(json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": { "name": { "type": "string", "minLength": 1 } },
        "required": ["name"],
        "additionalProperties": false
    }))?;

    Ok(Router::new().route(
        "/users",
        post(|| async { "accepted" }).layer(middleware::from_fn_with_state(
            policy,
            json_schema_guard_middleware,
        )),
    ))
}
```

The compiled layer returns `415` for unsafe requests with a non-JSON media
type, `400` for malformed/duplicate/oversized/deep JSON, and `422` for a valid
JSON value that does not match the selected schema. Authentication,
authorization, ownership and domain validation remain separate.

### 7. Opt-in proof-of-work assessment

```rust
use rullst_security::{
    ProofOfWorkConfig, SentinelObservation, SentinelPolicy, ThreatSentinel,
};
use std::time::Duration;

fn assess_login_window() -> Result<bool, rullst_security::SentinelError> {
    let sentinel = ThreatSentinel::try_new(
        b"replace-with-at-least-32-high-entropy-secret-bytes",
        SentinelPolicy::default(),
        ProofOfWorkConfig::default(),
    )?;
    let signals = SentinelObservation::try_new(
        Duration::from_secs(60), 25, 20, 8, 3, 1, 0,
    )?;
    Ok(sentinel
        .assess("account-or-device:opaque-id", signals)?
        .challenge()
        .is_some())
}
```

The host must derive the subject from trusted application state, provide an
accessible alternative, limit issuance and explicitly verify the returned
token/nonce before admitting the protected operation.

### 8. Persist a bounded local SIEM event

```rust
use rullst_security::{DurableSiemSpool, LiveSecurityEvent, SiemSpoolError};

fn persist_denial() -> Result<u64, SiemSpoolError> {
    let spool = DurableSiemSpool::try_open("var/security-events.spool")?;
    let receipt = spool.append_local(LiveSecurityEvent::local(
        "RBAC_ACCESS_DENIED",
        "owner mismatch",
        "192.0.2.50",
    ))?;
    Ok(receipt.sequence())
}
```

Create and permission the parent directory before opening the spool, use only
one writer process per file, and deliver/rotate it through an application-owned
operator. Reopening validates frames but does not authenticate the event source
or confirm remote receipt.

### 9. Persist and verify an authenticated local SIEM event

```rust
use rullst_security::{
    AuthenticatedSiemSpool, AuthenticatedSiemSpoolError, LiveSecurityEvent,
    SiemIntegrityKey, SiemKeyRing,
};

fn persist_authenticated() -> Result<(), AuthenticatedSiemSpoolError> {
    let active = SiemIntegrityKey::try_new(
        "security-2026-09",
        b"0123456789abcdef0123456789ABCDEF".to_vec(),
    )?;
    let keys = SiemKeyRing::try_new(active, std::iter::empty())?;
    let spool = AuthenticatedSiemSpool::try_open("var/security-events.auth", keys)?;
    spool.append_local(LiveSecurityEvent::local(
        "RBAC_ACCESS_DENIED",
        "owner mismatch",
        "192.0.2.50",
    ))?;
    let verified = spool.read_verified()?;
    assert!(verified.iter().all(|event| event.verified_hmac));
    Ok(())
}
```

Generate the key from a cryptographic secret manager rather than using the
example literal. During rotation, reopen with the new active key and every
historical key referenced by retained frames. This API does not send events to
an external SIEM or acknowledge remote receipt.

---

## 📖 License

Licensed under the MIT License. Part of the **Rullst Monorepo Framework**.
