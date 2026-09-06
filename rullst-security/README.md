# Rullst Security 🛡️⚡

> **v12 development notice:** This README documents the unreleased v12 source.
> Use a path dependency from this checkout until an immutable v12 RC exists on
> crates.io. The version below is the planned first RC, not a published claim.

`rullst-security` is the dedicated security suite for the **Rullst Framework**. It uses bounded in-memory state, established cryptographic primitives, and defense-in-depth middleware. Its WAF/RASP rules are heuristic controls and must be combined with secure application design, authentication, authorization, TLS, monitoring, and timely dependency updates.

---

## 🌟 Modules & Features

### 🍯 1. Rullst Honey (`rullst::security::honey`)
*Deception Security & Botnet Mitigation Engine*
- **Synthetic Honeypot Traps:** Intercepts reconnaissance bots attempting to scan paths like `/.env`, `/admin.php`, `/wp-login.php`, `/.git/config`.
- **Bounded In-Memory Ban List:** Tracks verified socket peers with an explicit TTL and cardinality limit.
- **Exact Route Matching:** Trap paths are matched as complete paths; untrusted forwarding headers are not used as ban identities.

### 🧹 2. Rullst Sanitizer (`rullst::security::sanitizer`)
*XSS Prevention & Dynamic CSP Nonces*
- **Allowlisted HTML Sanitization:** Uses `ammonia` to strip scripts, inline event handlers, unsafe attributes, and unsupported SVG/HTML instead of trying to make arbitrary markup safe.
- **Dynamic Content Security Policy (CSP):** Generates cryptographically secure base64 nonces (`nonce-<random>`) per HTTP request.
- **Clickjacking & Security Headers:** Enforces `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, and strict `Referrer-Policy`.

### 🛡️ 3. Rullst RBAC Guard (`rullst::security::rbac`)
*Role-Based Access Control & BOLA/IDOR Defense*
- **Declarative Authorization:** Inspects `UserContext` roles and fine-grained capabilities (`RbacGuard::authorize`).
- **BOLA / IDOR Prevention:** Provides `RbacGuard::authorize_owner_or_role` to enforce resource ownership boundaries dynamically.

### 📜 4. Rullst Audit Log (`rullst::security::audit`)
*HMAC-SHA256 Tamper-Evident Trail*
- **Canonical Event Chaining:** Signs a versioned, domain-separated, length-prefixed encoding of the sequence, timestamp, event fields, and predecessor hash.
- **Offline Integrity Checks:** `verify_record` checks one record's HMAC; `verify_sequence` additionally validates genesis, monotonic sequence IDs, and predecessor continuity.
- **Extensible Sinks:** Provides `AuditLogger` trait for database ORM logging or Cloud-Native stdout/JSON sinks.

### 🔑 5. TOTP Recovery Codes (`rullst::security::recovery_codes`)

- **One-time plaintext:** 80-bit codes are returned only during enrollment and zeroized on drop.
- **Storage-safe records:** Persist only the subject-bound salted HMAC-SHA256 verifiers.
- **Single-use contract:** `consume_recovery_code` removes one verifier; database-backed callers must make compare-and-delete transactional.

### 📲 6. MFA & Login Abuse Controls

- **TOTP enrollment:** OS-random 160-bit secrets, RFC 6238 code generation,
  constant-time six-digit verification, `otpauth://` URIs, and bounded SVG QR
  generation through `build_mfa_qr_svg`.
- **Applied tarpit:** `LoginGuard::record_login_failure_and_wait` records a
  failure and awaits its progressive delay; jail state is bounded and local to
  the process. Concurrent admission shares the configured identity ceiling.
- **Local rate limiter:** Fixed-window counters retain at most 16,384 identities
  with keys up to 256 bytes, reclaim expired identities on subsequent requests,
  and reject zero budgets or exhausted admission. Clones share state; this is
  not a distributed limit across application instances.

### 🔎 7. Bounded Payload, Log & Asset Guards

- **Schema Guard:** Rejects malformed JSON, recursive duplicate keys, excessive
  body size/depth, and ambiguous JSON content types. An application can also
  compile one bounded JSON Schema 2020-12 document or one explicit OpenAPI 3.1
  component into route-scoped middleware. References stay local, pattern
  matching uses the linear-time regex engine, and schema construction performs
  no filesystem or network retrieval.
- **Log redaction:** `redact_secrets` handles repeated Bearer/assignment, PEM,
  AWS, and database patterns, including escaped quoted values. Records over
  64 KiB are replaced wholesale by an oversized-record marker. The host must
  invoke it before emitting untrusted log fields; pattern matching is not a
  guarantee that arbitrary sensitive content can be recognized.
- **SRI:** Generate escaped SHA-384 tags from bytes or bounded local JS/CSS
  files with `sri_script_tag_from_file` and `sri_link_tag_from_file`.

### 🧩 8. Deterministic Threat Sentinel

- **Explainable assessment:** Classifies caller-supplied aggregate windows
  against explicit credential-stuffing, API-scraping and
  distributed-automation thresholds; it does not claim AI attribution.
- **Bounded proof of work:** Issues OS-random, HMAC-authenticated, subject-bound
  challenges with bounded TTL, difficulty and local cardinality.
- **One-shot verification:** Exactly one concurrent verifier consumes an active
  challenge in the current process. Distributed replay state and traffic
  identity remain application/deployment responsibilities.

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
rullst-security = "12.0.0-rc.1"
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
