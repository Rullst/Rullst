# Security Policy 🛡️

## Supported Versions

`main` develops the next major release. The v12 line reached end of life on
16 October 2026: the [`v12` branch](https://github.com/Rullst/Rullst/tree/v12)
is frozen and no further 12.x release is planned. Until 13.0.0 is stable, the
latest published v13 prerelease is supported as described in
[Security support for v13 prereleases](#security-support-for-v13-prereleases).
After the 13.0.0-alpha.4 feature freeze, 13.0.0-rc.1 takes fixes and
documentation only, and 13.0.0 follows.

Rullst adopts Semantic Versioning for each published crate; crates.io remains
authoritative for whether an exact package version has been published. Source
in a branch or an unpublished tag is not a distributed release by itself. Check
the published versions on [crates.io](https://crates.io/crates/rullst/versions);
the [v12 release record](https://github.com/Rullst/Rullst/blob/v12/docs/src/v12.md)
documents the end-of-life 12.x line.

| Version | Supported | Status |
| :--- | :---: | :--- |
| **12.x** | :x: | End of life since 16 October 2026; 12.3.0 is the last release. crates.io yanked the `zip` releases its Studio needs, so new 12.x projects no longer resolve dependencies. Move to v13. |
| **Latest 13.0.0 prerelease** | :white_check_mark: | Only the most recently published prerelease (for example 13.0.0-alpha.4 once it is published). Fixes ship in a new prerelease; see below. |
| **Earlier 13.0.0 prereleases** | :x: | Not patched; upgrade to the latest prerelease. |
| **13.x (from 13.0.0)** | planned | Patch releases of the newest 13.x minor carry bug and security fixes. 13.x stays the active line for at least 12 months after 13.0.0, and receives security fixes for 12 more months after 14.0.0. See the [support window](docs/src/compatibility-policy.md#supported-release-window). |
| **12.0.0-rc.1** | :x: | Immutable evaluation prerelease of the end-of-life 12.x line; move to v13. |
| **5.0.0** | :x: | Frozen legacy release; no routine maintenance. |
| < 5.0.0 | :x: | End of life. |

Individual crates have historically used different version numbers. Before
reporting an issue, confirm the exact package and version from `Cargo.lock`.
Security fixes are issued on the current supported line rather than by
replacing an already published archive. An application-specific patch does not
update other installations of the published framework.

The last 12.x release, **12.3.0** (9 October 2026), locks the hickory DNS family
at 0.26.3 for GHSA-5j98-2g5x-46v6, GHSA-6w6g-hm98-mhgm and GHSA-6f2x-v7q7-m7m5,
stops the Core WAF and the Security RASP from refusing ordinary text, and
deprecates the Capital and Mail APIs that 13.0 removes. See the
[release review](https://github.com/Rullst/Rullst/blob/v12/docs/src/v12-3-0-review.md)
and the [upgrade notes](https://github.com/Rullst/Rullst/blob/v12/docs/src/migration-v12-1.md#upgrading-to-123).
It includes the earlier fixes, such as the 12.1.2 Nexus stored-value escaping.
The **12.1.1** security release's
[publication record](https://github.com/Rullst/Rullst/blob/v12/docs/src/v12.md#1211-published-maintenance-release)
retains its source, workflow and verified registry checksums; applications
that used public example keys must rotate those keys and renew sessions.

### Security support for v13 prereleases

Until 13.0.0 is published, this policy applies:

- **Supported version.** Only the most recently published 13.0.0 prerelease
  is supported. A prerelease stops being supported when the next one is
  published; earlier prereleases do not receive backported fixes.
- **Reporting and response.** Reports follow
  [Reporting a Vulnerability](#-reporting-a-vulnerability) and the response
  and patch targets of the
  [coordinated disclosure policy](#coordinated-vulnerability-disclosure-cvd).
- **How fixes ship.** A Critical or High vulnerability in the supported
  prerelease is fixed in a new prerelease published out of cycle, admitted
  by the same release gates as any other release. Medium and Low issues are
  fixed in the next planned prerelease. A published archive is never
  replaced.
- **Notification.** Each fix is described in `CHANGELOG.md` and, for Critical
  and High issues, in a GitHub security advisory for the affected crates.
  A prerelease with a Critical vulnerability may be yanked from crates.io
  once a fixed prerelease is available.
- **Your part.** Prereleases may change APIs between versions. Pin the exact
  version (`=13.0.0-alpha.N`, later `=13.0.0-rc.N`), follow the changelog and
  move to the newest prerelease promptly.

The support policy for 13.0.0 and later stable releases will be published
with 13.0.0.

---

## 🚨 Reporting a Vulnerability

If you discover a potential security vulnerability within the Rullst framework, CLI tools, or runtime libraries, please **DO NOT open a public GitHub issue or pull request**.

Please send a private disclosure report to the Rullst Core Security Team at:
👉 **`officialrullst@gmail.com`**

You can also use GitHub's private vulnerability reporting: the **Report a
vulnerability** button on the repository's
[Security tab](https://github.com/Rullst/Rullst/security/advisories/new).
Third-party reviewers will find the scope, threat models, sample application
and tooling in the [external audit kit](docs/src/external-audit-kit.md).

If an encrypted channel is needed, first request and verify the team's key or
agreed channel through that contact. This policy does not publish an encryption key.

### What to Include in Your Report:
1. **Vulnerability Type**: (e.g., Remote Code Execution, SQL Injection, Authentication Bypass, IDOR/BOLA, CSWSH, Memory Safety violation).
2. **Affected Crate & Version**: (e.g., `rullst-security 13.0.0-alpha.3`, `rullst-auth 13.0.0-alpha.3`, `cargo-rullst 13.0.0-alpha.3`).
3. **Proof of Concept (PoC)**: Minimal reproducible example or step-by-step reproduction instructions.
4. **Estimated Impact**: Criticality assessment, attack vector preconditions, and potential blast radius.

### Coordinated Vulnerability Disclosure (CVD):
* **Initial Response**: Critical reports within one business day and High reports within two business days.
* **Triage & Patch Target**: Critical issues within 72 hours and High issues within seven calendar days. If that target cannot be met, the affected capability must be disabled or isolated, or a reviewed, expiring exception must be recorded.
* **Attribution**: We publicly credit security researchers in our [CHANGELOG.md](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md) and release advisories unless anonymity is requested.

The complete severity, ownership, mitigation, and temporary-exception policy is
recorded in [Security advisory exceptions](docs/src/security-advisory-exceptions.md).

---

## Verifying a release

Each release tag runs [`release.yml`](.github/workflows/release.yml). It
attaches these files to the GitHub release:

- `<crate>-<version>.crate` for every published crate, byte-identical to
  crates.io, with their SHA-256 digests in `checksums.txt`;
- from v12.1.0 on, native CLI executables `rullst-<version>-<target>` and
  `cargo-rullst-<version>-<target>` (`.exe` on Windows), each target's
  `cli-manifest-<target>.json` and `cli-checksums-<target>.txt`;
- release evidence (SBOM, `cargo-audit.json`, `Cargo.lock` and others) with
  `evidence-checksums.txt`;
- from v12.1.2 on, `rullst-release.sigstore.json` (the signed Sigstore bundle)
  and `rullst-release.intoto.jsonl` (its signed DSSE envelope).

**How releases are signed.** The `attest` job creates a GitHub artifact
attestation (SLSA build provenance) for every crate archive, CLI file and
evidence file. It uses Sigstore keyless signing: the release workflow's
short-lived GitHub OIDC identity receives a certificate for a single run, and
the signature is recorded in a public transparency log. There is no long-lived
signing key, so none is stored on GitHub releases or crates.io. The job neither
checks out nor executes source code. `checksums.txt` itself is not signed: it
detects corrupted downloads, while the attestation proves origin.

Verify with a recent [GitHub CLI](docs/src/gh-install.md) (it may ask you to
run `gh auth login` first). Replace the tag and file with the ones you use:

```sh
RELEASE_TAG=v13.0.0-alpha.3
gh release download "$RELEASE_TAG" --repo Rullst/Rullst \
  --pattern "rullst-macros-${RELEASE_TAG#v}.crate" \
  --pattern checksums.txt --pattern rullst-release.sigstore.json

# 1. Integrity: the digest matches the release's checksum list.
sha256sum --check --ignore-missing checksums.txt

# 2. Origin: signed by Rullst's release workflow for this tag.
gh attestation verify "rullst-macros-${RELEASE_TAG#v}.crate" \
  --repo Rullst/Rullst \
  --signer-workflow Rullst/Rullst/.github/workflows/release.yml \
  --source-ref "refs/tags/$RELEASE_TAG" \
  --deny-self-hosted-runners
```

For a native executable, check it with `sha256sum --check --ignore-missing
cli-checksums-<target>.txt`, then run the same `gh attestation verify` command
on the executable. Every attested file since v12.0.0 can be verified online this
way.

From v12.1.2 on, the downloaded bundle can be checked without looking up
GitHub's attestation store. Pin the full commit as well, taken from a source
you trust (release tags are not signed today):

```sh
gh attestation verify "$ARTIFACT" \
  --bundle rullst-release.sigstore.json \
  --repo Rullst/Rullst \
  --cert-identity "https://github.com/Rullst/Rullst/.github/workflows/release.yml@refs/tags/$RELEASE_TAG" \
  --cert-oidc-issuer https://token.actions.githubusercontent.com \
  --source-ref "refs/tags/$RELEASE_TAG" \
  --source-digest "$RELEASE_COMMIT" \
  --signer-digest "$RELEASE_COMMIT" \
  --deny-self-hosted-runners
```

A successful check exits with status 0; changed bytes, a wrong workflow or a
wrong commit fail. On 9 October 2026 these commands verified
`rullst-macros-12.2.0.crate` against tag `v12.2.0` (commit
`565e222a3249f788a0eb560b7ecd6a2ce379ad47`) and rejected a modified copy and a
wrong commit. On 29 October 2026 they verified `rullst-macros-13.0.0-alpha.3.crate`
against tag `v13.0.0-alpha.3` (commit
`be33683252d29244ea14311790ac66decaf9d299`) and rejected a modified copy and a
wrong commit.

**Crates from crates.io.** Cargo checks every downloaded crate against the
checksum in the registry index. The publish job only succeeds when the checksum
crates.io reports equals the SHA-256 of the attested archive, so a crates.io
checksum that matches `checksums.txt` refers to the signed bytes. The archives
are also reproducible from source; see
[Reproducible crate archives](docs/src/reproducible-builds.md). Versions before
12.0.0 were published without attestations.

---

## 🏛️ Rullst Security Architecture Matrix

Rullst provides composable defense-in-depth controls for a zero-trust
application architecture. The matrix below is an implementation inventory, not
a guarantee about an application's proxy, browser, identity policy, data model,
or deployment.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Rullst Zero-Trust Perimeter                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  1. Ingress Protection:   WAF Middleware + Honeypot Decoys + CSWSH Guard    │
│  2. Identity & Defense:   Anti-Bruteforce Tarpit & Login Jail (in-memory)   │
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
* **Compliance Evidence Exporter (`cargo rullst audit --compliance`)**: Generates `SECURITY_COMPLIANCE.md` with `NO FINDINGS`, `NO FINDINGS OUTSIDE EXCEPTIONS`, `FINDINGS`, `GENERATED`, `OBSERVED`, `NOT CHECKED`, or `ERROR` observations. Control families outside the command's evidence remain `NOT EVALUATED`. These bounded observations do not confer SOC 2, ISO 27001, OWASP, or TLS certification.
* **CycloneDX SBOM Exporter (`cargo rullst audit --sbom`)**: Generates CycloneDX 1.5 JSON and includes valid SHA-256 package checksums when recorded in `Cargo.lock`. Workspace/path packages without a recorded checksum are not assigned an invented hash.
* **Local Network Surface Scanner (`cargo rullst audit --network`)**: Bounded local port/bind inspection that helps identify unintended listeners; it cannot prove the absence of network exposure outside the scanned host and target set.
* **DevSecOps Git Pre-Commit Hook (`cargo rullst hook:install`)**: Optional local gate running rustfmt, strict Clippy (`-D warnings`), and static audits. Protected CI remains authoritative because local hooks can be bypassed.

---

## 🧪 Continuous Security & Assurance Verification

The repository defines the following assurance jobs. A named workflow is
evidence only when it passed for the exact commit and declared target; no one
tool proves the whole framework secure.
The [security assurance case](docs/src/assurance-case.md) argues how the threat
models, trust boundaries, design principles and these checks fit together, and
states its limits.

| Verification Suite | Target | Tooling |
| :--- | :--- | :--- |
| **Bounded model checking** | Explicit state/ledger harnesses only | **Kani; inspect the harness list and result for the commit** |
| **Memory safety & UB** | Selected compatible targets | **Miri; unsupported dependencies/features are reported, not silently counted** |
| **Dynamic sanitizers** | Declared Linux targets | **Nightly ThreadSanitizer and AddressSanitizer jobs where configured** |
| **Fuzzing** | Named parsers and protocol inputs | **`cargo-fuzz` / libFuzzer corpora and workflows; OSS-Fuzz enrollment is not currently established** |
| **Mutation testing** | Changed lines of a pull request; source-bound full campaigns or explicitly selected files | **`mutants-pr.yml` fails a pull request to `main` with a surviving mutant on its changed lines; manual `cargo-mutants` campaigns record survivors and timeouts as findings, and incomplete artifacts cannot establish full coverage** |
| **Supply chain** | Dependency advisories, policy, SBOM, provenance | **`cargo-audit`, `cargo-deny`, CycloneDX and GitHub attestations; no SLSA level is claimed** |
| **TLS & cryptography** | Feature-specific transport inventory | **Rustls-preferred first-party paths; no universal zero-C/OpenSSL claim across all optional/transitive features** |
