# Rullst Security - Roadmap 🛡️

`rullst-security` is the dedicated high-assurance security suite of the Rullst Framework, responsible for threat deception, sanitization, access guards, auditability, autonomous AI-driven defense, and post-quantum protection.

---

## 🎯 Phase 1: Core Protection Engine (v12.0.0) — Completed
- [x] **Rullst Honey (`rullst-honey`)**: Deception security engine deploying synthetic honeypot routes (`/.env`, `/admin.php`) and zero-latency `DashMap` memory ban tracking.
- [x] **Rullst Sanitizer (`rullst-sanitizer`)**: Deep HTML/SVG XSS sanitization via `ammonia` + per-request dynamic CSP Nonce generation (`CspSecurityLayer`) and anti-clickjacking headers.
- [x] **Rullst RBAC Guard (`rullst-rbac`)**: Declarative role authorization (`UserContext`, `RbacGuard`) preventing IDOR/BOLA attacks (`authorize_owner_or_role`).
- [x] **Rullst Audit Log (`rullst-audit-log`)**: HMAC-SHA256 chained tamper-proof cryptographic audit log (`AuditChain`) with offline record verification.
- [x] **AI Vulnerability Auditor (`cargo rullst audit --ai`)**: CLI security scanner for secret leaks in `.env`, dependency CVEs, and AI Sentinel recommendations.

---

## 🚀 Phase 2: Autonomous Intelligence & Threat Radar — Completed
- [x] **Visual Threat Radar (SOC) in Rullst Studio & Nexus (`/nexus/security`)**: Real-time visual dashboard displaying active threat attack vectors, live IP reputation scoring, blocked honeypot hits, and AI incident reports.
- [x] **AI Threat Sentinel (`rullst-security-ai`)**: Autonomous AI classifier detecting anomaly patterns (Credential Stuffing, API Scraping, Distributed Botnets) and issuing dynamic Proof-of-Work challenge tokens.
- [x] **RASP Engine (Runtime Application Self-Protection)**: Zero-latency request inspector blocking SQL Injection, XSS, Path Traversal, and SSRF before controller execution.
- [x] **Rullst Vault (`rullst-vault`)**: Zero-trust secret management with in-memory zeroization (`Zeroize`) preventing heap dump leaks and transparent field-level AES-256-GCM / ChaCha20-Poly1305 database encryption (`#[orm(encrypted)]`).

---

## 🔬 Phase 3: Post-Quantum & Deep Security (Upcoming)
- [ ] **Multi-Factor Authentication Engine (`rullst-security::mfa`)**: Native RFC 6238 TOTP (Time-based One-Time Password) generator, verification validator, and QR code builder for 2FA onboarding.
- [ ] **Real-Time Secret Guard (`rullst-security::secret_guard`)**: Zero-latency log and HTTP response interceptor preventing accidental leaks of JWT secrets, API keys, or private SSH keys.
- [ ] **Hardware Security Module (HSM) & KMS Integration**: Hardware token signing and Key Management System integration (AWS KMS, HashiCorp Vault).
- [ ] **Post-Quantum Cryptography Bridge (`rullst-security::pqc`)**: NIST ML-KEM (Kyber) & ML-DSA (Dilithium) quantum-resistant session encryption algorithms.
- [ ] **SLSA Binary Attestation & Cryptographic SBOM**: In-memory executable checksum validation preventing binary tampering and supply chain attacks in production.
