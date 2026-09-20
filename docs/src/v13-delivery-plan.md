# v13 delivery plan through 26 September 2026

**Engineering target: a reviewable, tested v13 release by 26 September 2026,
America/Sao_Paulo.** This is a bounded delivery plan, not a claim that the entire
historical roadmap fits the remaining week. The [SST](spec.md) governs APIs and
architecture; the [roadmap](roadmap.md) retains work outside this release window.
No deadline waives a security or publication gate.

## Starting evidence

- All sixteen v12.1.0 packages are published from `b62390b4`; the
  [release record](v12.md#1210-published-maintenance-release) retains their receipt.
- The integration candidate carries that exact published runtime source into
  v13, preserving the unpublished privacy crate and Labs/Verus plans. The
  combined source needs fresh CI; the v12.1.0 results do not certify v13.
- `rullst-privacy` currently provides policy, challenge, signed-attestation and
  replay interfaces. It has no production replay backend or live age provider.
  Its 16 integration tests and one documentation test passed during integration;
  this is foundation evidence, not end-to-end production acceptance.
- Release admission currently requires `main`; most security workflows still
  filter only that branch. The GitHub `v13` branch was not protected at this
  review. These are explicit preparation tasks before any v13 tag.
- The v12.1.0 post-release mutation campaign is informational. Its repaired
  controller discovers the measured source's complete inventory and limits the
  80 shards to four concurrent jobs. Findings are reviewed separately from v13
  readiness, with security-relevant corrections carried to each affected line.

## Release priorities and acceptance

| Priority | Deliverable | Acceptance before calling it complete |
| :--- | :--- | :--- |
| P0 | Published v12.1.0 corrections integrated without losing v13 work | Review conflicts; retain the stable runtime changes; pass the combined workspace tests, strict Clippy, format and feature/consumer checks. |
| P0 | Protected v13 verification and publication path | Explicit major-version/branch policy, protection equivalent to the stable line, complete required workflow/job inventory, negative wrong-branch/tag/evidence tests and an unpublished package rehearsal. Keep v12 maintenance independently releasable. |
| P0 | Usable proportional age-assurance boundary | Durable atomic replay protection, authenticated tenant/session/action binding, explicit method strength, fail-closed outage/expiry/replay behavior and a bounded application integration. Production must reject mocks and process-local state. |
| P0 | First complete privacy journey in a generated consumer | Explicit purpose/version choices, withdrawal enforced on subsequent optional processing, one authorized rights workflow with actual adapter effects, tenant isolation and documented retention/restore boundaries. Do not count a request row as completed export or erasure. |
| P0 | Safe 12.1→13 adoption | Inventory actual compatibility changes, implement only supported migration rules, run generated SaaS/LMS consumer fixtures and prove review, stale-input rejection and recovery. A major-version flag alone is not a migration. |
| P1 | Better application context for maintainers and assistants | Accurate generated `AGENTS.md`, a bounded deterministic project map, configuration key names without values, explicit file/secret exclusions, freshness information and executable regression fixtures. |
| P1 | Focused Verus pilot | One production age-policy property with checked linkage, pinned tooling, negative controls and measured cost. No framework-wide verification claim or automatic expansion to every crate. |

P0 items have precedence over new integrations, cosmetic rewrites and expanding
the number of crates. Each implementation should remain a small reviewable
change with its own tests and migration/documentation updates. The acceptance
column is a requirement, not a description of code already present.

## Age and privacy design decisions to close first

Keep age assurance and reusable privacy contracts in the existing optional
`rullst-privacy` package. Do not add a second age crate or make cameras, a vision
runtime, a database or a provider a default Core dependency.

The present synchronous `ReplayStore::claim` interface must be reviewed before
adding asynchronous database adapters. Prefer a static-dispatch asynchronous
boundary over blocking the HTTP runtime. The first persistence work must test
independent verifier instances, restart, simultaneous claims, capacity, expiry,
cancelled/uncertain writes and unavailable storage. SQLite's shared-local
boundary must be distinct from PostgreSQL multi-instance operation; neither
may claim that restoring an old backup preserves replay protection without a
reviewed restore/key-epoch procedure.

Use the current server-owned policy and signed threshold result rather than
storing photos or full birth dates in the framework. Provider authenticity,
subject/session binding and the quality of an age determination are separate
contracts. A facial estimate cannot silently become a verified age attribute.
Every denied, inconclusive or unavailable result needs an explicit product
outcome; an alternative path must meet the same policy instead of bypassing it.

A live adapter requires an available authorized sandbox, a reviewed native
protocol, failure tests and appropriate provider/capture evidence. The release
owner confirmed that no provider environment is available and directed work on
the foundation and local tests first. Prioritize that independently testable
scope while the external dependency is unresolved. If it remains
unavailable, do not label a simulated flow as live verification or mark the
provider milestone complete. Reassess the publishable privacy package boundary
explicitly at the feature freeze; do not quietly weaken its existing admission
criteria to meet the date.

For broader privacy, implement reusable processing controls and one real
consumer journey before claiming coverage across every blueprint. Keep policy
applicability, notices, processor obligations and external legal review explicit.
The [privacy roadmap](privacy-age-assurance-roadmap.md) remains authoritative
for the wider jurisdiction and provider programme; this plan does not certify
GDPR, LGPD or worldwide compliance.

## Improvements that reduce maintenance friction

1. **A current source baseline.** Synchronize stable fixes before feature work;
   avoid repeatedly rediscovering or reintroducing already repaired SaaS, mail
   and updater problems.
2. **One executable test map.** Preserve complete test inventories and choose
   focused development checks from actual crate/dependent changes. Unknown,
   security, generator, dependency and workflow changes retain conservative
   coverage. Measure job execution separately from runner queue time.
3. **Accurate generated guidance.** Derive agent instructions and context from
   the chosen blueprint/features. Link to version-matched public APIs and
   commands; do not include credentials or imply that optional adapters are active.
4. **Small complete examples.** Use reproducible SaaS/LMS reference journeys
   with negative ownership, replay, withdrawal and recovery assertions. A
   compiling tutorial alone does not establish those behaviors.
5. **A durable progress record.** Record the source, checks, limitations and next
   action for each delivered item. Avoid duplicated readiness statements that
   drift apart across the README, roadmap, spec and release guide.

These implement the narrow first phase of the
[AI-maintainability roadmap](ai-maintainability-roadmap.md). Broad model
benchmarks and quality guarantees about arbitrary assistants are outside this
release window. Keep changes modular; avoid unrelated mass refactoring merely
to meet a line-count target.

## Calendar and scope control

Dates below use Brasília time. Work may move earlier when its dependencies and
checks are ready; slow external acceptance reduces scope, never test quality.

| Date | Checkpoint |
| :--- | :--- |
| 20 September | Close v12.1 documentation, admit the integration baseline, settle privacy/storage API decisions and prepare the v13 release/branch policy. |
| 21–22 September | Implement and test the prioritized age/privacy consumer journey and migration boundaries. Establish any live provider environment by the end of the 22nd. |
| 23 September | Complete bounded maintainer tooling and documentation; evaluate the small Verus pilot only after P0 work. Freeze feature scope by the evening. |
| 24 September | Run the complete candidate verification campaign, including required native matrices, fuzzing, Miri, Kani and sanitizers. Preserve explicit evidence boundaries and start long jobs early. |
| 25 September | Repair findings, invalidate and repeat affected evidence, test packaged consumers and rehearse the complete publication transaction. |
| 26 September | Buffer for final failures, review and protected publication. Publish stable only with all mandatory criteria satisfied; otherwise report the exact blockers and an honestly scoped candidate. |

At the feature freeze, move unfinished optional work to a named follow-up rather
than leaving partially enabled behavior in a stable release. A blocking age,
authorization, data-loss or migration defect is not an optional follow-up.
Keep approved feature scope in the release notes and tests; do not mark the
historical roadmap complete merely because the version is published.

## Verification and final publication

Run `cargo test --workspace --all-features`, strict workspace/all-feature
Clippy and formatting, plus the existing feature, threat, provider-protocol,
generated-application, native-platform and release gates. New privacy parsers,
state transitions and durable adapters need their own meaningful negative,
concurrency and fuzz/property coverage. Local focused passes never substitute
for the final workspace campaign.

Use hosted CI for builds whose peak cannot fit the workstation's reserve;
retain at least 12 GiB available and intervene at 15 GiB. Reuse verification
only through the existing explicit input-equivalence/admission policy. Changes
to a candidate invalidate the affected evidence; neither a green older commit
nor cached compilation is a release result.

Before tagging, check every package/internal requirement, lockfile, release
order, architecture edge, feature forwarding, CLI version string, documentation
link and supported-platform claim. Include `rullst-privacy` in publication only
after its advertised package scope meets its gates and its ownership/trusted
publisher configuration is ready. Confirm 12.1→13 consumer behavior, authenticated
native assets, attestations and the protected deployment approval. Published
12.1.0 tags and archives remain immutable.

Execution proceeds within the active authorized session, while GitHub jobs can
continue independently. Session availability and a running CI job are different
things; this plan does not promise an unattended uninterrupted week. At each
handoff, retain the next action and distinguish work still possible from work
actually waiting on an external result.
