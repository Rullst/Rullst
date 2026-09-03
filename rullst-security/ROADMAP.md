# Rullst Security - Master Roadmap 🛡️⚡

> **Status policy (2026-08-26):** the defensive ideas below are preserved, but
> legacy `[x]` markers do not establish absolute claims such as zero leakage,
> universal OWASP coverage, certification, or live external intelligence. See
> the audited [`rullst-security` row](../ROADMAP.md#audit-of-the-detailed-crate-roadmaps)
> and the [capability ledger](../docs/src/capability-ledger.md).

`rullst-security` is the dedicated defense-in-depth suite of the Rullst Framework. It currently provides bounded deception, sanitization, access guards, audit primitives and runtime middleware; autonomous threat response and post-quantum protection remain future research.

---

## 🎯 Phase 1: Core Protection Engine (v12.0.0) — Audited
- [x] **Rullst Honey (`rullst-honey`)**: Exact synthetic honeypot routes (`/.env`, `/admin.php`) with bounded, expiring `DashMap` state keyed from verified socket peers.
- [x] **Rullst Sanitizer (`rullst-sanitizer`)**: Bounded HTML sanitization via `ammonia`, per-request CSP nonce generation (`CspSecurityLayer`) and anti-clickjacking headers.
- [x] **Rullst RBAC Guard (`rullst-rbac`)**: Declarative role authorization (`UserContext`, `RbacGuard`) preventing IDOR/BOLA attacks (`authorize_owner_or_role`).
- [x] **Rullst Audit Log (`rullst-audit-log`)**: Versioned HMAC-SHA256 tamper-evident chain (`AuditChain`) with bounded record and sequence verification plus an explicit sink contract.
- [~] **Security Auditor (`cargo rullst audit --ai`)**: Bounded `.env`, dependency, unsafe-syntax and route heuristics with deterministic remediation suggestions. The legacy flag does not invoke an LLM or replace a security review.

---

## 🚀 Phase 2: Autonomous Intelligence & Threat Radar (v12.0.0) — Audited
- [~] **Visual Threat Radar in Rullst Studio & Nexus (`/nexus/security`)**: Bounded process-local counters and recent security events with explicit unavailable states; no external IP reputation, threat feed or SOC operation is implied.
  - **Escopo auditado na v12:** Studio e Nexus exibem contadores e os eventos
    recentes, limitados e locais do processo. Reputação externa de IP, feed de
    inteligência e operação de um SOC não foram implementados.
  - **Ambição restante:** vale conectar fontes versionadas e mostrar saúde,
    atraso e proveniência de cada feed; a interface nunca deve inventar dados.
- [~] **Threat Sentinel (`rullst-security::sentinel`)**: A deterministic classifier now recognizes transparent credential-stuffing, API-scraping and distributed-automation thresholds over trusted caller-supplied aggregates, and an opt-in HMAC-authenticated, subject-bound, expiring, process-local one-shot Proof-of-Work gate is tested under concurrency. It is not AI, botnet attribution, traffic collection, autonomous blocking, distributed replay state or a DDoS guarantee.
- [~] **RASP Deep Request Inspector (`rullst-security::rasp`)**: Bounded request/header/body heuristics block recognized SQLi, XSS, traversal, SSRF, RCE and JNDI patterns before the handler; this is not a complete parser or zero-latency guarantee.
- [~] **Rullst Vault (`rullst-vault`)**: `VaultSecret` reduces secret lifetime and `FieldEncryptor` supplies authenticated AES-256-GCM envelopes with AAD and rotation. It cannot prevent process-memory capture; ChaCha20-Poly1305 is not implemented.
- [x] **Secure Headers Suite (`rullst-security::headers`)**: Unified middleware applies HSTS, per-request CSP nonces, Permissions-Policy, COOP, COEP and CORP. Scanner grades still depend on the complete deployed application.
- [x] **Anti-Bruteforce Tarpit & Login Jail (`rullst-security::login_guard`)**: Bounded progressive delays and temporary in-memory jail bans, including `record_login_failure_and_wait` so handlers can apply the delay directly.
- [~] **HTTP Response DLP Interceptor (`rullst-security::dlp`)**: Bounded textual response inspection masks recognized private keys, AWS credentials and database URLs; it is defense in depth, not a zero-leak contract.
- [x] **Multi-Factor Authentication Engine (`rullst-security::mfa`)**: OS-random 160-bit secrets, RFC 6238 TOTP generation/constant-time verification, `otpauth://` enrollment and a real bounded SVG QR builder.
- [~] **Zero-Trust Client Fingerprinting (`rullst-security::zero_trust`)**: HMAC-bound normalized subnet and caller-supplied client observations with strong-key validation. TLS/JA3/JA4 acquisition and session invalidation remain host responsibilities.
- [x] **Dynamic Threat Deception Traps (`rullst-security::deception`)**: Bounded, strictly validated exact decoy routes emit process-local telemetry when triggered.
- [x] **Bounded API Schema & Payload Inspector (`rullst-security::schema_guard`)**: Exact JSON content-type, syntax, recursive duplicate-key, size and depth checks compose with a reusable JSON Schema 2020-12 policy or one explicit OpenAPI 3.1 component. Construction bounds document size/nodes/depth, rejects external references, disables network/filesystem resolution and selects linear-time regexes. Generic query/header/form parameter schemas and application authorization remain separate.
- [~] **Log & Secret Redaction Engine (`rullst-security::log_redactor`)**: Repeated Bearer/assignment, PEM, AWS and database patterns are bounded and tested. The application must invoke it before `tracing`; no global interception or zero-leak guarantee is implied.
- [~] **Cross-Site WebSocket Hijacking (CSWSH) Guard (`rullst-security::cswsh`)**: Exact Origin/Host or explicit allowlist validation is implemented. CSRF tickets and application-level frame encryption are not.
- [x] **Subresource Integrity (SRI) & Asset Signer (`rullst-security::sri`)**: SHA-384 hashes and escaped script/link tags can be generated from bytes or bounded local JS/CSS files; asset discovery/injection remains explicit.
- [~] **SIEM evidence boundary (`rullst-security::siem`)**: CEF escaping,
  process-local recording, a compatible unsigned spool and an opt-in
  HMAC-chained single-process journal with explicit key rotation exist; no
  Datadog, Splunk, Elastic, Slack or Syslog delivery is implemented.
  - **Escopo auditado na v12:** `DurableSiemSpool` writes normalized unsigned v1
    events behind an in-process lock, exact byte/record quotas, a versioned
    length/digest frame and `sync_data`. `AuthenticatedSiemSpool` separately
    chains sequence/predecessor/payload with HMAC-SHA256 and supports one active
    plus seven historical zeroized keys. Restart, quota, symlink, forgery,
    wrong/missing key, interior deletion/reordering and external-length-change
    tests fail closed. Whole-tail rollback still needs a separately trusted
    checkpoint; the name `dispatch_siem_alert` does not mean external delivery.
  - **Ambição restante:** file compaction/rotation, key-retirement tooling,
    retention and a delivery contract with retry, backoff, dead-letter,
    redaction, backpressure, health and acknowledgement, followed by provider
    contract suites.
- [~] **CLI IDOR / BOLA Static Audit Scanner (`cargo rullst audit --idor`)**: Fail-closed bounded source heuristic for parameterized-route classifications and recognized guards; it does not prove handler/domain authorization.
- [~] **Cargo Geiger Memory Safety Scanner (`cargo rullst audit --geiger`)**: Bounded first-party unsafe-syntax scan plus an explicitly requested `cargo-geiger` run whose absence/failure is fatal. Dependency unsafe does not become a universal safety proof.
- [~] **Parser Fuzz Testing Suites (`rullst-security/tests/fuzz_robustness.rs` & `fuzz/`)**: Deterministic robustness loops and libFuzzer targets exist; only executed corpora and paths are evidence, never universal panic-freedom.
- [~] **Security Evidence Exporter (`cargo rullst audit --compliance`)**: Generates a report of checks actually executed and explicitly marks unassessed controls. It is not OWASP, SOC 2, HIPAA or ISO certification.

---

## ⚡ Phase 3: Enterprise SaaS & Zero-Trust Deepening (v12 hardening / v13)
- [~] **Anti-Timing User Enumeration Guard (`rullst-security::timing_guard`)**: Minimum-duration padding, jitter and optional synthetic work reduce coarse timing differences; scheduler, network and statistical side channels are not eliminated.
- [~] **LLM Security Firewall & Prompt Injection Shield (`rullst-security::ai_firewall`)**: Bounded recursive JSON prompt inspection recognizes documented jailbreak/exfiltration patterns and fails closed for malformed declared JSON. It is a heuristic layer, not an LLM safety guarantee.
- [x] **Automated SBOM Exporter (`cargo rullst audit --sbom`)**: Parses Cargo metadata into CycloneDX 1.5 JSON with a valid UUID serial, unique component references, package URLs and valid available checksums.
- [~] **Network Surface & Port Binding Scanner (`cargo rullst audit --network`)**: Bounded loopback probes plus source, `.env` and available `ss` listener inspection flag unspecified bindings; deployment/firewall reachability still requires external evidence.
- [~] **High-Contention Concurrency Suite (`rullst-security/tests/concurrency_tests.rs`)**: Multi-threaded stress tests exercise the named local stores. They are regression evidence, not a proof that every interleaving or external backend is race-free.
- [~] **DevSecOps Git Hook Engine (`cargo rullst hook:install`)**: Safely preserves/chains hooks and runs format, all-feature Clippy and bounded unsafe/IDOR checks. CI and review remain authoritative.
- [x] **Framework Toolchain & Security Doctor (`cargo rullst doctor`)**: Diagnostics parse/enforce the declared Rust 1.96 MSRV and check linters, cargo-audit, cargo-geiger, cargo-deny, Kani and supporting tools; autofix reports success only after verification.
- [~] **Rustls-first network stack (`tls-rustls`)**: First-party HTTP/SQLx paths prefer Rustls, but a universal zero-OpenSSL/zero-FFI guarantee across every optional transitive dependency is not promised.
- [ ] **Supply-Chain Dependency Attestation (`cargo-vet` & `cargo-deny`)**: Cryptographic dependency verification and viral license prevention.
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
