# rullst-security 🛡️

`rullst-security` is the official high-assurance security suite of the Rullst Framework. It provides honeypot deception, XSS/SVG HTML sanitization, dynamic CSP nonces, role-based access control (RBAC), and HMAC cryptographic audit logs.

## 📦 Features & Modules

### 🍯 Rullst Honey (`rullst-honey`)
Synthetic deception routes (`/.env`, `/admin.php`, `/.git/config`) and invisible form inputs that fingerprint and ban malicious bots in memory (`DashMap`) and WAF.

### 🧹 Rullst Sanitizer (`rullst-sanitizer`)
XSS and SVG sanitization engine powered by `ammonia`, plus `CspSecurityLayer` middleware generating dynamic per-request CSP nonces and Clickjacking headers (`X-Frame-Options: DENY`).

### 🛡️ Rullst RBAC Guard (`rullst-rbac`)
Declarative role authorization (`UserContext`, `RbacGuard`) preventing IDOR/BOLA attacks via `authorize_owner_or_role`.

### 📜 Rullst Audit Log (`rullst-audit-log`)
HMAC-chained cryptographic tamper-proof audit log (`AuditChain`) preserving event integrity during database breaches.

### ⚡ RASP - Runtime Application Self-Protection (`rullst-security::rasp`)
Zero-latency middleware (`RaspSecurityLayer`, `RaspInspector`) inspecting URI parameters and payload strings to intercept SQL Injection, Path Traversal, SSRF, and RCE attacks before reaching controllers.

### 📊 Visual Threat Radar (SOC)
Visual dashboard integrated into Rullst Studio (`http://localhost:5555/studio/security`) showing real-time threat vectors, banned IP counts, and HMAC audit chain status.

## 🚀 Usage Example

```rust
use axum::{Router, routing::get};
use rullst_security::{HoneypotLayer, HoneypotState, CspSecurityLayer, HtmlSanitizer};

#[tokio::main]
async fn main() {
    let state = HoneypotState::default();

    let app = Router::new()
        .route("/api/data", get(|| async { 
            let safe_input = HtmlSanitizer::sanitize("<b>Clean Text</b>");
            safe_input
        }))
        .layer(CspSecurityLayer::default())
        .layer(HoneypotLayer::new(state));
}
```
