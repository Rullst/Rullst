# Rullst audit guide

This checked-in document defines audit scope and reproduction steps. It is not a
PASS certificate and does not claim that an arbitrary branch, tag, generated
application, or deployment is secure. Results are valid only for the exact commit,
toolchain, feature set, target, and configuration recorded by the producing CI
run.

## Repository gates

The minimum source-tree verification is:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Database features that select mutually exclusive implementations also require
dedicated jobs for PostgreSQL, MySQL, and SQLite. Generated projects require their
own temporary-directory scaffold, format, check, and smoke-test harness.

The workflows in `.github/workflows/` add platform matrices, RustSec and
dependency-policy checks, CodeQL, fuzzing, sanitizer jobs, SemVer checks, SBOM
generation, package verification, and release provenance. A green badge links to
a run; it does not turn an informational workflow into a formal assurance.

## Security control inventory

| Boundary | Implemented repository control | Required verification outside this document |
| --- | --- | --- |
| Nexus | Fail-closed authentication and server-side authorization/field policy. | Deployment TLS, admin identity lifecycle, route-by-route tests, and rate-limit configuration. |
| Sessions and auth | Argon2 password hashing, versioned AES-GCM sessions, WebAuthn/MFA helpers. | Origin/RP configuration, recovery policy, key management, and application integration. |
| HTTP security | CSRF composition, webhook verification, strict headers, bounded WAF/RASP and DLP helpers. | Final middleware order, proxy trust, page CSP, protocol/content tests, and penetration testing. |
| Data access | SQLx binds, strict identifier validation, repository/ownership primitives. | Application query review and tenant/owner checks on every data route. |
| Secrets | AES-256-GCM field encryption and memory-zeroization helpers. | Key provisioning, rotation, access control, backups, and incident response. |
| Audit logs | Canonical HMAC chain and continuity verification primitives. | Durable append-only storage, protected keys, sequence retention, and independent verification. |
| Supply chain | Lockfile, advisory/policy workflows, SBOM and provenance generation. | Review exceptions, retain artifacts, and validate them against the released digest. |

## Explicit product boundaries

- NFS-e homologation and production issuance are disabled. Mock output is an
  offline fixture, never an official authorization.
- IoT currently provides `no_std` data/frame helpers and Ed25519-signed OTA
  manifest verification. MQTT transport, HSM, PQC, flashing, and bootloader
  integration remain roadmap items.
- `rullst-connect` currently focuses on OAuth2/OIDC. Kafka, RabbitMQ, and Redis
  Streams adapters in that crate remain roadmap work; queue facilities currently
  implemented elsewhere must be documented by their actual module.
- Security headers provide a strict baseline but cannot guarantee a third-party
  scanner grade for every deployed application.
- Static analysis, fuzzing, Kani, Miri, mutation testing, and DAST each cover a
  bounded scope. None alone proves absence of vulnerabilities or panics.

## Compliance status

The repository is not, by itself, SOC 2, ISO 27001, PCI DSS, or FedRAMP
certification. Those programs assess a complete system and organization, including
people, processes, infrastructure, configuration, monitoring, vendors, and
retained evidence. See `SECURITY_COMPLIANCE.md` for the control inventory.

## Evidence requirements

For a release audit, retain at least:

1. the immutable commit and tag;
2. Rust/Cargo and tool versions;
3. complete logs for blocking jobs, including skipped tests;
4. per-target and per-feature results;
5. SBOM, package checksums, signatures, and provenance attestations;
6. advisory exceptions with owner, rationale, compensating control, and expiry;
7. external contract evidence for providers or protocols claimed as live.

Any future generated audit report must report `PASS`, `FAIL`, `SKIPPED`, or
`NOT_EVALUATED` from real checks. It must never print unconditional compliance
results.
