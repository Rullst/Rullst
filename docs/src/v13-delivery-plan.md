# v13 delivery plan through 26 September 2026

**Engineering target: a reviewable, tested v13 release by 26 September 2026,
America/Sao_Paulo.** This is a bounded delivery plan, not a claim that the entire
historical roadmap fits the remaining week. The [SST](spec.md) governs APIs and
architecture; the [roadmap](roadmap.md) retains work outside this release window.
No deadline waives a security or publication gate.

## Source-line transition approved on 21 September

The owner approved development on `main`, stable maintenance on `v12`, and
retention of the Pages branch. Stable source `184bc1f7` is preserved in the
`v12` history. [PR #237](https://github.com/Rullst/Rullst/pull/237) prepared its
maintenance workflows and versioned release rules; it merged at `0bb7406b`
after all 43 required checks and 24 relevant workflows passed. `main` and `v12`
have the same strict, admin-enforced 43-check protected profile.

The six-feature source was admitted in PR #236 as recorded below.
[PR #238](https://github.com/Rullst/Rullst/pull/238) combines that source with
all six Dependabot updates and the `13 → main`, `12 → v12` transition. Its exact
combined source still requires workspace/platform, coverage, security and
extracted-package admission before a normal protected merge. No tag or
published crate is changed.

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
8. **First additional priority, explicitly requested on 20 September:** evaluate
   `rullst-supervision` for transparent exam/learner supervision and parental
   rules within the application, ahead of other conditional extensions. Start
   after the required deliveries are implemented and validated, leaving time
   for combined acceptance and publication. The preliminary engineering estimate
   is one to two days for a bounded first journey and its tests, plus hosted CI;
   it is not a commitment to complete device-wide controls by 26 September.
9. **Subsequent September 20 direction: finish usable journeys before opening
   more optional implementation streams.** After the current deployment and
   supervision implementations, prioritize Bunny Stream private course video,
   then Labs with an actual isolated exercise/grading path. A signing helper or
   mock-only Labs contract is an intermediate checkpoint, not either product's
   completion. Remote brokers, a new gateway and expansion of the Verus pilot
   follow these journeys; mandatory fixes and release checks retain precedence.

## Additional priorities approved on 21 September

After the six-feature source admission, the owner selected the following two
deliveries for implementation through September 23. They are **planned**, not
implemented or release-admitted. Finish the current combined PR #238 admission
and carry necessary fixes forward while independent implementation progresses.

| Order | Selected delivery | Required acceptance |
| :--- | :--- | :--- |
| 1 | Shared PostgreSQL consent in `rullst-privacy` | Preserve purpose/version binding, exact-revision grants, withdrawal precedence, clock checks and durable tombstones across independent application pools. Add explicit initialization, restricted-role operation, bounded waits, concurrency/cancellation/restart tests, opt-in facade features and a documented consumer. Keep database restore/failover obligations explicit. |
| 2 | Single-use email login links in `rullst-auth`, composed with `rullst-mail` | Complete issuance, delivery and deliberate redemption into the existing authenticated session lifecycle. Require purpose-separated secret tokens, durable atomic consumption, expiry, tenant/account binding, bounded abuse controls, enumeration-resistant responses and safe redirects. Email scanners must not consume credentials merely by following a GET. Test replay races, mail/storage failures, stale accounts and session invalidation with deterministic delivery fixtures. |

The email-login item now has a local Auth/Mail candidate: explicit account opt-in,
independent browser/email secrets, atomic opaque-session creation, durable
SQLite/PostgreSQL state, fenced encrypted delivery and deterministic localized
Mail templates. Local SQLite, native PostgreSQL, Chromium, fresh-process,
database-restart and extracted-facade Auth/Mail contracts passed. Hosted
source/package/coverage admission is still required. See the
[email-login contract](email-login.md). No owner-provider accounts were used.

The owner subsequently approved the five additional items below as the next
implementation queue after the two priorities, emphasizing security throughout.
All seven are targeted through September 23; completion still requires the
advertised behavior and its acceptance evidence. Report any unfinished item
before the freeze instead of silently including it in the supported release.
The previously discussed metadata-only Studio messaging inspector remains a
separate candidate.

| Priority | Additional candidate | User benefit and required boundary |
| :--- | :--- | :--- |
| 1 | Scoped, revocable application API tokens | Let integrations access explicit tenant/account permissions without borrowing browser sessions. Store token digests, bound scopes and lifetime, support revocation/rotation, and test cross-tenant denial and concurrent revocation. Existing provider credentials and application JWT helpers do not implement this lifecycle. |
| 2 | Shared PostgreSQL mail suppression | Let independent mail workers honor the same hard-bounce, complaint and explicit suppression state. Extend the current Memory/SQLite contract with atomic event replay handling, minimized state, restart/concurrency evidence and a final pre-delivery check; actual provider inbox acceptance remains separate. |
| 3 | Durable recurring schedules across application instances | Coordinate recurring occurrences through authoritative storage and leased dispatch into the existing queue/outbox. Define missed-run policy, cancellation, restart, stale-worker fencing and idempotent occurrence keys. Current per-process cron and delayed queue jobs are foundations; neither proves exactly-once external effects. |
| 4 | Durable outgoing application webhooks | Deliver application events to explicitly approved destinations with signatures, bounded retries and inspectable terminal failure. Compose the existing outbox, destination/SSRF policy and idempotency contracts; incoming payment-webhook verification is a different capability. |
| 5 | Resumable multipart uploads for private S3-compatible storage | Support larger attachments and interrupted uploads with bounded parts, authenticated tenant/object ownership, checksums, completion/abort and orphan cleanup. Extend the existing private-object adapter; this is separate from Bunny Stream resumable video uploads and requires native protocol evidence. |

Execution order may respond to measured implementation and validation cost. Do not replace
full journeys with mock-only placeholders to increase the feature count. Keep
September 24–25 for combined validation and September 26 for final adjustments
and separately authorized publication. No owner/provider account tests are
authorized; disposable local services and deterministic protocol fixtures remain
available. Architectural/API decisions must be recorded in the SST as each
candidate becomes implementation work.

## Depth before additional optional features

The owner asked to pursue a complete, comprehensive implementation of Bunny
and Labs before moving to other new capabilities. Finish the supported user
journey, its failure/recovery paths, integration documentation and executable
acceptance before widening the provider, language or backend matrix. This
changes priority and the intended outcome; it is not evidence that either
implementation exists or a promise to finish both by the 26th. The feature
freeze and validation dates remain unchanged.

| Order | Intended usable outcome | Completion boundary |
| :--- | :--- | :--- |
| 1 — Bunny Stream | An instructor manages a course-owned video through creation, authorized resumable upload, processing, publication, metadata changes, withdrawal and deletion; an entitled learner obtains and renews private playback access. | Complete the selected provider lifecycle, durable ownership/state, notification reconciliation, retry/recovery paths, application/browser consumer, automated protocol tests and operations guidance. Live-account interoperability remains unvalidated under the owner's no-live-testing constraint. See the [detailed scope](managed-video-roadmap.md#completion-target-before-another-optional-integration). |
| 2 — Labs | An instructor defines a versioned exercise and grader; an authorized learner submits, executes and receives a bounded result through a separately deployed runner, with cancellation and recovery. | Complete the contract stage, then pursue one named isolated backend and a supported exercise/toolchain profile with actual execution, deterministic grading, isolation and failure tests. Contracts and a mock alone do not complete this outcome. See the [delivery target](rullst-labs-roadmap.md#v13-usable-journey-target). |

Divide implementation into small reviewable commits and PRs without declaring
the journey complete early. While hosted checks run, advance related
documentation, recovery tests and packaging. Move implementation effort to the
next journey after the preceding supported lifecycle and focused acceptance
pass; required hosted admission may continue independently. Do not open an
unrelated optional feature while useful completion work remains.

If a journey cannot meet its acceptance by the evening of September 23, report
the precise unfinished behavior and keep it out of the stable supported scope.
An independently useful contract package may remain explicitly experimental,
but cannot be presented as a completed execution product. Preserve the reserved
validation days and the isolation, provider-evidence and publication requirements.

A provider adapter advertised as live-validated requires environment, protocol
and acceptance evidence. The owner's later direction allows opt-in integrations
with automated protocol evidence and explicit unvalidated-provider status; do
not use personal/live libraries for testing. Provider availability does not
suspend independent framework work. The final privacy package scope must accurately identify every
supported method; unimplemented facial or provider methods cannot be advertised
as functioning production verification. The package remains unpublished until
its scoped consumer/state/API and release admission criteria pass.

## Blueprint scope and automated acceptance

On September 20 the owner prioritized implementation and testing over extending
every new feature to every blueprint. Keep existing working integrations and
their regression tests. For new work, choose the smallest real consumer that
exercises the capability: supervision in LMS, billing entitlements in SaaS,
and a focused server/client fixture for an API contract. A public generator must
still test each supported output shape; do not advertise an untested shape.
Additional blueprint coverage belongs in the release only when it demonstrates
a different supported contract or provides clear product value. Opt-in features
must remain opt-in, and source/manifest, authentication, persistence and packaged
installation checks cannot be replaced with mock-only examples.

Human exploratory testing is not assumed to be available during this delivery
window. Use automated unit, integration, real-database, HTTP/browser and failure
tests appropriate to each supported feature. Record environmental and usability
limits explicitly; neither AI authorship nor a passing suite proves every
deployment. Missing provider/device/isolation evidence stays missing and limits
the supported scope. Defer optional scope and non-critical improvements when
needed, but fix known authorization, privacy or data-integrity failures before
admission. The possibility of a later patch does not turn a failing contract into
an accepted release.

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
PR #219's corrected head `d783f3b7` subsequently passed 85 hosted checks with
four declared skips, including all-feature workspace tests, strict Clippy,
coverage floors and the generated privacy consumers. It merged into `v13` at
`df589770` on 20 September UTC. This admits that privacy increment; it does not
certify the later adoption, Android or entitlement changes.

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
hosted SDK job passed on PR #220 head `52657955`: it compiled and verified a
signed release APK through the CLI and the independent SDK check
([run 35497371156](https://github.com/Rullst/Rullst/actions/runs/35497371156)).
The rest of the combined campaign remains pending. This is SDK interoperability
evidence; device/store acceptance remains separate.

The PR #219 follow-up CodeQL analysis reported one additional threshold-to-nonce
conflation in the isolated privacy fuzz helper. Alert 338 was individually
reviewed against SARIF 1806278303 and dismissed; the
[fuzz-policy review receipt](evidence/v13-codeql-fuzz-policy-review.json) records
the exact source and reasoning. The policy literal is 18; the fuzz-only nonce
is a separate deterministic fixture, and production issuance retains OS entropy.
No source path, query or fuzz target was disabled.

PR #220 head `52657955` passed the real SDK signed-APK job, but its coverage
and macOS CLI campaign exposed two Android fixture/build-timing issues. The
follow-up preserves OS process helpers in the fixture PATH and compares artifact
timestamps against a temporary filesystem anchor. Local native selection and
redaction scenarios passed after correction; a fresh combined campaign is required.

[PR #220](https://github.com/Rullst/Rullst/pull/220) admitted the adoption,
Android and conditional supervision-planning changes into `v13` at `6ccbe2a7`
on 20 September at 14:20 UTC. Its exact head `ee4f46df` completed 86 successful
checks and four declared skips, with no unresolved review threads. This includes
the full OS/workspace matrix, strict Clippy, coverage and the real SDK signed-APK
job. The initial campaign's changelog, filesystem timing and terminal prerelease
fixture failures were corrected without weakening their checks. This admission
does not certify the later entitlement, context, API, Verus or packaging changes.

The entitlement candidate adds a typed current-state gate in Capital and an
authenticated SaaS billing report. It requires an exact server plan allowlist,
fresh revision-fenced Stripe reads, a matching live/sandbox mode and an active,
unexpired subscription. The six focused local contracts passed tenant/owner,
status/mode, revocation, expiry, stale/future clock and adapter-failure cases.
The existing generated SQLite/Turso billing contract also passed. The additional
generated SaaS HTTP/reconciliation fixture passed all four configuration profiles,
including real session/owner checks, a delayed active read losing to revocation,
the 20-second provider deadline and strict generated all-target Clippy. Its
test-only provider boundary is explicit; it is not a live Stripe acceptance run.
CMS projections and offline billing fixtures cannot grant the new report.
Local validation also passed the public doctest, strict Capital default-feature
all-target Clippy, CLI all-feature/all-target Clippy, production panic checks,
353 CLI unit tests (one parent-owned child fixture ignored by the outer harness),
20 command/scaffold contracts and book/local links. Combined hosted acceptance
remains outstanding.

The project-context candidate replaces raw source concatenation with a bounded,
versioned file/dependency/configuration-key inventory. It creates common project
instructions only when absent, preserves user `AGENTS.md`, supports non-writing
freshness checks and records source scope/exclusions explicitly. Local contracts
cover private-value omission, links, size/depth/count budgets, stale inputs,
legacy migration, conflicting output and all six generated blueprints. Semantic
route/authorization inference and task-specific context shards remain follow-up
work; the [guide](project-context.md) states the exact candidate boundary.

The [schema-first API candidate](typed-api.md) now generates Rust DTOs/operation
codecs and a strict TypeScript client from one bounded OpenAPI 3.1 JSON profile.
Seven generator contracts cover unsupported shapes, links, duplicate keys,
collisions, regeneration and freshness. Local validation passed 371 CLI unit
tests (one parent-owned child fixture ignored), strict all-target Clippy, the
real generated Rust/TypeScript HTTP consumer, ten existing command contracts
and the six-blueprint context journey. The HTTP fixture proves typed statuses,
Unicode, optional/null semantics, exact safe integers, nested nullable arrays,
unauthenticated/cross-owner denial and adversarial transport rejection. A
GET-only Rust module also compiles with strict Clippy. Hosted Linux CLI acceptance
requires the pinned TypeScript compiler and the actual HTTP journey; combined
hosted admission remains pending.

The Verus pilot has a local candidate for the unchanged production
`AgePolicy::permits` predicate. Rust syntax extraction checks the actual package,
module path, enum/type/signature domain and body; the verifier covers all nine
risk/method combinations. Both local runs verified the predicate and rejected
all three deliberately incorrect implementations. They took about 12.1 seconds
including complete pinned-tool validation, with prover peak RSS below 292 MiB.
The runtime age suite and MSRV 1.96 check passed. The prepared manual workflow
is separate from release admission; clean hosted evidence and any promotion
remain pending. See the [proof boundary](verus-roadmap.md); this does not prove
age evidence, expiry, replay state or application authorization.

The next packaging candidate adds `rullst-privacy` as the seventeenth package,
with its license, a bounded source archive, opt-in `rullst::privacy` features,
coordinated upgrade inventory and registry-shaped age/privacy generators.
The local seventeen-archive packaging/content audit and the durable facade
reopen test passed. Both generated SaaS/LMS privacy journeys passed, including
the registry-shaped LMS dependency and the existing local-source SaaS profile.
Context freshness for both new commands, 372 CLI unit tests (one parent-owned
child fixture ignored), strict CLI Clippy and the minimal facade panic gate also
passed. A separate consumer compiled only the extracted framework archives and
passed that durable facade test. All 42 fuzz targets' eleven locked dependency
graphs resolved offline without running a new fuzz campaign.
Combined hosted acceptance remains outstanding. All seventeen local archives
were subsequently generated from clean commit `30e2f753`; their source identity,
licenses and excluded secret/database paths were audited. The archive-only
privacy consumer and privacy publication dry run passed without uploading a
version. These are local rehearsals, not final release receipts.
The archive-only hosted test compiles the installed CLI's SaaS/LMS opt-ins and
executes the same durable facade contract from extracted packages. It passed at
`365d2252` in [run 35520146394](https://github.com/Rullst/Rullst/actions/runs/35520146394)
on September 20: seventeen package archives were audited, the durable facade
test passed and all six installed-CLI blueprints compiled. This diagnostic does
not replace the final full/native release campaign or publication authorization.
A crates.io read on 20 September returned 404 for `rullst-privacy`;
initial registration and Trusted Publishing configuration remain required.
The stable publisher continues to reject an unregistered package.
The ownership-policy validator now checks each proposed bootstrap name against
the inventory, with unknown/duplicate/malformed-name negatives; the empty
bootstrap allowlist and the stable publisher's refusal remain in force.

PR 220's Linux terminal fixture also needed an explicit prerelease dependency
requirement and `--prerelease` when exercising the alpha CLI. All four local
terminal scenarios passed after that correction: decline copying, decline
execution, apply/recover, and a failing application. The corrected `ee4f46df`
head then passed its own combined hosted campaign, as recorded above; previous
Android/macOS passes were not substituted.

On 20 September at 07:31 UTC, the first four v12 mutation shards had hit their
330-minute execution budget; their partial artifacts were retained. For example,
shard 2 had executed 72 of 220 scheduled mutations (40 caught, 30 surviving,
two unviable), so it is explicitly incomplete. Surviving mutations require
individual review and are not automatically confirmed runtime defects. The
remaining campaign continues; partial results must not be reported as a complete
mutation pass, and any later continuation must preserve source/inventory identity.

The first combined [PR #221](https://github.com/Rullst/Rullst/pull/221) campaign
at `4773171a` passed 85 checks, including the OS/workspace and generated consumer
matrix, strict Clippy, coverage, feature boundaries and MSRV. Two inventory checks
failed: the README omitted the added Verus workflow, and the observational
scorecard omitted the seventeenth package. The correction retains both checks,
adds a conservative unpublished privacy ceiling and preserves its planning floor.
The corrected head requires its own hosted admission. A new explicit package-only
diagnostic runs the existing archive/installed-CLI acceptance without repeating
the OS matrix; a regression proves that this subset cannot admit a release.

The corrected `365d2252` head subsequently passed **87 hosted checks**, with four
declared skips and no unresolved review threads. PR #221 merged normally into
`v13` at `48a6ee51` on September 20 at 17:23 UTC, with all 43 required checks and
administrator enforcement preserved. Its seventeen-archive/installed-CLI
diagnostic passed separately as recorded above. This admits the combined
application/privacy candidate; the final full/native/security release campaign
and initial new-package registration remain outstanding.

The first conditional supervision implementation now has a separate unpublished
crate, bounded domain contracts and shared-local SQLite state. Its focused local
tests cover explicit acknowledgement, scoped/revocable authority, session
transitions, event bounds, parental course windows, retention, fresh-process
reopen, concurrent revisions, lock-wait expiry, cancellation and corrupt rows.
An explicit full-SQLite-LMS generator now installs the original learning-service
gate, signed/scoped SSR forms, local operator provisioning and a bounded visibility
collector. Three local CLI integration tests pass, including both privacy/age
composition orders, public-profile compilation, the real HTTP lifecycle and
Chromium keyboard/no-JavaScript/visibility controls. A start form is bound to the
last retained session revision so an old acknowledgement cannot silently start a
new session after end. The generated application passes all fourteen original
LMS library tests and strict production Clippy/zero-panic checks, and all 376 CLI
library tests pass. These local results do not admit the package: full workspace
regression, installed archives and hosted acceptance remain outstanding.

PR #222 head `20a9fca7` subsequently passed 86 hosted checks with four declared
skips and no unresolved review threads. Its independent packaged-distribution
run `35534311217` also passed at that exact head. The supervision source merged
normally into `v13` at `081c3805` on September 20, 21:14 UTC, preserving all 43
required checks and administrator enforcement. This admits that source increment;
stable package registration/publication and the final combined v13 campaign are
still outstanding. The next shared-passkey candidate is under its own hosted
campaign; the deployment/body and local diagnostic increments remain separate.

The local deployment diagnostic passed seven executable contracts, including a
generated SaaS, and two additional literal-parser/boundary contracts. All 376
previous CLI unit tests passed; strict all-target CLI Clippy, book/local links
and architecture checks passed. A bounded six-mutation decision sample was fully
caught. Inspection also reproduced acceptance of the public `.env.example`
application-key placeholder in Auth; explicit rejection now passes the runtime
regression, 65 Auth unit tests and three key-resolution process contracts, with
strict all-target Auth Clippy. These are local candidate results; hosted and
installed-package acceptance remain outstanding. The corresponding stable-line
backport still needs preparation and its own checks; published artifacts remain
immutable.

The next four source increments were merged into `v13`; their evidence is
recorded individually below:

- [PR #223](https://github.com/Rullst/Rullst/pull/223), shared PostgreSQL passkey
  ceremonies, merged at `b354d67c`. The exact `15addc3d` candidate passed hosted
  checks and [archive acceptance](https://github.com/Rullst/Rullst/actions/runs/35542202337).
- [PR #224](https://github.com/Rullst/Rullst/pull/224), HTTP response-body draining
  and the Caddy/Redis deployment contract, merged at `a4a3b3e6`. The exact
  `2e348f96` candidate passed hosted checks and
  [archive acceptance](https://github.com/Rullst/Rullst/actions/runs/35547984264).
- [PR #225](https://github.com/Rullst/Rullst/pull/225), deployment diagnostics and
  rejection of the public Auth key placeholder, merged at `873bc14a`. The exact
  `45a573cc` candidate passed hosted checks. The archive job in
  [run 35554979487](https://github.com/Rullst/Rullst/actions/runs/35554979487)
  was skipped, so that run is **not archive acceptance**. Correctly selected
  [exact-commit validation](https://github.com/Rullst/Rullst/actions/runs/35564762822)
  subsequently passed on September 21 at 06:32 UTC, including the actual archive
  audit, extracted consumers and all six installed-CLI blueprints. This repairs
  that missing gate retrospectively.
- [PR #226](https://github.com/Rullst/Rullst/pull/226), transparent exam
  observations and bounded analysis adapters, merged at `bb71dad8`. The exact
  `ae9b6d57` candidate passed 86 hosted checks. The archive job in
  [run 35558912935](https://github.com/Rullst/Rullst/actions/runs/35558912935)
  was skipped, so that run is **not archive acceptance**. The correctly selected
  [exact-commit validation](https://github.com/Rullst/Rullst/actions/runs/35564765646)
  subsequently passed on September 21 at 06:27 UTC, including the actual archive
  audit, extracted consumers and all six installed-CLI blueprints. This repairs
  that missing gate retrospectively.

The September 21 audit found that these last two merges preceded their required
installed-archive validation: a successful workflow was incorrectly credited
despite its skipped archive job. Both gaps are now repaired by the actual
exact-commit runs recorded above. These retrospective results do not erase the
missed premerge gates; subsequent source admission requires checking the named
archive job itself before integration.

These merges do not publish packages or replace the final combined release
campaign. Bunny subsequently passed its own source admission in
[PR #227](https://github.com/Rullst/Rullst/pull/227): the exact `b21d52e7` candidate
passed all 87 current checks, the actual
[installed-archive job](https://github.com/Rullst/Rullst/actions/runs/35564717398),
and review with no unresolved threads, then merged at `618655c1`.
[PR #228](https://github.com/Rullst/Rullst/pull/228) subsequently admitted Labs
source at `278a115b`. Candidate `977e40a3` passed all 24 native shards, 43
branch-required checks, actual isolated execution, the actual installed-archive
job and both 90% whole/library coverage floors. Its non-required Codecov patch
check remained at 83.45936%, below its 90% target; the
[profile evidence](labs-first-profile.md#recorded-linux-acceptance) records this
limitation without exporting worker profiles or relaxing isolation. Independent
isolation review and final release admission remain outstanding. Stable-line
backport preparation remains separate as well.

The September 20 observation extension now adds selected browser categories,
capture-status contracts and bounded analysis adapters to the supervision
candidate. Its five generated CLI/composition/process contracts pass with real
Chromium, alongside 378 CLI unit tests, isolated feature checks, strict Clippy
and an offline simulated adapter example. A targeted nine-mutation guard sample
is fully caught. No camera/audio model, automatic misconduct determination or
live-provider validation is claimed. Hosted workspace checks passed in PR #226;
the missing archive gate subsequently passed as recorded above. The final
release campaign remains required. See
[the integration guide](supervision-observations.md).

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
| First conditional extension | `rullst-supervision` candidate | One generated LMS journey with visible sessions, explicit permissions, independently established guardian/reviewer authority, effective revocation, bounded events/retention and tested application-side restrictions. Prove cross-school/subject/reviewer denial and browser behavior. Client observations cannot prove misconduct or automatically change grades. Device-wide control, camera inference and a mock-only package do not satisfy this scope. |

P0 items have precedence over new integrations, cosmetic rewrites and expanding
the number of crates. Each implementation should remain a small reviewable
change with its own tests and migration/documentation updates. The acceptance
column is a requirement, not a description of code already present.

## Coverage of the wider roadmap

### Additional increments approved on September 21

The owner approved the following order after the Bunny/Labs source work. Finish
each supported journey and its acceptance before expanding its advertised scope.
The six implementations passed the source admission recorded below; their
combined dependency validation and final release admission remain separate.

| Order | Increment | Completion boundary |
| :---: | :--- | :--- |
| 1 | Private S3/R2 storage in existing Core/facade APIs | Upload, download, metadata, deletion and temporary private GET grants; current tenant/owner authorization, bounded failures, an independent disposable S3 service and an extracted-package consumer. See [the candidate contract](private-object-storage.md). |
| 2 | Active-session inventory and effective revocation in Auth | Authenticated inventory, expiration, selective logout and logout of other sessions with rejection by actual request verification across processes. Preserve account/tenant isolation, recovery and fail-closed storage behavior. See [the candidate and migration contract](session-management.md); source/package admission passed in PR #236. |
| 3 | One remote Messaging adapter | Redis Streams selected; see [the candidate contract](redis-messaging.md). Prove publication acknowledgement, restart/redelivery, competing consumers, retry/DLQ and outbox composition against a disposable broker. Source/package admission passed in PR #236. |
| 4 | Reconnection and recovery for server-driven real-time interfaces | The opt-in Core/facade candidate uses full snapshots, current authorization, transactional expected revisions and bounded connections, actions and slow peers. Actual WebSocket and Chromium tests cover conflicts, uncertain writes and application restart without automatic mutation replay. See [the recovery contract](live-recovery.md); source/package admission passed in PR #236. |
| 5 | Operation tracing across processes | Core/facade's optional candidate has explicit parent trust, approved operation labels, minimized bounded OTLP export and an owned lifecycle. Separate Messaging processes reach a standard TLS collector with verified ancestry; local failure/queue/TLS contracts pass. See [the tracing profile](distributed-tracing.md); source/package admission passed in PR #236. |
| 6 | An application-driven ORM increment | Transactional partial updates selected: merge a submitted lesson/profile patch into the current row and reuse policy, hooks, encryption, atomic audit and post-commit effects. Preserve the caller model on rejected operations and add explicit transaction support. See [the candidate contract](transactional-partial-updates.md); local database/cancellation/cache/Scout and extracted-consumer checks passed; source/package admission passed in PR #236. |

### Six-increment source admission on September 21

[PR #236](https://github.com/Rullst/Rullst/pull/236) admitted all six increments
at source `548216881e062b83d6b73a824abe140936aca5b8`. It merged normally into
protected `v13` at `d246ea072a0bddd1ced86471c49ffdb22d2b96cc`; its tree matches
the tested merge candidate. Evidence for this source:

- [Rust CI](https://github.com/Rullst/Rullst/actions/runs/35634162531): all 43
  protected requirements passed, including the Linux/macOS/Windows matrix,
  strict Clippy, format, MSRV and feature boundaries. Private S3 and isolated
  Labs acceptance also passed. All 21 relevant PR workflows completed successfully.
- [Coverage](https://github.com/Rullst/Rullst/actions/runs/35634162210):
  103382/114571 whole-repository lines (90.2340%) and 74906/82383 governed
  library lines (90.9241% across 581 files). Both unchanged 90% floors passed;
  the downloaded summary was independently checked. The all-feature nextest
  artifact records 2701 cases with zero failures, errors or skips; default,
  browser, database, collector and isolated-controller profiles ran separately.
- [CodeQL](https://github.com/Rullst/Rullst/actions/runs/35634162178): Rust and
  JavaScript scans bound to the exact merge source completed with zero open
  findings. All seven review conversations were resolved.
- [Packaged distribution](https://github.com/Rullst/Rullst/actions/runs/35634201254):
  21 candidate archives audited, archive-only consumers exercised, and all six
  blueprints compiled through the isolated installed CLI. The actual package
  job executed; skipped diagnostic jobs were not credited as PR checks.

This admits the documented source contracts, not a crates.io release or every
parent roadmap milestone. No owner/provider accounts were exercised. The Labs
profile retains its separate independent isolation-review boundary. The new
main-line dependency combination must pass its own exact-source campaign.

After this approved round, report the delivered scope and remaining time to the
owner before selecting another round. September 24–25 remain reserved for
combined validation; September 26 remains the conditional final-adjustment and
publication day. Publication is excluded from the current implementation goal.

The [master roadmap](../../ROADMAP.md#executive-milestone-tracker) owns the current
status of the framework milestones and the separately governed M31 programme.
Its milestones differ substantially in scope and remaining effort; counting
their status labels cannot tell us whether this release adds "10% of all future
work". Nor does finishing one increment close its entire parent milestone.
The wider v13/v13+ programme remains available for subsequent minor releases.

Items deferred beyond the September 26 target do not automatically become v14
work. Compatible additions can ship in 13.1, 13.2 and later minor releases;
breaking changes belong in a future major under the
[compatibility policy](compatibility-policy.md). The current major's concrete
CLI/configuration change and the limits of its SemVer evidence are recorded in
the [migration inventory](migration-v13.md#why-the-candidate-uses-a-new-major).
The owner explicitly confirmed retaining the v13 release train on September 20.

This queue deliberately builds on existing code instead of restarting those
capabilities. Within P1, a small independent increment can precede a larger one
when its dependencies and verification capacity are ready.

| Roadmap area | Starting point and next bounded increment | Scheduling boundary |
| :--- | :--- | :--- |
| M41 — privacy and age | Hosted admission includes proportional policy, declarations, signed attestations, SQLite/PostgreSQL replay adapters, optional processing withdrawal and a scoped own-account profile export in generated consumers. | Preserve those admitted journeys in the final campaign. Initial package registration remains outstanding; live age providers and facial models remain outside the delivered scope. |
| M11/M33 — SaaS entitlements | The typed current-state Capital gate and generated SaaS enforcement journey were admitted with PR #221. | Preserve current tenant/owner, plan and subscription checks during combined validation; the broader billing roadmap is not complete. |
| M5/M29/M34 — API/SDK contracts | The bounded schema-first Rust/TypeScript API profile and real HTTP consumer were admitted with PR #221. | Retain schema/transport/ownership regression coverage. React, Dart and Swift targets remain follow-up work; route scanning does not supply typed response semantics. |
| M21 — Omni/Android | CLI verification of the exact signed APK and configured certificate was admitted with PR #220 after hosted Android SDK acceptance. | Preserve the artifact/signature checks in the final campaign. Physical-device and store acceptance remain unvalidated. |
| M9 — Auth/session consistency | PR #223 admitted optional PostgreSQL passkey ceremonies with tenant/account/session/RP binding, bounded single use, real database/process recovery and a Chromium virtual authenticator. | Preserve credential-owner/revocation/counter CAS at the host and repeat affected combined-release checks. See [the contract](shared-passkey-ceremonies.md). |
| Transparent supervision | The baseline passed hosted and archive acceptance in PR #222. PR #226 merged the prioritized exam-platform extension after hosted checks; its missing archive gate subsequently passed in the exact-commit run recorded above. | Preserve transparent permissions, typed uncertain observations, manual review and no raw media retention in the final campaign. See the [integration boundary](supervision-observations.md). |
| M27 — deployment acceptance with an existing proxy | PR #224 admitted the response-body lifetime correction and the real two-process Caddy/Redis contract for readiness, draining, shared budgets/outage, forwarding, CSRF/body limits and WebSocket behavior. | Preserve combined-release coverage. This loopback fixture does not establish generated multi-replica Foundry, cross-host failover or zero downtime. See [deployment acceptance](deployment-acceptance.md). |
| M10/M27 — cloud and VPS application protection | PR #225 merged the offline `deploy:doctor` with explicit environment sources, bounded inputs, redacted reports and rejection of the public Auth key placeholder after hosted checks; its missing archive gate subsequently passed in the exact-commit run recorded above. | Preserve the installed-archive and diagnostic coverage in the final campaign and prepare the separate stable Auth backport. No automatic host/cloud changes or volumetric DDoS guarantee. See the [diagnostic](deployment-diagnostic.md) and [deployment boundary](security-architecture.md#cloud-and-vps-deployments). |
| LMS/Academy — managed private video | PR #227 admitted the unpublished `rullst-media` candidate after hosted workspace/platform, browser and installed-archive acceptance. It implements Bunny lifecycle management, resumable upload, authoritative processing, private playback and deletion with SQLite recovery. | Preserve the supported journey in the final combined release campaign. Live-account interoperability stays unvalidated; release-inventory admission remains separate. See the [managed-video candidate](managed-video-roadmap.md). |
| M40 — Labs | Unpublished `rullst-labs` and `rullst-labs-runner` candidates provide encrypted durable exercises/jobs, exact grading, cancellation/recovery/retention and a separate Linux Rust/Wasmi executor. The named profile passed 26 ordinary and 26 instrumented hosted journey checks at `977e40a3`, including actual execution, compiler deadline/cleanup and recovery at full group capacity; the scoped coverage report was also generated. | PR #228 passed workspace/platform and actual extracted-package source admission, plus both 90% coverage floors. The non-required patch-coverage gap, independent isolation review and final release admission remain outstanding. See the [recorded profile evidence](labs-first-profile.md#recorded-linux-acceptance). |
| M15 — remote messaging | PR #236 admitted the optional standalone Redis Streams profile with TLS, restart/redelivery, fenced leases, exact replay, retry/DLQ and outbox evidence. | Preserve the documented Redis and operator boundaries in combined validation. Other broker adapters, native Redis group interoperability and replication/failover remain separate roadmap work. |
| M39 — optional Rullst Gateway | No `rullst-gateway` crate or executable exists. Keep the separate opt-in proxy/load-balancer design from the master roadmap; readiness helpers and deployment templates do not implement it. | Lower priority than supervision, shared passkey state, deployment acceptance, one remote broker and bounded Labs work. Reconsider when a concrete self-hosted need justifies implementation and operations; no September 26 delivery commitment. |
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

The owner reconfirmed this calendar on September 20: prioritize remaining
implementation through September 23 by user benefit, maintenance cost and
acceptance capacity; reserve September 24–25 for validation/corrections and
September 26 for final adjustments and protected publication. Local deployment
configuration diagnostics and the supervision observation extension are merged
with hosted checks passed. Their archive gaps have now been repaired as recorded
above.
The owner's subsequent direction selects the complete
supported Bunny journey next, followed by usable Labs execution and grading,
before remote messaging or other optional expansion. Bunny has passed source
admission; Labs has now passed source admission with its remaining quality,
independent-review and release requirements tracked above. Increasing the crate
count is not a priority.

The owner subsequently requested no manual or real-provider account testing in
this window, including the confirmed existing Bunny library. Continue automated
local/hosted tests, disposable databases and protocol fixtures. Preserve required
regression/security gates and report unvalidated provider interoperability
explicitly; do not describe simulated acceptance as a live integration pass.
New optional provider scope can be delivered with that stated limitation, rather
than waiting for manual tests or using the owner's live resources.

| Date | Checkpoint |
| :--- | :--- |
| 20 September | Close v12.1 documentation, admit the integration baseline, settle privacy/storage API decisions and prepare the v13 release/branch policy. |
| 21–22 September | Admit and repair the existing candidates; prioritize completing the Bunny lifecycle, then the Labs execution/grading journey as dependencies and verification capacity permit. Maintain migration fixtures and automated acceptance with each API change. Assess provider scope by the end of the 22nd using automated protocol evidence, without live-account testing. |
| 23 September | Close supported product journeys, recovery paths, generated guidance and documentation. Resolve the Bunny/Labs acceptance boundaries before considering another optional implementation. Freeze feature scope by the evening. |
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
