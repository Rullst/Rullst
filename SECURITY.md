# Security Policy 🛡️

## Supported Versions

Rullst adopts Semantic Versioning for each published crate. The repository is
currently preparing the v12 release candidate; a version present in the
workspace is not a published stable release by itself.

| Version | Supported | Status |
| :--- | :---: | :--- |
| **5.0.0** | :white_check_mark: | Latest published `rullst` umbrella release on crates.io as checked on 2026-08-26; receives security triage while v12 is prepared. |
| **12.0.0 source** | :construction: | Unreleased RC candidate; receives fixes but is not yet a production release. |
| < 5.0.0 | :x: | End of life. |

Individual crates have historically used different version numbers. Before
reporting an issue, confirm the exact package and version from `Cargo.lock`.
This table must be updated as part of the v12 RC publication.

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
* **Initial Response**: Critical reports within one business day and High reports within two business days.
* **Triage & Patch Target**: Critical issues within 72 hours and High issues within seven calendar days. If that target cannot be met, the affected capability must be disabled or isolated, or a reviewed, expiring exception must be recorded.
* **Attribution**: We publicly credit security researchers in our [CHANGELOG.md](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md) and release advisories unless anonymity is requested.

The complete severity, ownership, mitigation, and temporary-exception policy is
recorded in [Security advisory exceptions](docs/src/security-advisory-exceptions.md).

---

## 🏛️ Rullst Security Architecture Matrix (v12.0.0)

Rullst provides composable defense-in-depth controls for a zero-trust
application architecture. The matrix below is an implementation inventory, not
a guarantee about an application's proxy, browser, identity policy, data model,
or deployment.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Rullst Zero-Trust Perimeter                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  1. Ingress Protection:   WAF Middleware + Honeypot Decoys + CSWSH Guard    │
│  2. Identity & Defense:   Anti-Bruteforce Tarpit & Login Jail (DashMap)     │
│  3. Deep Inspection:      RASP Layer (URI + Headers + Text + JNDI/RCE)      │
│  4. Data Protection:      Zeroize Vault + Field AES-256-GCM Encryption      │
│  5. Egress Defense:       HTTP Response DLP Interceptor (Private Keys/AWS)  │
│  6. Client Hardening:     Strict CSP and secure-header baseline             │
│  7. Tamper Evidence:      Local HMAC SHA-256 chained audit records          │
│  8. Threat Radar SOC:     Live Telemetry & SIEM Streamer (CEF / JSON Webhook│
└─────────────────────────────────────────────────────────────────────────────┘
```

### Core Security Engines:
* **OWASP Secure Headers Layer (`rullst-security::headers`)**: Enforces a tested HSTS, CSP nonce, Permissions-Policy, COOP, COEP, and CORP baseline. Scanner grades depend on the final application, cookies, proxy, TLS, and rendered content; an A+ grade is not guaranteed.
* **Anti-Bruteforce Login Jail (`rullst-security::login_guard`)**: Progressive async delay tarpit (0s-4s) and temporary 15-minute in-memory jail bans after 5 failed authentication attempts.
* **HTTP Response DLP Interceptor (`rullst-security::dlp`)**: Detects and redacts a bounded set of private-key, AWS-key, and database-URL patterns. It reduces accidental disclosure risk but cannot guarantee zero leakage.
* **RASP Request Inspector (`rullst-security::rasp`)**: Bounded heuristic inspection of supported URI, header, textual, and JSON inputs for selected SQLi, traversal, SSRF, RCE, and JNDI signatures. It does not replace typed parsing, parameterized SQL, authorization, or egress allowlists.
* **CLI IDOR / BOLA Static Scanner (`cargo rullst audit --idor`)**: Heuristic source scanner that flags parameterized routes lacking recognized ownership or role guards. Findings require review, and absence of a finding is not proof of authorization.
* **Compliance Evidence Exporter (`cargo rullst audit --compliance`)**: Generates `SECURITY_COMPLIANCE.md` with explicit `PASS`, `FAIL`, `SKIPPED`, and `NOT_EVALUATED` results. It maps evidence to controls but does not confer SOC 2, ISO 27001, OWASP, or TLS certification.
* **CycloneDX SBOM Exporter (`cargo rullst audit --sbom`)**: Automated Software Bill of Materials generation in CycloneDX 1.5 JSON format with package SHA-256 hashes.
* **Local Network Surface Scanner (`cargo rullst audit --network`)**: Bounded local port/bind inspection that helps identify unintended listeners; it cannot prove the absence of network exposure outside the scanned host and target set.
* **DevSecOps Git Pre-Commit Hook (`cargo rullst hook:install`)**: Optional local gate running rustfmt, strict Clippy (`-D warnings`), and static audits. Protected CI remains authoritative because local hooks can be bypassed.

---

## 🧪 Continuous Security & Assurance Verification

The repository defines the following assurance jobs. A named workflow is
evidence only when it passed for the exact commit and declared target; no one
tool proves the whole framework secure.

| Verification Suite | Target | Tooling |
| :--- | :--- | :--- |
| **Bounded model checking** | Explicit state/ledger harnesses only | **Kani; inspect the harness list and result for the commit** |
| **Memory safety & UB** | Selected compatible targets | **Miri; unsupported dependencies/features are reported, not silently counted** |
| **Dynamic sanitizers** | Declared Linux targets | **Nightly ThreadSanitizer and AddressSanitizer jobs where configured** |
| **Fuzzing** | Named parsers and protocol inputs | **libFuzzer/AFL corpora and workflows; OSS-Fuzz enrollment is not currently established** |
| **Supply chain** | Dependency advisories, policy, SBOM, provenance | **`cargo-audit`, `cargo-deny`, CycloneDX and GitHub attestations; no SLSA level is claimed** |
| **TLS & cryptography** | Feature-specific transport inventory | **Rustls-preferred first-party paths; no universal zero-C/OpenSSL claim across all optional/transitive features** |
