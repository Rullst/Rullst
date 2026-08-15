# Rullst Security - Master Roadmap 🛡️⚡

`rullst-security` is the dedicated high-assurance security suite of the Rullst Framework, responsible for threat deception, sanitization, access guards, auditability, autonomous AI-driven defense, and post-quantum protection.

---

## 🎯 Phase 1: Core Protection Engine (v12.0.0) — Completed
- [x] **Rullst Honey (`rullst-honey`)**: Deception security engine deploying synthetic honeypot routes (`/.env`, `/admin.php`) and zero-latency `DashMap` memory ban tracking.
- [x] **Rullst Sanitizer (`rullst-sanitizer`)**: Deep HTML/SVG XSS sanitization via `ammonia` + per-request dynamic CSP Nonce generation (`CspSecurityLayer`) and anti-clickjacking headers.
- [x] **Rullst RBAC Guard (`rullst-rbac`)**: Declarative role authorization (`UserContext`, `RbacGuard`) preventing IDOR/BOLA attacks (`authorize_owner_or_role`).
- [x] **Rullst Audit Log (`rullst-audit-log`)**: HMAC-SHA256 chained tamper-proof cryptographic audit log (`AuditChain`) with offline record verification.
- [x] **AI Vulnerability Auditor (`cargo rullst audit --ai`)**: CLI security scanner for secret leaks in `.env`, dependency CVEs, and AI Sentinel recommendations.

---

## 🚀 Phase 2: Autonomous Intelligence & Threat Radar (v12.0.0) — Completed
- [x] **Visual Threat Radar (SOC) in Rullst Studio & Nexus (`/nexus/security`)**: Real-time visual dashboard displaying active threat attack vectors, live IP reputation scoring, blocked honeypot hits, and AI incident reports.
- [x] **AI Threat Sentinel (`rullst-security-ai`)**: Autonomous AI classifier detecting anomaly patterns (Credential Stuffing, API Scraping, Distributed Botnets) and issuing dynamic Proof-of-Work challenge tokens.
- [x] **RASP Deep Request Inspector (`rullst-security::rasp`)**: Zero-latency request and header inspector blocking SQL Injection, XSS, Path Traversal, SSRF, RCE, and JNDI/Log4j before controller execution.
- [x] **Rullst Vault (`rullst-vault`)**: Zero-trust secret management with in-memory zeroization (`Zeroize`) preventing heap dump leaks and transparent field-level AES-256-GCM / ChaCha20-Poly1305 database encryption (`#[orm(encrypted)]`).
- [x] **OWASP Secure Headers Suite (`rullst-security::headers`)**: Unified middleware layer enforcing HSTS, dynamic CSP Nonces, Permissions-Policy, COOP, COEP, and CORP granting out-of-the-box A+ rating on security audits.
- [x] **Anti-Bruteforce Tarpit & Login Jail (`rullst-security::login_guard`)**: Progressive async delay tarpit (0s to 5s) and temporary in-memory jail bans (15 min) for repeated auth failures.
- [x] **HTTP Response DLP Interceptor (`rullst-security::dlp`)**: Zero-leak response stream inspector masking private keys, AWS credentials, connection strings, and database secrets.
- [x] **Multi-Factor Authentication Engine (`rullst-security::mfa`)**: Native RFC 6238 TOTP generator, verification validator, and QR code builder for 2FA onboarding.
- [x] **Zero-Trust Client Fingerprinting (`rullst-security::zero_trust`)**: Cryptographic session binding to TLS client fingerprints (JA3/JA4) and subnet ranges with auto-invalidation on mismatch.
- [x] **Dynamic Threat Deception Traps (`rullst-security::deception`)**: Dynamic decoy routes (`/api/v1/admin/debug`, `/graphql/v1`) baiting automated scanners and pushing real-time telemetry to the SOC Threat Radar.
- [x] **Strict API Schema & Payload Inspector (`rullst-security::schema_guard`)**: Strict JSON/OpenAPI schema validation intercepting parameter pollution, buffer overflows, and JSON bomb nesting attacks.
- [x] **Real-Time Log & Secret Redaction Engine (`rullst-security::log_redactor`)**: Zero-latency log filter suppressing passwords, Authorization tokens, AWS credentials, and SSH keys prior to `tracing` stdout export.
- [x] **Cross-Site WebSocket Hijacking (CSWSH) Guard (`rullst-security::cswsh`)**: Handshake origin validation, CSRF ticket verification, and frame-level encryption for real-time WebSocket streams.
- [x] **Subresource Integrity (SRI) & Asset Signer (`rullst-security::sri`)**: Automatic SHA-384 SRI hash generation and tag injection for static JS/CSS assets preventing CDN supply chain tampering.
- [x] **SIEM & SOC Export Adapter (`rullst-security::siem`)**: Real-time alert streamer sending security events to Datadog, Splunk, Elastic, and Slack via CEF (Common Event Format) and Syslog.
- [x] **CLI IDOR / BOLA Static Audit Scanner (`cargo rullst audit --idor`)**: Static scanner analyzing parameterized routes and verifying RbacGuard / UserContext ownership enforcement.
- [x] **Automated Security Compliance Exporter (`cargo rullst audit --compliance`)**: Automated compliance auditor evaluating codebase adherence to OWASP Top 10, SOC2 Type II, HIPAA, and ISO 27001 control requirements and generating markdown reports.

---

## ⚡ Phase 3: Enterprise SaaS & Zero-Trust Deepening (v12.0.0 / v12.1.0)
- [x] **Anti-Timing Attack User Enumeration Guard (`rullst-security::timing_guard`)**: Constant-time response padding for authentication, user lookup, and password-reset endpoints, eliminating timing side-channel attacks.
- [x] **LLM Security Firewall & Prompt Injection Shield v2 (`rullst-security::ai_firewall`)**: Dedicated AI endpoint protection inspecting prompts for jailbreaks, system prompt leaking, indirect injection, and training data extraction.
- [ ] **Zero-Downtime Secret Rotation & JWKS Server (`rullst-security::key_rotation`)**: Automated cryptographic key rotation with transition grace periods and dynamic `/oauth/jwks.json` serving, meeting SOC2/PCI-DSS compliance.
- [ ] **Passkeys & WebAuthn FIDO2 Engine (`rullst-security::webauthn`)**: Native passwordless biometrics (Touch ID, Face ID, Windows Hello) and hardware token (YubiKey) authentication.
- [ ] **Multi-Tenant SaaS Data Isolation Guard (`rullst-security::tenant_guard`)**: Zero-trust multi-tenancy middleware guaranteeing database query isolation and preventing cross-tenant data leakage in multi-tenant DB schemas.
- [ ] **Hardware Security Module (HSM) & Cloud KMS Bridge (`rullst-security::kms`)**: Key signing and master key encryption abstraction supporting AWS KMS, Google Cloud KMS, HashiCorp Vault, and PKCS#11 hardware tokens.
- [ ] **Adaptive WAF Anomaly Engine (`rullst-security::adaptive_waf`)**: Dynamic request scoring pipeline calculating a real-time risk index (0–100) per IP, escalating from stealth logging to JS challenge, CAPTCHA, or TCP drop.
- [ ] **Decentralized OIDC & PKCE Enforcer (`rullst-security::oidc_guard`)**: OAuth2 Proof Key for Code Exchange (PKCE) mandatory validator with JWKS auto-rotation, signature verification, and replay protection.
- [ ] **SQL AST Query Firewall (`rullst-security::sql_firewall`)**: Static & runtime SQL Abstract Syntax Tree inspector blocking unparameterized SQL queries, stacked queries, and second-order SQL injections.
- [ ] **OAuth Scope & Claims Guard (`rullst-security::scope_guard`)**: Granular endpoint authorization verifying OAuth2 token scopes and JWT claims against dynamic endpoint requirements with least-privilege enforcement.

---

## 🔬 Phase 4: Post-Quantum Architecture & Kernel-Level Defense (Planned for v13.0.0)
- [ ] **Post-Quantum Cryptography Bridge (`rullst-security::pqc`)**: NIST ML-KEM (Kyber) & ML-DSA (Dilithium) quantum-resistant session encryption algorithms.
- [ ] **Autonomous Threat Containment & OS Firewall Pipeline (`rullst-security::containment`)**: Rule engine dispatching kernel-level iptables/eBPF IP blocks or Cloudflare API bans when RASP threat thresholds are crossed.
- [ ] **Sandboxed Wasm Plugin Engine (`rullst-security::wasm_sandbox`)**: Isolated WebAssembly execution sandbox for third-party multi-tenant SaaS extensions with strict memory/CPU limits and zero host filesystem access.
- [ ] **Post-Quantum TLS & Hybrid KEM Handshake (`rullst-security::pqc_tls`)**: Hybrid X25519 + Kyber (ML-KEM-768) key exchange integration for client-server sessions against future quantum decryption threats.
- [ ] **Cryptographic Binary SBOM & Runtime Attestation (`rullst-security::sbom`)**: In-memory executable checksum validation and `.so`/`.dylib` integrity checks against signed CycloneDX SBOM manifests.
- [ ] **In-Memory Heap Zeroization & Guard Pages (`rullst-security::mem_guard`)**: Automatic memory sanitization for key buffers on drop (`zeroize::Zeroizing`) with OS guard page protection against memory heap dump exploits.
