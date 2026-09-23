# Rullst 12.1.1 candidate and defensive review

This is an **unpublished maintenance candidate**, based on the protected `v12`
line. It is not a release announcement or an assertion that pending checks have
passed. Version 12.1.0 remains the published baseline; `main` remains v13
development. No new v13 capability is included in this patch release.

## Changes selected for review

| Area | Maintenance change | Regression evidence to inspect |
| :--- | :--- | :--- |
| Auth | Reject every public scaffold/documentation application-key placeholder after whitespace/case normalization, including encryption, decryption and JWT entry points. Accept only exact `app_key` or legacy `key` names from `Rullst.toml`, not prefix collisions such as `key_id`. | `rullst-auth/tests/scaffold_app_key.rs`, `app_key_resolution.rs`, and existing key-validation and session tests. |
| Core | Retain lifecycle admission until the ordinary HTTP response body finishes, errors or is dropped, instead of releasing it when headers return. | `rullst-core/src/lifecycle/body/tests.rs` and `lifecycle_tests.rs`: pending frames, data, trailers, cancellation, errors, empty/full bodies and bounded drain. |
| Messaging | Sample trusted time after acquiring the SQLite write transaction. A worker cannot acknowledge an expired lease after waiting for the lock. | `rullst-messaging/tests/sqlite_deadlines.rs`: ACK/retry/dead-letter expiry, full new-claim duration, publication and retry admission timestamps. |
| Generated LMS | Keep existing controller errors and generated public signatures compatible while correcting strict-Clippy diagnostics. | Existing materialized LMS tests and generated application compilation; narrow argument-count expectations preserve existing APIs. |
| Dependencies | Include the compatible maintenance refresh already merged in PR #240, plus the SES update from #252 and immutable CI tool pins from #255. | Locked all-feature builds, SES protocol tests, advisory checks, MSRV and SemVer jobs. |
| CLI verification | Bind advisory-cache fixtures and both CLI entry-point version assertions to the actual package version. | `cargo-rullst/tests/update_discovery_cli.rs`: private cache reuse, invalid/expired/yanked rejection, no writes or installation authority and executable versions. |
| Verification | Release canceled observational scorecards promptly; fetch the full locked graph before offline archive inspection; permit an archive-only diagnostic. | Workflow lint plus the actual archive-consumer run. A diagnostic never substitutes for the full PR matrix or native-artifact evidence. |

Selected upstream sources are Auth `7fa83fbf` (prepared v12 patch `3891999e`),
Core `4dfac0ce` (only the existing lifecycle correction), Messaging `2629d349`,
generated LMS `92c8aa06` and CLI test fixtures `d97f3511` (tests only). The Core
deployment example and unrelated v13 features are deliberately outside this backport. The Redis Streams TLS and
new Android artifact-verification fixes depend on v13-only implementations and
are not imported into the v12 API surface.

All sixteen publication packages and their internal version requirements are
12.1.1. Workspace and ten fuzz-package locks must resolve without modification.
The CLI reports its package version through `CARGO_PKG_VERSION`; historical
12.1.0 artifact fixtures and publication receipts retain their original identity.
Executable discovery fixtures derive their eligible target from the current
package version so a version bump cannot turn a cache check into a downgrade.

## Compatibility and application action

The intended contract is a compatible patch over 12.1.0 with Rust **1.96.0** as
MSRV. The development compiler remains 1.98.1. There are no public API removals,
new database schemas, provider-account migrations or automatic application edits.
Dependency metadata alone does not prove the MSRV; the actual compiler job must
pass. SemVer tooling has explicit procedural-macro/binary limitations, covered
separately by compilation and CLI consumer tests.

Applications using a published example key must replace it with their own
securely generated secret. Validation now fails closed for those placeholders.
Changing the key invalidates encrypted sessions made with the previous key;
plan session renewal and application-specific recovery. A package update does
not rotate a deployed secret.

Lifecycle-aware applications may now wait longer during drain because a returned
HTTP response can still own a streaming body. The wait remains bounded. Client
receipt, upgraded connections and detached jobs require application supervision.
SQLite delivery remains at least once; host clock rollback policy is unchanged.
Generated LMS files already copied into applications require a reviewed source
update if their stricter lint behavior is wanted.

## Acceptance evidence

The review handoff must bind the candidate SHA, protected merge SHA/tree and
each workflow URL/result. Historical 12.1.0 or v13 successes cannot certify this
candidate. The final report is produced after the relevant runs finish; this
document specifies what must be checked rather than inventing their outcome.

- Complete all-feature unit, integration and documentation tests, including the
  generated application shards on Linux, macOS and Windows.
- Strict workspace/all-target Clippy, format integrity, actual Rust 1.96.0 MSRV
  compilation, SemVer and every required protected-branch check.
- Coverage floors, architecture/feature boundaries, dependency/security scans,
  CodeQL results, secret scanning and repository workflow-policy checks.
- Real package archives for the sixteen packages, normalized dependency and
  version inspection, an archive-only consumer, isolated installed CLI and
  version-bound native artifacts; none of this publishes packages.
- The required bounded Miri and Kani scopes and all 40 fuzz targets. Fuzz reuse
  requires the existing input-equivalence policy and independent verification;
  changed inputs are rerun. Diagnostic subsets are not release evidence.
- Any manual gate required by `.github/release-required-workflows.json`, with
  no weakened assertion, lowered coverage floor or skipped failure.

Use hosted runners for the large matrices. Local work must preserve the AGENTS
disk reserve. Provider fixtures use deterministic mocks or isolated local
protocol servers; no real customer, payment, mail or cloud account is tested.
Informational mutation results are reported as findings, not a security proof.

## Daybreak Blue review procedure

Changing model is a separate user action after this candidate is ready. Prepare
an isolated checkout at the recorded SHA, with no production secrets or live
account access. Begin with `AGENTS.md`, `docs/src/spec.md`, `SECURITY.md`, the
12.1.0-to-candidate diff and the attached evidence report.

1. Review the application-key policy at every entry point, normalization,
   error paths and existing session compatibility. Add a failing regression
   before any corrective patch.
2. Review lifecycle guard ownership across body polling, error, cancellation,
   stream termination and drain races. Check timeout and upgrade boundaries.
3. Review SQLite transaction admission, lease timestamps, rollback/cancellation,
   stale workers and retry/dead-letter state transitions under contention.
4. Inspect dependency and workflow deltas for changed trust boundaries, exact
   pins, MSRV compatibility and false-positive assurance claims. Check package
   contents, internal version constraints and CLI update/install boundaries.
5. Expand into applicable existing Auth, tenant/authorization, webhook,
   generated-SaaS/LMS and installer boundaries where the diff or evidence gives
   a concrete reason. Use local reproductions and mock providers only.

For each finding, record severity, affected code and version, reproducible local
evidence, compatibility implications, proposed fix and required regression
checks. Follow private coordinated disclosure for security findings. A review
with another model is supplementary analysis, not independent certification.

After review, validated corrections may update the candidate. A new source SHA
requires corresponding evidence before protected integration. Tags, immutable
artifacts, registry publication and deployment remain outside this preparation
goal and require a separate release decision.
