# Rullst security control inventory

This document is a source-tree control inventory, not a certification, audit
opinion, penetration-test report, or generated compliance proof. Status refers
to the current repository revision; applications built with Rullst must still
validate their own configuration, deployment, integrations, and threat model.

| Area | Framework status | Application or operator responsibility |
| --- | --- | --- |
| Access control | RBAC and ownership-check primitives are available. | Apply them to every protected route and test authorization boundaries. |
| Secrets at rest | The vault uses authenticated AES-256-GCM encryption. | Provision, rotate, restrict, and back up encryption keys securely. |
| Injection defenses | SQLx bind parameters, identifier validation, and bounded WAF heuristics are available. | Avoid dynamic SQL, validate domain input, and perform application security testing. |
| HTTP security headers | A strict nonce-based baseline is available. | Tune CSP for the application and verify the deployed response; no external grade is guaranteed. |
| Authentication | Argon2 password hashing, encrypted sessions, WebAuthn, MFA, and authorization helpers are available. | Configure origins, relying-party data, session policy, recovery, and account lifecycle controls. |
| Memory safety | Production code is expected to follow the reviewed unsafe allowlist and zero-panic policy. | Keep CI policy checks enabled and review any new `unsafe` usage. |
| Transport security | Rustls-backed clients and servers are used on selected framework paths. | Configure certificates, TLS termination, proxies, and service-to-service trust. |
| Audit integrity | A tamper-evident audit-chain primitive is available. | Persist records durably, protect signing material, verify continuity, and export evidence. |
| Supply chain | CI can generate an SBOM, provenance attestations, and dependency reports. | Retain the generated artifacts and review findings for every release. |
| SOC 2, ISO 27001, PCI DSS, FedRAMP | Rullst is not certified by this repository. | Certification and compliance depend on the complete organization, system, processes, and evidence. |

## Reproducing repository checks

Run the workspace verification commands from the repository root:

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --all -- --check
```

Release and security evidence is produced by the workflows under
`.github/workflows/`; a checked-in snapshot must not be treated as proof that a
particular revision passed those workflows.
