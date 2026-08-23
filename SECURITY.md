# Security Policy 🛡️

## Supported Versions

Rullst adopts strict Semantic Versioning. Active security patches, CVE remediations, and vulnerability fixes are provided for the latest major release family (`v12.x`).

| Version | Supported | Status |
| :--- | :---: | :--- |
| **12.x.x** | :white_check_mark: | **Current Stable Production Release** |
| < 12.0.0 | :x: | End of Life (Deprecated) |

---

## 🚨 Reporting a Vulnerability

If you discover a potential security vulnerability within the Rullst framework, CLI tools, or runtime libraries, please **DO NOT open a public GitHub issue or pull request**.

Please send an encrypted or direct disclosure report to the Rullst Core Security Team at:
👉 **`officialrullst@gmail.com`**

### What to Include in Your Report:
1. **Vulnerability Type**: (e.g., Remote Code Execution, SQL Injection, Authentication Bypass, IDOR/BOLA, CSWSH, Memory Safety violation).
2. **Affected Crate & Version**: (e.g., `rullst-security v12.0.0`, `rullst-auth v12.0.0`, `cargo-rullst v12.0.0`).
3. **Proof of Concept (PoC)**: Minimal reproducible example or step-by-step reproduction instructions.
4. **Estimated Impact**: Criticality assessment, attack vector preconditions, and potential blast radius.

### Coordinated Vulnerability Disclosure (CVD):
* **Initial Response**: Within 24-48 hours of receipt.
* **Triage & Patch**: High and Critical vulnerabilities are patched within 72 hours and released as a patch update (`v12.x.y`).
* **Attribution**: We publicly credit security researchers in our [CHANGELOG.md](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md) and release advisories unless anonymity is requested.

---

## 🏛️ Rullst Security Architecture Matrix (v12.0.0)

Rullst is built with a **Zero-Trust Application Self-Protection** architecture embedding defensive layers directly into the compiled binary:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Rullst Zero-Trust Perimeter                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  1. Ingress Protection:   WAF Middleware + Honeypot Decoys + CSWSH Guard    │
│  2. Identity & Defense:   Anti-Bruteforce Tarpit & Login Jail (DashMap)     │
│  3. Deep Inspection:      RASP Layer (URI + Headers + Text + JNDI/RCE)      │
│  4. Data Protection:      Zeroize Vault + Field AES-256-GCM Encryption      │
│  5. Egress Defense:       HTTP Response DLP Interceptor (Private Keys/AWS)  │
│  6. Client Hardening:     OWASP Secure Headers Suite (A+ Benchmark)         │
│  7. Tamper-Proof Trail:   HMAC SHA-256 Cryptographic Audit Ledger Block     │
│  8. Threat Radar SOC:     Live Telemetry & SIEM Streamer (CEF / JSON Webhook│
└─────────────────────────────────────────────────────────────────────────────┘
```

### Core Security Engines:
* **OWASP Secure Headers Layer (`rullst-security::headers`)**: Out-of-the-box A+ score on `securityheaders.com` enforcing HSTS, CSP Nonce, Permissions-Policy, COOP, COEP, and CORP.
* **Anti-Bruteforce Login Jail (`rullst-security::login_guard`)**: Progressive async delay tarpit (0s-4s) and temporary 15-minute in-memory jail bans after 5 failed authentication attempts.
* **HTTP Response DLP Interceptor (`rullst-security::dlp`)**: Neutralizes accidental leakage of private keys, AWS access keys, and database connection strings before responses leave the server.
* **RASP Deep Request Inspector (`rullst-security::rasp`)**: Runtime protection filtering URI, text payloads, and HTTP headers against SQLi, SSRF, RCE, and Log4j exploits.
* **CLI IDOR / BOLA Static Scanner (`cargo rullst audit --idor`)**: Recursive AST scanner identifying unauthenticated entity access across parameterized routes (`/:id`, `/{id}`).
* **Automated Compliance Exporter (`cargo rullst audit --compliance`)**: Automated generation of `SECURITY_COMPLIANCE.md` reports mapping codebase controls to OWASP Top 10, SOC2 Type II, ISO 27001, and Pure-Rustls requirements.
* **CycloneDX SBOM Exporter (`cargo rullst audit --sbom`)**: Automated Software Bill of Materials generation in CycloneDX 1.5 JSON format with package SHA-256 hashes.
* **Local Network Surface Scanner (`cargo rullst audit --network`)**: High-speed port and interface binding scanner (inspired by *RustScan*) preventing sensitive leaks to `0.0.0.0`.
* **DevSecOps Git Pre-Commit Hook (`cargo rullst hook:install`)**: One-click local Git gatekeeper enforcing rustfmt, strict Clippy (`-D warnings`), and static security audits.

---

## 🧪 Continuous Security & Assurance Verification

Every commit and pull request in the Rullst ecosystem undergoes rigorous automated security verification:

| Verification Suite | Target | Tooling |
| :--- | :--- | :--- |
| **Formal Verification** | State transitions, cryptographic ledgers | **Kani Verifier (100% proofs)** |
| **Memory Safety & UB** | Strict provenance, alignment, concurrency | **Miri Interpreter (13 packages)** |
| **Dynamic Sanitizers** | Data races (`TSan`), memory leaks (`ASan`) | **Nightly ThreadSanitizer & ASan** |
| **Continuous Fuzzing** | Input parsers, macro decoders, network streams| **libFuzzer, AFL.rs, Google OSS-Fuzz** |
| **Supply Chain Audit** | Dependency CVEs, license compliance | **`cargo-audit`, `cargo-deny`, CycloneDX SBOM, SLSA 3** |
| **TLS & Cryptography** | Zero C/OpenSSL memory corruption | **100% Pure-Rustls Native (`tls-rustls`)** |
