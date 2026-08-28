# Rullst Security 🛡️⚡

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
- **HTML/SVG Sanitization:** Uses `ammonia` to strip malicious `<script>` tags, inline event handlers, and unsafe attributes.
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

---

## 📦 Installation

Add `rullst-security` to your `Cargo.toml`:

```toml
[dependencies]
rullst-security = "12.0.0"
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

---

## 📖 License

Dual-licensed under MIT License. Part of the **Rullst Monorepo Framework**.
