# v13 delivery plan through 26 September 2026

**Engineering target: a reviewable, tested v13 release by 26 September 2026,
America/Sao_Paulo.** This is a bounded delivery plan, not a claim that the entire
historical roadmap fits the remaining week. The [SST](spec.md) governs APIs and
architecture; the [roadmap](roadmap.md) retains work outside this release window.
No deadline waives a security or publication gate.

## Approved delivery sequence

The release owner approved this scope on 20 September 2026 UTC: a first-party,
provider-independent privacy foundation, optional external integrations, and a
local facial engine as follow-up work. This does not make declarations into
verified attributes or permit incomplete higher-assurance checks in production.

The privacy decision sets the first required product journey. The broader
execution queue below was reconciled with the master roadmap on 20 September
after the owner asked for more of the planned v13 capabilities. Privacy,
migration and release preparation are the minimum priorities, not a ceiling
on useful implementation during the active sessions.

The prioritized implementation sequence is:

1. Retain the admitted v12.1 closeout and admit the v13 integration after its
   existing hosted checks pass; carry applicable stable corrections and keep
   mutation findings visible.
2. Admit the optional PostgreSQL replay candidate after its hosted checks. Its
   real-database tests cover concurrency, quota, expiry, failures, cancellation,
   clock rollback and server restart. Keep SQLite's shared-local boundary and
   PostgreSQL's operator-owned replication/failover obligations explicit.
3. Build an authenticated first-party declaration journey, with server-owned
   policy, tenant/session/action binding and rejection when stronger assurance
   is required. Exercise the supported SaaS/LMS consumer shapes without a paid
   provider, fake age determination or mandatory camera capture.
4. Implement purpose/version choices, effective withdrawal of optional processing
   and one scoped rights workflow that actually exports or erases application
   data. Test ownership, cross-tenant denial, retention and restore boundaries.
5. Extend the product queue with server-side SaaS plan entitlements, a typed API
   contract with its first TypeScript consumer, and CLI verification of signed
   Android artifacts. Their bounded acceptance requirements appear below.
   Resolve their SST/API decisions before implementation; none is shipped merely
   by appearing in this plan. Independent work can advance while hosted checks
   run, without launching duplicate campaigns for unchanged inputs.
6. Inventory the actual 12.1-to-13 compatibility changes as each feature lands,
   implement the supported migration rules and exercise generated consumers,
   stale inputs and recovery.
7. Improve the documentation and bounded generated project context for the
   delivered scope. Attempt the small production-linked Verus pilot only after
   the mandatory feature work is ready for its verification campaign.

A provider adapter may enter this release only if its environment, protocol and
acceptance evidence are ready in time. Its absence does not suspend independent
framework work. The final privacy package scope must accurately identify every
supported method; unimplemented facial or provider methods cannot be advertised
as functioning production verification. The package remains unpublished until
its scoped consumer/state/API and release admission criteria pass.

## Starting evidence

- All sixteen v12.1.0 packages are published from `b62390b4`; the
  [release record](v12.md#1210-published-maintenance-release) retains their receipt.
- The post-release closeout in
  [PR #217](https://github.com/Rullst/Rullst/pull/217) merged into `main` at
  `184bc1f7` on 20 September UTC, after 84 successful checks and three skips.
  It records publication, repairs mutation discovery and corrects the SemVer
  baseline resolver; the published source and artifacts remain unchanged.
- The integration candidate carries that exact published runtime source into
  v13, preserving the unpublished privacy crate and Labs/Verus plans. The
  combined source needs fresh CI; the v12.1.0 results do not certify v13.
- [PR #218](https://github.com/Rullst/Rullst/pull/218) admitted the integration
  and SQLite/PostgreSQL replay foundation into `v13` at `7f48d882` on
  20 September UTC. Its reviewed head `03601566` passed 85 hosted checks with
  four declared skips, including workspace tests, strict Clippy and the exact
  coverage floors. Three remaining review threads referred to the already
  individually reviewed test-only alerts 327–329 and were resolved after
  rechecking those fixtures and their existing dispositions. The later native
  declaration, challenge transport and generated consumers require their own
  hosted campaign; this admission does not certify those subsequent changes.
- The initial privacy foundation passed 16 integration tests and one doctest.
  The persistence change adds asynchronous verification, trusted clock rechecks
  and optional shared-local SQLite claims. Its 29 local integration tests cover
  independent pools, a fresh process, reopen, concurrency, quota, cancellation and failure paths.
  Hosted candidate validation and a live age provider remain outstanding; this
  is foundation evidence, not end-to-end production acceptance.
- The PostgreSQL candidate passed focused real-database bootstrap, restricted
  runtime-role, multi-pool replay/quota, expiry during lock wait, schema/durability
  drift, cancellation and server-restart checks. CI and coverage explicitly run
  that disposable-database contract; combined candidate evidence is still required.
- The next local increment adds native declarations and authenticated challenge
  transport, plus an opt-in generated SaaS dashboard journey. Local checks passed
  42 privacy integration tests, one unit test and five doctests; the disposable
  PostgreSQL lifecycle and restart contract also passed explicitly. A generated
  SaaS executes real session/user lookup, production browser middleware, explicit
  choice and durable replay denial; a second process verifies missing-key denial,
  and the PostgreSQL consumer profile compiles. Installer tests cover formatted
  source, changed authentication, source selection, preserved edits and rollback
  after a write failure. Strict local CLI Clippy and book/link checks passed.
  The follow-up LMS consumer also passes a generated SQLite journey: bounded
  school selection, real membership/role resolution, header selection carried
  into a browser form, cross-school/user denial, replay and revoked membership.
  Its declaration writes no subject-age or guardian-consent row and does not
  install age-state middleware around other learning routes.
  Hosted admission and broader purpose/rights workflows remain
  outstanding; these checks do not establish deployed browser/provider acceptance.
- The initial CodeQL scan's 26 age-replay alerts were individually reviewed:
  23 intentional integration-test identifiers, two policy-field data-flow
  conflations and one unmodeled OS-random buffer overwrite. The
  [review receipt](evidence/v13-codeql-age-review.json) binds those dispositions
  to the analyzed source. No query, workflow or source/test path was excluded.
- The initial review found main-only release admission/security filters and an
  unprotected `v13` branch. The preparation change binds each major to its
  release branch and enables the missing automatic checks. On 20 September UTC,
  hosted v13 protection was enabled and read back with 43 required existing
  checks; [the observed profile](../../WORKFLOWS.md#observed-v13-protection)
  records its scope. Combined hosted evidence and packaging remain pending.
- The v12.1.0 post-release mutation campaign is informational. Its repaired
  controller discovers the measured source's complete inventory and limits the
  80 shards to four concurrent jobs. Findings are reviewed separately from v13
  readiness, with security-relevant corrections carried to each affected line.

## Release priorities and acceptance

The next local increment adds two privacy fuzz targets to the v13 release
inventory (42 total; v12 remains 40). Two five-minute ASan diagnostics completed
without a reproducer: 1,442,644 challenge-token inputs and 929,346 attestation
inputs. These are local diagnostics, not the complete hosted release campaign.
The independent `consent`/`consent-sqlite` foundation now provides explicit
purpose/notice choices, revision-bound grants, unconditional withdrawal,
default denial and per-action checks. Its shared-local adapter retains bounded
state and clock metadata, refuses implicit initialization/repair and preserves
withdrawal across pools and processes. Local contracts cover stale forms,
version/tenant/subject boundaries, capacity, clock/expiry, cancellation and
faulty adapters. The opt-in `make:privacy` consumer now supplies authenticated
preferences, an optional name-based greeting and a direct own-account profile
export through a concrete parameterized SQL adapter. Local generated SaaS/full
LMS HTTP fixtures passed both age/privacy installation orders, explicit choices,
withdrawal and stale grants, CSRF, account/session/school changes, expiry, unknown
and oversized inputs, and missing/failed consent state. The profile export stays
independent of optional-consent storage and form keys and never claims to export
other application records or complete queued rights requests. A normal
PostgreSQL-primary SaaS profile also compiles; live primary-database isolation
and deployed browser acceptance are separate from that compile check.

These HTTP fixtures found a Core header composition defect: the outer layer
replaced an endpoint's `no-referrer`. Core and both Security header layers now
share a narrow preservation rule; 36 local composition cases cover layer order,
configured defaults, weaker handler policies and duplicate values. The prior
PR #219 campaign finished with 84 successful checks, four skips and one macOS
unit-fixture failure caused by the system temporary-directory symlink. Its
fixture now resolves the same root as the CLI, with an explicit child-symlink
rejection regression. Four new CodeQL policy-threshold conflations were
individually reviewed and dismissed; the
[declaration review receipt](evidence/v13-codeql-declaration-review.json)
binds those decisions to the old analyzed source. No query was disabled.
Combined hosted acceptance for the subsequent changes remains pending.

The next adoption increment gives the existing sixteen release packages and
their internal requirements the explicit development version `13.0.0-alpha.1`.
Privacy remains unpublished. Migration catalog v2 admits source major 13,
retains target-major selection and downgrade rejection, and invalidates old
preparations. Local updater contracts exercise a distinct 12.1.0→13 resolution,
review, stale-input rejection, application and recovery. Both executable names
report the actual source version; prerelease tests keep the explicit opt-in.
The [adoption guide](migration-v13.md) inventories the current additive changes
and distinguishes tiny updater protocol packages from actual generated
framework consumers. Combined hosted acceptance and an unpublished package
rehearsal remain outstanding.

The development train passed 348 local CLI unit tests, 46 updater process
contracts, 32 command/scaffold contracts and the generated foundation matrix
(Blank/API, SaaS and full LMS, including LMS authorization negatives). Release
preflight still identifies sixteen publishable packages with a consistent
version and topological order; it does not admit the unpublished privacy package.
Strict CLI all-feature/all-target Clippy and production panic checks passed.
These are local checks, not the outstanding combined hosted/package campaign.

The Android candidate now binds one fresh bounded APK to the application's
expected certificate through the SDK verifier, checks its final bytes, rejects
stale/ambiguous outputs and withholds captured signing-tool diagnostics. Native
process fixtures exercise success, explicit selection, wrong certificates,
missing/stale/oversized/changed files, verifier/build failures and output/time
bounds. The hosted Android workflow now exercises this CLI against a real
signed release and independently checks its certificate and receipt. That new
hosted result remains pending; protocol fixture success does not certify SDK
interoperability or devices/stores.

The PR #219 follow-up CodeQL analysis reported one additional threshold-to-nonce
conflation in the isolated privacy fuzz helper. Alert 338 was individually
reviewed against SARIF 1806278303 and dismissed; the
[fuzz-policy review receipt](evidence/v13-codeql-fuzz-policy-review.json) records
the exact source and reasoning. The policy literal is 18; the fuzz-only nonce
is a separate deterministic fixture, and production issuance retains OS entropy.
No source path, query or fuzz target was disabled.

| Priority | Deliverable | Acceptance before calling it complete |
| :--- | :--- | :--- |
| P0 | Published v12.1.0 corrections integrated without losing v13 work | Review conflicts; retain the stable runtime changes; pass the combined workspace tests, strict Clippy, format and feature/consumer checks. |
| P0 | Protected v13 verification and publication path | Explicit major-version/branch policy, protection equivalent to the stable line, complete required workflow/job inventory, negative wrong-branch/tag/evidence tests and an unpublished package rehearsal. Keep v12 maintenance independently releasable. |
| P0 | Usable proportional age-assurance boundary | SQLite shared-local and PostgreSQL multi-host replay protection, authenticated tenant/session/action binding, a first-party declaration journey only where policy permits it, explicit method strength and fail-closed outage/expiry/replay behavior. Production must reject mocks, process-local state and unsupported stronger methods. |
| P0 | First complete privacy journey in a generated consumer | Explicit purpose/version choices, withdrawal enforced on subsequent optional processing, one authorized rights workflow with actual adapter effects, tenant isolation and documented retention/restore boundaries. Do not count a request row as completed export or erasure. |
| P0 | Safe 12.1→13 adoption | Inventory actual compatibility changes, implement only supported migration rules, run generated SaaS/LMS consumer fixtures and prove review, stale-input rejection and recovery. A major-version flag alone is not a migration. |
| P1 | Server-side SaaS plan entitlements | A typed authorization gate bound to authenticated tenant, subject, feature and validity, consumed by a generated SaaS route. Deny expired/revoked or cross-tenant grants. Payment redirects, mock results and unverified events must never grant access. Keep subscription reconciliation explicit. |
| P1 | Typed API and first TypeScript consumer | One explicit schema source for a bounded supported set of request/response shapes, parameters and errors; compile and execute a generated consumer against its server fixture. Prove nullability, serialization, error handling and denied access; reject unsupported schema shapes rather than emitting guessed types. |
| P1 | Android artifact verification through the CLI | Select the intended release output, run signature verification and bind the signer to the application-owned certificate. Reject missing, ambiguous, stale, unsigned or wrong-key artifacts; test process failures and redaction. Validate a real generated APK in hosted Android CI; device/store acceptance remains separate. |
| P1 | Better application context for maintainers and assistants | Accurate generated `AGENTS.md`, a bounded deterministic project map, configuration key names without values, explicit file/secret exclusions, freshness information and executable regression fixtures. |
| P1 | Focused Verus pilot | One production age-policy property with checked linkage, pinned tooling, negative controls and measured cost. No framework-wide verification claim or automatic expansion to every crate. |

P0 items have precedence over new integrations, cosmetic rewrites and expanding
the number of crates. Each implementation should remain a small reviewable
change with its own tests and migration/documentation updates. The acceptance
column is a requirement, not a description of code already present.

## Coverage of the wider roadmap

The master roadmap contains 41 umbrella milestones, including the separately
governed M31 programme. Its 40 framework rows currently label five bounded
implementations, 25 partial foundations and ten unimplemented ambitions.
These unequal units cannot tell us whether this release adds "10% of all future
work". Nor does finishing one increment close its entire parent milestone.
The wider v13/v13+ programme remains available for subsequent minor releases.

This queue deliberately builds on existing code instead of restarting those
capabilities. Within P1, a small independent increment can precede a larger one
when its dependencies and verification capacity are ready.

| Roadmap area | Starting point and next bounded increment | Scheduling boundary |
| :--- | :--- | :--- |
| M41 — privacy and age | Policy, signed attestations, SQLite and PostgreSQL replay adapters exist in the unpublished candidate. Admit their hosted evidence, then add an authenticated declaration path and enforceable purpose/rights effects in a consumer. | First product priority; provider availability does not block independent work. |
| M11/M33 — SaaS entitlements | LMS already has application-owned course entitlements. Add a reusable typed plan gate and one generated SaaS enforcement journey; a public attribute macro needs its own design and compile tests. | Active P1 queue; do not imply that the entire billing programme is complete. |
| M5/M29/M34 — API/SDK contracts | Scalar and the route-scanning OpenAPI generator exist, but scanning currently emits placeholder responses. Add a schema-backed supported API profile and one TypeScript target. | Active P1 queue; React, Dart and Swift targets follow the proven schema contract. |
| M21 — Omni/Android | Version 12.1 configures application-owned signing; hosted Android CI already verifies signatures and the expected certificate. Bring that verification into the public CLI with exact artifact handling. | Active P1 queue; this closes a CLI gap, not physical-device or store acceptance. |
| M9 — Auth/session consistency | Durable account recovery and revocation exist. Evaluate one shared passkey-ceremony lifecycle with atomic single use, expiry and tenant/session binding. | Next extension after the active product increments; choose the storage/API boundary before promising implementation. |
| M15 — remote messaging | Wire contracts, local durable state and the ORM outbox exist. Evaluate one remote broker adapter with real restart, redelivery and lease/idempotency evidence. | Conditional extension; select a broker and supported semantics first. Seven adapter names are not seven functioning integrations. |
| M40 — Labs | The threat model and separate web-contract/runner roadmap are retained. The first candidate is a bounded job/grading/receipt contract with a deterministic local protocol fixture. | Conditional extension; contracts alone do not deliver code execution. A usable runner requires its own isolated deployment and adversarial acceptance. |
| M1/M3/M7/M12 — adoption and assurance | Carry the compatible updater forward, add actual major-version migrations, improve generated guidance and connect new code to the relevant verification inventory. | Required adoption/security work plus bounded maintainer tooling; Verus begins with one production-linked pilot. |

Gateway/load-balancer implementation, fiscal homologation, physical IoT/Embassy,
PQC protocols, local facial inference and broad database replication retain
their dedicated roadmap scope. They are not counted as completed by a proposal,
an empty crate or a mock. Reconsider them through their own dependency and
acceptance evidence instead of promising the whole long-term programme by the
26th.

At each implementation checkpoint, record the delivered behavior, source,
focused checks, pending hosted checks and next useful increment. The decision
to start the next item uses remaining integration/test capacity as well as
coding time. Preserve working increments and report unfinished acceptance
explicitly; do not inflate a feature count by splitting contracts into crates.

## Age and privacy design decisions to close first

Keep age assurance and reusable privacy contracts in the existing optional
`rullst-privacy` package. Do not add a second age crate or make cameras, a vision
runtime, a database or a provider a default Core dependency.

The unpublished `ReplayStore::claim` interface now returns a statically
dispatched `Send` future. Verification samples a trusted clock before validation
and after storage, denying expiry or rollback during the wait. The initial
SQLite adapter uses serialized durable transactions, persisted quota/clock
metadata and hashed nonces. Local tests cover independent verifier pools, reopen,
simultaneous claims, capacity, expiry, cancellation, uncertain acknowledgement
and unavailable storage. Native hosted evidence remains pending. Its
shared-local boundary is distinct from the PostgreSQL adapter's multi-host operation;
restoring an old backup requires quiescing verification and invalidating all
outstanding challenges through a newly enforced policy or retired signing keys.

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
provider milestone complete. The approved independent foundation has its own
consumer/state/API acceptance; any live adapter additionally needs its provider
acceptance. Review this precise publishable package boundary at feature freeze,
preserving the security guarantees of every advertised method.

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
| 21–22 September | Implement and test PostgreSQL/age/privacy consumers; advance the active SaaS/API/Omni increments as dependencies and verification capacity permit. Maintain migration fixtures with each API change. Decide any optional provider inclusion by the end of the 22nd based on available sandbox evidence. |
| 23 September | Close accepted product increments, generated guidance and documentation; evaluate Auth/broker/Labs extensions and the small Verus pilot against remaining acceptance capacity. Freeze feature scope by the evening. |
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
