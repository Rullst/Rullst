# rullst-security 🛡️

`rullst-security` is the official high-assurance security suite of the Rullst Framework. It provides honeypot deception, XSS/SVG HTML sanitization, dynamic CSP nonces, role-based access control (RBAC), HMAC cryptographic audit logs, RASP (Runtime Application Self-Protection), zero-trust vault encryption, and autonomous AI threat analysis.

## 📦 Features & Modules

### 🍯 Rullst Honey (`rullst-honey`)
Synthetic deception routes (`/.env`, `/admin.php`, `/.git/config`) and invisible form inputs that fingerprint and ban malicious bots in memory (`DashMap`) and WAF before reaching application logic.

### 🧹 Rullst Sanitizer (`rullst-sanitizer`)
XSS and SVG sanitization engine powered by `ammonia`, plus `CspSecurityLayer` middleware generating dynamic per-request CSP nonces and Clickjacking headers (`X-Frame-Options: DENY`).

### 🛡️ Rullst RBAC Guard (`rullst-rbac`)
Declarative role authorization (`UserContext`, `RbacGuard`) preventing IDOR/BOLA attacks via `authorize_owner_or_role`.

### 📜 Rullst Audit Log (`rullst-audit-log`)
HMAC-chained cryptographic tamper-proof audit log (`AuditChain`) preserving event integrity during database breaches.

### ⚡ RASP — Runtime Application Self-Protection (`rullst-security::rasp`)
Zero-latency middleware (`RaspSecurityLayer`, `RaspInspector`) inspecting URI parameters and payload strings to intercept SQL Injection, Path Traversal, SSRF, and RCE attacks before controller execution.

### 🔐 Rullst Vault (`rullst-security::vault`)
Zero-trust secret management with in-memory zeroization (`Zeroize`) preventing heap dump leaks (`VaultSecret<T>`) and transparent field-level AES-256-GCM / ChaCha20-Poly1305 database encryption (`#[orm(encrypted)]`).

### 📊 Visual Threat Radar (SOC)
Visual dashboard integrated into Rullst Studio (`http://localhost:5555/studio/security`) displaying active threat attack vectors, live IP reputation scoring, blocked honeypot hits, and AI incident reports.

---

## 🤖 Local AI Security Sentinel & Ollama Integration

Rullst Security supports **100% Offline / Local AI Security Analysis** powered by local LLMs (via Ollama). This allows lightweight models (e.g., `llama3:8b`, `mistral:7b`, `phi3:mini`) to act as autonomous AI threat classifiers and self-healing error consoles without sending sensitive traffic or logs to external cloud APIs.

### Configuring Local AI via Ollama

To use a local Ollama model for AI security scanning and self-healing:

```dotenv
# .env configuration for local Ollama AI model
AI_PROVIDER=ollama
OLLAMA_HOST=http://localhost:11434
AI_MODEL=llama3:8b
```

### Benefits of Local AI Security:
- **Zero Cloud Costs:** Runs entirely on local CPU/NPU/GPU.
- **Air-Gapped Privacy:** No user payloads or internal source code leave your infrastructure.
- **Sub-Second Anomaly Classification:** Evaluates suspicious botnet request clusters locally.

---

## 🚀 Usage Example

```rust
use axum::{Router, routing::get};
use rullst_security::{
    HoneypotLayer, HoneypotState, CspSecurityLayer, HtmlSanitizer,
    RaspSecurityLayer, VaultSecret,
};

#[tokio::main]
async fn main() {
    let state = HoneypotState::default();

    // Zeroize secret upon drop
    let secret = VaultSecret::new("super_secret_api_key".to_string());

    let app = Router::new()
        .route("/api/data", get(|| async { 
            let safe_input = HtmlSanitizer::sanitize("<b>Clean Text</b>");
            safe_input
        }))
        .layer(RaspSecurityLayer::default())
        .layer(CspSecurityLayer::default())
        .layer(HoneypotLayer::new(state));
}
```
