# Sustainable v12 maintenance and v13 priorities

## Direction revised on 22 September 2026

The owner prioritizes a maintained, secure stable v12.1 line and useful v13
deliveries over publishing a major version merely to meet September 26. Work
may continue in later funded sessions. September 26 is a handoff/review point,
not a mandatory release date. An alpha is an optional evaluation milestone,
not a substitute for safety, packaging or publication checks.

Keep `main` for v13 development and `v12` for stable maintenance. Compatible
bug/security/dependency fixes can become 12.1.x after validation; new compatible
public functionality may require a minor release, and breaking changes remain
in the major development line. A dependency's SemVer number alone does not
prove compatibility. Preserve the supported compiler, features, serialized
contracts and generated-project behavior. Do not backport v13 wholesale.

This document proposes investment priorities, not approval to implement every
row, publish, expand supported platforms or claim a permanent LTS programme.
Actual release status and support remain governed by [SECURITY.md](../../SECURITY.md)
and the [compatibility policy](compatibility-policy.md).

The owner subsequently adopted the
[crate maintenance and extension strategy](v13-maintenance-scope.md) on
September 22. It turns the cost assessment into a direction for scoped
investment: preserve common security/productivity contracts, contain Capital's
provider and fiscal scope, and give specialized extensions explicit ownership
and migration boundaries. It does not remove supported capabilities or change
the v12.1.1 candidate. Specific restructuring remains follow-up work.

## Counting method

The [canonical roadmap](../../ROADMAP.md#executive-milestone-tracker) contains
**41 top-level milestones, M1–M41**. Their detailed per-crate roadmaps overlap
and decompose these themes; adding every checkbox would double-count work.
This is a prioritization of those 41 themes, not a fresh audit of every source
file or a claim that all of them were committed for 13.0.0.

| Proposed category | Themes | Existing roadmap labels |
| :--- | ---: | :--- |
| A — invest and maintain | 25 | 5 bounded implemented; 20 partial |
| B — defer expansion until justified | 16 | 8 partial; 8 not implemented |
| Total | 41 | 5 bounded implemented; 28 partial; 8 not implemented |

Use a separate **C — high maintenance cost** marker across either category.
The initial review flags 12 existing implementation themes below (11 in A and
1 in B). C overlaps A/B; it is not another twelve items and does not mean the
feature should be removed. Cost assessments are architectural estimates, not
measured monthly spending or guarantees about future contributor availability.

M31 is a separately governed aerospace/autonomous/defence programme. Excluding
it gives **40 framework themes: 25 A and 15 B**, matching the master roadmap's
40-theme horizon. A partial theme may include substantial working software and
years of optional extensions. Counts do not measure effort or completion
percentage. Supervision is cross-referenced under the education/privacy themes;
Media belongs to M17. They are not added again as duplicate milestones.

The assignment concerns the *next investment*. Existing supported functionality
still receives defect/security maintenance even when its expansion is in B.
For broad A themes, only the bounded scope below has priority; selecting a row
does not select every research idea in its original wording.

## A — invest and maintain

| ID | Priority scope | Why it is worth the maintenance cost |
| :--- | :--- | :--- |
| M1 | CLI and supported generator/blueprint combinations | A broken generated application prevents adoption; preserve real compilation and behavior checks. |
| M2 | Measured build/reload improvements | Reduce repeated developer/CI cost; keep supervised restart and defer a stable dynamic Rust ABI. |
| M3 | Feature boundaries, escape hatches and migrations | Keep upgrades reversible and optional capabilities out of small applications. |
| M4 | Resource generator and local error console | Maintain the bounded existing implementation; autonomous modification belongs to deferred M37. |
| M5 | Navigable, executable documentation and typed API contracts | Users need accurate examples and errors more than additional undocumented surfaces. |
| M6 | Reliable relational ORM, transactions and supported adapters | Prioritize data integrity and recovery; do not bundle transparent replication into this scope. |
| M9 | Sessions, account recovery, passkey assurance and revocation | Authentication failures directly affect users; close named security/conformance gaps before enlarging identity scope. |
| M10 | Validation, bounded requests, mail and shared rate limits | Improve common SaaS paths with reusable contracts and explicit backend failures. |
| M11 | SaaS administration and supported billing journeys | Stabilize actual Academy/SaaS needs; do not equate this with universal gateways or the whole Omni vision. |
| M12 | Security, tenant isolation, secrets and supply-chain assurance | Continuous prevention and repair have broad value; additional tools need a concrete uncovered risk. |
| M14 | Accessible HTMX/SSR and clear frontend interoperability | Improve the primary supported experience before full Leptos/Dioxus integration. |
| M15 | Durable jobs, scheduling and one supported remote broker | Finish recovery, retries, quotas and operations before adding a broker catalogue. |
| M17 | Private files, resumable upload and course video | Directly serves SaaS/Academy; retain access, expiry and recovery checks. A package marketplace is not part of this priority. |
| M18 | Recoverable server-driven UI | Maintain the admitted authorization/reconnect journey before adding a new rendering ecosystem. |
| M19 | Honest runtime telemetry and standard exports | Preserve implemented diagnostics; unavailable sources must not become fabricated values. |
| M26 | Safe deployment guidance and read-only diagnostics | Help operators find configuration mistakes and rehearse rollback without automatic infrastructure mutation. |
| M27 | Readiness, draining and deployment scaffolds | Maintain the bounded implementation and actual proxy/application contracts. |
| M28 | Existing typed dependency injection | Keep its API small and tested; no new DI framework is necessary for release. |
| M29 | Accurate OpenAPI and its playground | Prefer schema/transport fidelity over adding UI controls around incomplete contracts. |
| M32 | Axum/Tower interoperability and precise macro errors | Preserve existing escape hatches and diagnosability. |
| M33 | Current-state SaaS entitlements | Access must reflect current account, tenant, plan and payment state; syntactic convenience comes later. |
| M34 | One dependable TypeScript SDK profile | Maintain serialization and migration evidence; additional React/Dart/Swift targets require users and tests. |
| M35 | Standard trace propagation/export and useful diagnosis | Build on existing OpenTelemetry collectors; a custom durable observability service is not a prerequisite. |
| M40 | One bounded Academy Labs profile | Product value is concrete, but untrusted execution needs its outstanding independent isolation review. Keep the runner separate and unpublished until admitted; Labs need not block the rest of v13. |
| M41 | Practical privacy and proportional age policies | Preserve minimal data, explicit decisions, consent and rights workflows. Supervision remains transparent and scoped; declarations/signals do not prove age, guardianship or cheating. |

M4, M19, M27, M28 and M32 are the five already-implemented labels. The other
twenty A themes need scoped decisions, maintenance or further work; none is a
blank cheque to finish all of its historical ambitions.

Within A, use this order: security/data-loss and stable regressions first;
authentication, tenant boundaries and billing correctness next; migrations,
recovery and usable documentation next; then product expansion supported by
Academy/SaaS demand. Run one bounded feature effort at a time alongside the
stable maintenance lane. Reassess after each completed journey.

The [Nexus application integration plan](nexus-integration-plan.md) refines M11
with host-session authorization, read-only capabilities, strict-CSP assets,
localization and explicit data ownership. It also identifies a separate M32
boolean-attribute ergonomics proposal. These are scoped follow-ups within the
existing counts, not delivered APIs or additional top-level milestones.

## B — defer expansion until justified

| ID | Deferred expansion | What would justify revisiting it |
| :--- | :--- | :--- |
| M7 | Portable edge runtime and distributed-data expansion | A named deployment target and application need, with vendor-specific consistency and rollback evidence. |
| M8 | Intent-based modelling and index recommendations | A measured database problem; begin with explainable, reviewed advice, never unattended production DDL. |
| M13 | Post-quantum web architecture and plugin containment | A concrete protocol/threat model, maintained audited primitives and an interoperability programme. |
| M16 | Wasm islands and broad hydration/component packaging | A user journey that SSR/HTMX cannot reasonably meet and a supported browser/toolchain matrix. |
| M20 | New immutable ledger/event-streaming engine | A consistency/recovery requirement unmet by established storage, plus a funded maintenance owner. |
| M21 | Broader Omni/mobile/offline platform | A real mobile product with physical-device, secure-key, networking and store acceptance resources. Keep existing artifact verification maintained. |
| M22 | Agentic infrastructure provisioning | A named operator need and reviewed plans, scoped credentials, audit and rollback; advisory diagnostics already fit A. |
| M23 | Auto-healing production code or databases | A safely bounded, approved repair contract; no general autonomous mutation promise. |
| M24 | Additional IoT transport/firmware/hardware integrations | Selected boards, actual devices and transport/rollback interoperability tests. Preserve existing no_std and manifest contracts. |
| M25 | Embassy integration | Stable hardware/transport boundaries and a maintained target application. |
| M30 | Dedicated gRPC crate and broader scaffolding | A concrete service consumer and protobuf compatibility matrix; the existing generator still receives fixes. |
| M31 | Aerospace, autonomous vehicles and defence | A separate project with domain expertise, governance, hardware and applicable certification. |
| M36 | Natural-language database copilot | Concrete operator demand and a read-only, bounded, explainable contract; production writes remain separate. |
| M37 | AI autofix workflow | A confined local patch/review/test/rollback journey with demonstrated value over existing development tools. |
| M38 | Generic local read replicas and background replication | A named database's supported semantics and recovery guarantees; no generic transparent-replication claim. |
| M39 | Rullst's own gateway/load balancer | A measured limitation in an established proxy and resources to maintain a separate networking product. |

“Deferred” means lower expected return under the current product, budget and
validation constraints, not that the idea is intrinsically bad. An explicit
user requirement can change this ranking. Unsupported guarantees such as
automatic worldwide legal compliance, zero leakage or general autonomous safe
mutation are not future deliverables to promise at any budget.

## C — high maintenance cost, including existing implementations

The adopted [crate-level decisions](v13-maintenance-scope.md#crate-level-investment-decisions)
refine this theme-level assessment without changing its counts. Their
[transition sequence](v13-maintenance-scope.md#transition-sequence-and-acceptance)
governs future scope reductions and extractions.

The source already contains at least a bounded implementation in each theme
below. “Existing” includes unpublished v13 code and does not imply a complete
milestone or an available crates.io release. The primary repeated costs are
provider drift, supported combinations, security review and recovery evidence,
not merely writing the first implementation.

| Theme | Existing surface requiring care | Recurring cost and how to contain it |
| :--- | :--- | :--- |
| M1 — CLI/scaffolds | Multiple blueprints, database selections and installed generators | Changes multiply into generated applications. Maintain a declared supported matrix and shared templates; add combinations only for a user need. |
| M2 — build/reload | Cross-platform process supervision, filesystem behavior and compiler/toolchain integration | OS-specific regressions and repeated native compilation. Preserve reproducible targeted tests and measure build versus queue time. |
| M6 — persistence | Relational and heterogeneous database/native adapters | Different transactions, types, consistency and native libraries. Keep adapter boundaries explicit and avoid universal parity promises. |
| M9 — authentication | Passwords, sessions, recovery, tokens and passkey state | Security-sensitive lifecycle, standards and negative/concurrent cases. Prefer maintained primitives and one coherent supported identity path. |
| M10 — mail/protection | Multiple external transports, provider feedback and shared limits | Provider schemas, auth, quotas and delivery behavior change. Use common contracts while retaining provider-specific assertions and capability limits. |
| M11 — payments/SaaS | Multiple payment adapters, signed webhooks, subscriptions and entitlements | Remote semantics, API changes, retries and money/state reconciliation. Prioritize used methods and never claim untested uniform gateway coverage. |
| M12 — security/assurance | WAF/DLP, tenant policy, secrets, audits and verification tooling | Threat/advisory changes and tooling/fixture upkeep. Keep a risk-to-test map and prevent duplicated or unbounded campaigns. |
| M15 — async delivery | Local/durable queues, retry/leases/outbox and Redis profile | Failure ordering, replay, shutdown and backend recovery. Finish one durable journey before broadening brokers. |
| M17 — files/video | Private storage, multipart and Bunny browser/provider lifecycle | Upload interruption, signing/expiry, notifications, SDK and browser changes. Bound supported protocols and separate fixture evidence from live interoperability. |
| M21 — native delivery | Desktop/mobile scaffolding and Android signing/artifact verification | SDK, OS, signing and device changes. Maintain current guarantees but defer broader device/offline promises without test resources. |
| M40 — Labs | Trusted orchestration and separately isolated compiler/runner | Untrusted execution, OS/toolchain changes and independent review. Keep one narrow profile and a separate lifecycle; never trade containment for convenience. |
| M41 — privacy/supervision | Age/consent/replay and transparent observation contracts | Sensitive state, retention, authorization and evolving product/legal requirements. Minimize data and keep application/jurisdiction obligations explicit; no automatic legal certification. |

PQC, embedded hardware, autonomous production changes, generic replication and
a new gateway also have potentially high *future* maintenance costs. Their B
status means those costs are not accepted merely by keeping the ideas on a list.
Other themes still need maintenance; omission from C does not label them free
or low-risk.

Before enlarging a C surface, record a named product need, supported boundaries,
repeatable acceptance, a person/team responsible for reviewing incidents and
updates, and a fallback if that support is unavailable. Prefer freezing new
scope or deprecating with migration notice over silently abandoning a supported
capability. A feature's past implementation cost is not a reason to keep adding
expensive options without users.

## Eight concrete follow-up suggestions

These are proposed **subtasks**, several already implicit in the detailed
roadmaps. They do not create eight additional top-level milestones and are not
automatically authorized implementation work.

| Proposal | Category and parent | Bounded useful result |
| :--- | :--- | :--- |
| S1 — downstream regression fixtures | A; M1/M3/M11 | Compile and exercise sanitized SaaS/Academy fixtures against a candidate release, including upgrades and denied access, without production data or provider accounts. |
| S2 — application recovery rehearsal | A; M6/M11/M15 | Restore a disposable application snapshot and verify accounts, tenant isolation, durable jobs and billing idempotency. Document quiescence, keys and restore order; this does not become a managed backup service. |
| S3 — reusable authorization test scenarios | A; M9/M12 | Offer a small application-test recipe or helper for anonymous/user/admin and two-tenant negative cases, using actual routes. Do not treat a static scanner as proof. |
| S4 — resumable maintenance handoff | A; M1/M3/M12 | Keep a concise source-bound receipt of supported versions, unresolved findings, exact CI evidence, next commands and session/build cost. Reuse existing reports so later, less frequent sessions need not repeat the investigation. |
| S5 — first-party facial age engine | B; M41 | Consider only with a suitable dataset, accuracy/bias and attack evaluation, capture/privacy controls, alternative paths and a long-term maintainer. Continue policy/attestation support independently. |
| S6 — additional Labs languages | B; M40 | Add one demanded language only after the first profile's isolation review and operational acceptance, with separate toolchain/resource/threat tests. |
| S7 — accessible, localized primary journeys | A; M5/M14 | Exercise keyboard focus, form errors, captions and translation/locale boundaries on the chosen auth, checkout and learning journeys. Reuse the existing LMS groundwork; automated checks do not establish universal accessibility or translation quality. |
| S8 — per-tenant resource budgets | A; M10/M11/M15 | Define explicit quotas/backpressure for stored bytes, concurrent jobs, mail and AI usage, with atomic claims and visible rejection. Extend existing bounded contracts only where an application gap is demonstrated; no automatic provider-bill guarantee. |

S4 and stable defect/compatibility maintenance are the immediate cost-control
priorities. S1–S3 should be selected by an observed gap, not implemented as
duplicate test frameworks. S5 and S6 illustrate valuable possibilities with
substantial continuing costs; neither belongs on the release critical path.
S7/S8 can add practical user value without another standalone subsystem, but
must have a bounded accepted contract before implementation. These proposals
comprise **six A subtasks and two B subtasks**. S5/S6 would also carry C; S1/S2/S8
can become expensive if allowed to grow into an unbounded platform matrix.

Do not report “49 independent milestones”: the eight proposals refine the
existing 41. Promote a truly distinct need to a new canonical milestone only
after checking overlap and documenting who will use and maintain it. A richer
framework delivers reliable useful journeys; raw crate or checkbox counts are
not the success criterion.

## Comparative evaluation adopted on 23 September

Use the [framework reference and evaluation plan](v13-framework-comparison.md)
to study Loco/Laravel productivity, Django security/documentation and Axum
composition, with other frameworks as targeted references. Measure development
effort, failures, performance and maintenance on equivalent application
journeys. This refines existing A themes and S1-S3; it does not add milestones
or change the 25/16 counts.

The 25 A themes are not prerequisites for the first comparison. After the active
v12.1.1 maintenance work, start with a small usable SaaS journey, establish a
baseline and use the findings to choose subsequent improvements. Broader
evaluation can accompany later v13 checkpoints. Keep the same acceptance
contracts, record unfavorable and inconclusive results, and distinguish
AI-assisted measurements from independent user evidence. Only free resources
and disposable offline fixtures are authorized; no superiority is established
by this planning decision or the internal quality scorecard.

## What makes v13 ready

Do not wait for all 41 themes, all 25 A themes or a percentage of this list.
Choose a bounded release contract, document intentional breaking changes and
prove its supported user journeys, migration/recovery, security, packaging and
protected publication. Existing admitted work should be completed and hardened
before unrelated expansion. Any deferred capability stays disabled or clearly
scoped; a known security or data-loss defect is not an optional future feature.

An optional `13.0.0-alpha.N` can collect explicit evaluation feedback while
stable users remain on supported v12. It does not resolve registration,
independent-review or safety blockers. Further compatible capabilities can ship
in 13.1 and later minors; unfinished roadmap ideas do not all require v14.

Before an extended pause, leave stable maintenance and candidate code with
their actual evidence, pending jobs and next actions recorded. Prefer a small
recoverable checkpoint over another half-finished subsystem. This approach
supports intermittent work without promising continuous model availability or
unfunded indefinite maintenance.
