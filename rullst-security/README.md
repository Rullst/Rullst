# Rullst Security 🛡️⚡

`rullst-security` is the dedicated, high-performance security suite for the **Rullst Framework**. Built with a strict **Zero-Panic Policy**, zero-latency in-memory tracking, and high-assurance cryptographic primitives, it shields web applications against modern threat vectors without compromising request throughput.

---

## 🌟 Modules & Features

### 🍯 1. Rullst Honey (`rullst::security::honey`)
*Deception Security & Botnet Mitigation Engine*
- **Synthetic Honeypot Traps:** Intercepts reconnaissance bots attempting to scan paths like `/.env`, `/admin.php`, `/wp-login.php`, `/.git/config`.
- **Zero-Latency In-Memory Ban List:** Uses `DashMap` for sub-millisecond IP lookup before request execution.
- **WAF Integration:** Automatically tags and syncs malicious IPs with the global `Rullst Shield`.

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
*HMAC Cryptographic Tamper-Proof Trail*
- **Cryptographic Event Chaining:** Formats each log as `hash_n = HMAC_SHA256(seq:time:actor:action:resource:payload:hash_{n-1})`.
- **Breach-Resistant Verification:** Allows offline audit verification (`verify_record`) to detect database tampering during security incidents.
- **Extensible Sinks:** Provides `AuditLogger` trait for database ORM logging or Cloud-Native stdout/JSON sinks.

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
async fn main() {
    let secret = b"my-master-hmac-secret-key";
    let logger = Arc::new(StdoutAuditLogger::default());
    let chain = AuditChain::new(secret, logger);

    let record = chain
        .record_event("admin_user", "UPDATE_ROLE", "user_456", "{\"role\":\"admin\"}")
        .await
        .unwrap();

    assert!(AuditChain::verify_record(secret, &record));
}
```

---

## 📖 License

Dual-licensed under MIT License. Part of the **Rullst Monorepo Framework**.
