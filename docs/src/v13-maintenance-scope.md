# v13 maintenance scope and extension strategy

**Decision adopted on 22 September 2026:** concentrate Rullst's investment on
secure, productive application development and keep specialized capabilities
within explicit maintenance boundaries. Reduce scope before removing useful
software. Code generation by an AI assistant does not remove the need for
maintenance, integration evidence or security review.

This is an approved planning direction, not an implemented package split or a
deprecation notice. It leaves the published v12.1.1 release, supported APIs, defaults,
MSRV and current publication inventory unchanged. Specific extractions,
provider selections and retirements need their own reviewed changes. The
[SST](spec.md) describes the current architecture; the
[compatibility policy](compatibility-policy.md) governs transitions.

**Direction reaffirmed on 24 September 2026:** prioritize measured maintenance
cost and coherent application journeys before expanding the adapter catalogue.
The next planning deliverable is a decision record per integration, beginning
with Capital, specialized database adapters and the Labs runner. Retain useful
maintained implementations; an interface without a usable implementation can
transfer both development work and security mistakes to every application.

This direction can improve the conditions for broad adoption by concentrating
effort on reliability, predictable upgrades and developer experience. It is not
evidence of a global framework ranking. Use the
[comparative evaluation plan](v13-framework-comparison.md), independent feedback
and application adoption to assess progress. Feature count and mutation scores
alone do not establish superiority.

## How to assess maintenance cost

The assessments below are qualitative, not measured engineering hours or
monthly costs. They include unpublished v13 candidates. Review each capability
using its actual user journey and these recurring obligations:

- external API, protocol, operating-system and toolchain changes;
- supported database, platform, blueprint and feature combinations;
- consequences of security, data-integrity and recovery failures;
- dependency updates, diagnostics, migrations and documentation;
- repeatable acceptance and any outstanding provider or independent review;
- a named person or team able to review incidents and maintain the contract.

Source size and crate count alone are poor cost measures. A small untrusted-code
runner can require more operational assurance than a larger template library.
An optional dependency reduces the consumer's selected build surface; it does
not eliminate the project's support obligations. Moving code to another
repository saves work only when scope, ownership or release coupling changes.

For each integration, record its actual consumer, maintained operations,
dependency/API versions, compatibility matrix, validation gaps and accountable
maintainer. Measure update/review effort, regressions and CI execution separately
from queue time; leave unknown costs explicitly unknown. Recommend one of:

- retained official support with a bounded expansion scope;
- specialized optional extension with an explicit maintenance/release owner;
- an application/community integration with a tested contract and transition;
- deprecation with a replacement and compatibility plan, when justified.

Provider release frequency alone does not decide the category: a compatible
upstream release need not require an adapter rewrite, while a rarely changing
executor can still carry substantial security obligations. Evaluate independent
adapter releases where stable interfaces and dependency ranges allow them; a
provider correction should not unnecessarily force unrelated framework releases.
This requires an actual packaging, compatibility and release-workflow migration,
not just a new repository name. Security support for shipped capabilities remains
in force throughout that transition.

## Crate-level investment decisions

| Crate or area | Maintenance assessment | Adopted direction |
| :--- | :--- | :--- |
| `rullst-core`, `rullst`, `rullst-macros` | Essential shared surface with substantial compatibility and security impact. | Maintain explicit HTTP/lifecycle contracts, safe defaults, useful diagnostics and Axum/Tower interoperability. Keep specialized domains optional and avoid expanding the kernel into a general infrastructure platform. |
| `cargo-rullst` | High: generated applications multiply database, blueprint, platform and installer combinations. | Retain as a central productivity investment. Share templates, declare the supported matrix and validate complete generated journeys. Defer additional combinations until there is demand and acceptance capacity. |
| `rullst-orm`, `rullst-orm-macros` | High: transactions, schema changes, database differences and native dependencies. | Prioritize dependable relational persistence and migrations. Treat specialized stores as explicit adapters; do not promise universal parity or generic replication. |
| `rullst-auth`, `rullst-security`, `rullst-connect` | High and necessary: failures affect application identity, secrets and tenant boundaries. | Maintain narrow, coherent contracts using established primitives. Reduce duplicated responsibilities and close documented gaps before expanding providers or security-product ambitions. |
| `rullst-messaging` | High: leases, retries, duplicates, outbox consistency and backend recovery. | Maintain bounded durable delivery and supported backends. Additional brokers need a demonstrated application requirement and their own recovery evidence. |
| `rullst-capital` | High: provider-specific money/state semantics, reconciliation, subscriptions and fiscal expansion. | Preserve reusable SaaS billing. Separate common contracts, provider adapters and fiscal responsibilities as described below; do not discard the entire crate. |
| `rullst-mail`, `rullst-ai`, `rullst-media` | Medium to high: external services, feedback/events, transport changes and provider-specific behavior. | Maintain common contracts and adapters justified by actual use. Prefer interoperable protocols where appropriate, while retaining provider-specific assertions and explicit unsupported operations. |
| `rullst-privacy` | Bounded current foundations; broader legal or biometric promises would create high continuing cost. | Keep reusable consent, minimal-data and proportional age-policy mechanisms. Provider attestations remain separate from native declarations; global legal certification and a first-party facial model are not part of this direction. |
| `rullst-supervision` | Specialized, sensitive application state; models, capture and operational expansion raise the cost. | Keep transparent observation contracts as an optional education/parental extension. Product workflows, verified relationships, media models and human review belong to the application or separately governed integration. |
| `rullst-labs` | Bounded trusted orchestration with sensitive authorization, leases and grading contracts. | Keep separate from execution. Retain one useful Academy profile without making broader language or exercise support a framework release prerequisite. |
| `rullst-labs-runner` | Very high security and operational burden despite its small source size. | Maintain separate deployment and plan a separate release lifecycle. Require the outstanding independent isolation review; never move execution into the web process or weaken containment to simplify maintenance. |
| `rullst-studio`, `rullst-nexus` | Medium, increasing with operational dashboards and autonomous administration. | Preserve useful local diagnostics and authorized administration. Avoid growth into a complete security-operations or autonomous infrastructure product. |
| `rullst-iot` | The current helper surface is small; broad hardware and transport support would be expensive. | Preserve existing contracts and defer hardware expansion until there are named users, target devices and interoperability evidence. Treat future expansion as a specialized extension. |

These decisions refine the existing
[A/B priorities and overlapping C cost marker](v13-priorities.md).
They do not add milestones, change the 41-theme count or label an expensive
capability dispensable merely because it is expensive.

## Capital: retain billing, contain the responsibility

**Owner-selected v13 scope, 24 September 2026:** the official billing-provider
set is Stripe and Paddle. Crypto integrations are excluded from this release.
Stripe already serves the maintainer's SaaS journey; Paddle's
merchant-of-record subscription offering addresses another requested product
need. Keep useful checkout, subscription and authenticated-event behavior
alongside the common billing contracts, within their supported and validated
scope. Cost control must not leave every developer to implement payment
protocols.

This is the target release scope, not a claim that the existing adapters have
already been removed. Plan the compatibility transition for other providers
before changing APIs, facade features, CLI choices, generated applications or
dependencies. Preserve the published v12 support commitments. Do not silently
redirect existing customers/subscriptions to another provider.

| v13 provider | Reason and current boundary |
| :--- | :--- |
| Stripe | Existing SaaS use and the generated durable billing integration. Preserve the reviewed operation boundaries rather than promising every Stripe product. |
| Paddle | Subscription billing with provider-managed merchant-of-record responsibilities. Typed checkout, signed subscription contracts, a bound portal and a generated durable candidate cover the limited recurring flow. Live provider interoperability remains unvalidated. |

Keeping these two adapters still incurs upstream API and security maintenance.
The framework owns bounded protocol validation and reference lifecycle tests;
applications own credentials, catalog setup, billing-owner permissions, business
rules and deployment-specific operation. An adapter is not a promise to implement
every provider product. Generated reference flows must reuse storage/security
foundations and retain explicit limits, without growing into a full billing
platform. Capital remains an optional feature. An application-specific need
does not automatically justify another official adapter.

[Paddle's SaaS documentation](https://developer.paddle.com/get-started/how-paddle-works/saas/)
describes subscription lifecycle, customer self-service and sales-tax handling
under its merchant-of-record model. These provider capabilities are not a claim
that every operation is implemented in Rullst or that the application inherits
universal legal compliance.

Crypto research is deferred beyond v13. Preserve applicable billing extension
contracts, but do not add a crypto adapter, universal blockchain API, custody
service or new crypto-specific release gate. The existing Commerce code still
requires the same reviewed compatibility transition as other excluded
providers. Two findings remain relevant if a future application requests this:

- Coinbase's [Commerce transition notice](https://help.coinbase.com/en/transitioning-from-coinbase-commerce-to-coinbase-business)
  sets 31 March 2026 as the Commerce shutdown deadline. Its
  [Business availability page](https://help.coinbase.com/en/coinbase/other-topics/business/business-overview)
  currently lists eligible receiving businesses in the United States and
  Singapore. Existing Commerce code is not a Business API implementation.
- [BTCPay Server](https://docs.btcpayserver.org/FAQ/General/) is an optional
  evaluation candidate, not a selected Rullst provider. Its
  [altcoin FAQ](https://docs.btcpayserver.org/FAQ/Altcoin/) separates the core
  Bitcoin scope from community integrations. Its
  [August 2026 advisory](https://blog.btcpayserver.org/security-advisory-btcpay-server-2-4-2/)
  records an exploited LND credential vulnerability fixed in 2.4.2. This dated
  reference does not identify the latest release or certify another deployment.
  Future evaluation must include current advisories, exact plugin/engine versions
  and operator maintenance, not merely API access.

Plan three explicit responsibilities, using the existing implementation as the
starting point rather than rewriting it:

1. **Reusable billing contracts:** amounts/currencies, supported checkout and
   subscription state, current entitlements, idempotency and safe event
   processing. Keep authorization and money/state integrity reusable instead
   of asking every generated application to reconstruct them.
2. **Provider adapters:** document capabilities per operation and provider,
   version assumptions, signature verification, duplicate/out-of-order events,
   ambiguous outcomes and reconciliation. Select the actively supported
   expansion scope from real product demand and available validation. A common
   trait must not imply that every provider supports the same operations.
3. **Fiscal and specialized finance:** prepare a separate optional domain and
   maintenance lifecycle for fiscal preparation and other specialized work.
   Existing NFS-e preparation is bounded; live transmission and fiscal
   authorization remain disabled pending their required external validation.

These responsibilities need not produce one crate per provider. Decide whether
modules, features, packages or repositories provide a measurable benefit after
checking dependencies, downstream consumers and release costs. Independent
versioning is a future migration decision, not a property already implemented.

No gateway is retired by this document. Existing supported behavior keeps its
maintenance obligations under the published support policy. Before narrowing
support, identify affected users, a replacement, a responsible maintainer and
the compatibility/migration path. Tests with deterministic mocks establish local
contracts, not actual provider interoperability. The prohibition on real-account
tests remains in force; outstanding external evidence stays explicitly pending.

The recurring obligations are concrete: Stripe documents duplicate events,
delivery ordering and retries in its [webhook guide](https://docs.stripe.com/webhooks),
and version transitions in its [API upgrade guide](https://docs.stripe.com/upgrades).
Other providers require their own evidence rather than inferred equivalence.

## Turso: assess a complete application journey before universal parity

**Assessment on 24 September 2026; implementation is not scheduled here.**
SQLite, PostgreSQL and MySQL/MariaDB are the principal SQLx ORM backends.
MariaDB shares the MySQL driver but has its own executable container contract.
This does not imply every optional crate or generated application supports all
four databases: some domain stores deliberately have a SQLite-only or
PostgreSQL-specific implementation.

The current Turso/libSQL path already supplies typed CRUD, equality filters,
ordering, bounded pagination/counts, explicit SQL transactions and reversible
checksummed migrations. Blank/API generation and selected generators have
bounded acceptance; the remote matrix runs against an official libSQL server.
SQLx-specific relations/hooks, schema auto-diff, seed generation, full blueprint
parity and transparent replica synchronization remain outside that profile.
See the [current persistence contracts](polyglot-persistence.md).

The remote adapter owns an HTTP/Hrana protocol implementation; local offline
execution uses SQLite through SQLx. Reusing SQL syntax does not make those
transports, row codecs, transactions or generated consumers interchangeable.
The current [official Rust guide](https://docs.turso.tech/sdk/rust/quickstart)
also distinguishes libSQL, the newer Turso Database engine, remote clients and
local-first sync. The existing libSQL evidence does not validate every newer
engine or sync configuration. Select an engine/protocol/version profile before
expanding support, and preserve Rust 1.96.0 compatibility.

| Potential scope | Qualitative maintenance assessment |
| :--- | :--- |
| Maintain the current bounded libSQL profile | Moderate: existing transport, macro, migration and local/remote acceptance obligations remain. |
| Complete one named SaaS journey on one selected profile | Medium to high initial effort; recurring work can be bounded by an explicit operation and platform matrix. Inventory actual Auth, billing, tenant, migration and recovery dependencies first. |
| Match every ORM feature, domain store and blueprint across remote/local/replicated modes and both engine families | High to very high: multiplies implementation and recovery/consistency validation, not just connection strings. |

These are engineering judgments from current boundaries, not measured hours or
update-frequency forecasts. Retain useful Turso support. If an actual SaaS or
Academy requirement selects expansion, implement the smallest complete journey
and review its recurring cost before adding another profile. Validate atomic
updates/rollback, tenant isolation, concurrency, uncertain network outcomes and
schema changes. Keep replica freshness and authoritative authorization/payment
decisions explicit. Local/container evidence remains distinct from managed
Turso Cloud acceptance; real-account tests are still prohibited in this session.

## Education, supervision and Labs remain supported use cases

Rullst remains a foundation for complete education, parental-supervision and
exercise platforms. Preserve the useful domain capabilities already implemented
in the optional packages, alongside authentication, tenant boundaries, storage
and durable work. Their current candidate status and evidence limits still
apply; this decision does not promote unpublished code to a stable guarantee.

| Layer | Responsibility |
| :--- | :--- |
| Framework foundations | Reusable identity, authorization helpers, persistence, queues, request protections and explicit extension contracts. |
| Optional domain packages | Existing Supervision observation/session/collection contracts and Labs exercise/job/lease/grading contracts, maintained within their declared boundaries. |
| Application | School or family workflows, authenticated membership and verified relationships, user interfaces, review decisions, exercise content, grading policies and model/provider selection. A developer or assistant may implement these through the documented contracts. |
| Separately deployed execution | A reviewed runner and operator-owned isolation/resources for untrusted submissions. The application must not improvise execution in the web process. |

Keep small tested examples and supported integration paths where they serve
actual users. Developers should be able to compose existing capabilities rather
than recreate sessions, leases or observation protocols. They still own domain
authorization and deployment obligations. Observation signals do not establish
cheating, and an AI-written integration does not prove model accuracy or safe
code execution. The outstanding Labs isolation review remains required.

### Labs and the runner have different maintenance responsibilities

`rullst-labs` keeps the trusted exercise, authorization, submission/job,
cancellation, receipt-verification and grading contracts. It does not execute
the learner's program. A runner is the separately deployed program that takes
authorized work, compiles/executes the submitted code under enforced limits,
returns a bounded authenticated result and tears down the execution environment.
Trusted grading and expected answers remain outside the untrusted worker.

Two kinds of separation must not be confused:

- **Execution isolation is mandatory today.** Student-controlled code can loop,
  exhaust resources or attempt to read data. Run it in the separately restricted
  worker without application secrets and enforce CPU, memory, time and output
  limits. Merely putting code in another crate or process is not adequate
  containment; the runner's full observed isolation controls remain required.
- **Independent maintenance/releases are a proposed governance choice.** The
  Linux/compiler/sandbox matrix has different deployment and review obligations
  from an ordinary web library. Its updates need not force unrelated consumers
  to change. This can remain an official Rullst tool in the same repository;
  neither removing the runner nor moving repositories is required by this plan.

There is no release-frequency evidence that the runner changes more often than
the framework: it is still unpublished. Both Labs packages currently declare
`13.0.0-alpha.1` with publication disabled and are not part of the published
12.1.1 release. Keep that candidate versioning until a reviewed migration is
needed. Independent patch releases can retain a shared major version; they do
not require inventing an unrelated numbering scheme. Any future separation
needs an explicit Labs/runner protocol and package compatibility matrix plus
installation/upgrade tests. Some operating-system image updates may instead
change a pinned deployment image without changing Rust source. Isolation
evidence must match the deployed toolchain and image, regardless of versioning.

Retain the bounded Labs foundation. For the existing experimental runner,
recommend a separately maintained optional implementation with its own release
and security-review responsibilities, rather than requiring every application
developer to write a sandbox. The application operator still configures and
operates its infrastructure. A future third-party executor would require an
adapter and validation of the same authorization, fencing, result and isolation
requirements; interchangeable external executors are not implemented today.

The current first profile is a restricted Rust pure-function exercise compiled
to Wasm on Linux. It does not run arbitrary Rullst projects or support every
language. Labs and its runner remain unpublished candidates outside the normal
publication inventory, and the runner still needs independent isolation review.
If a sustainable maintainer cannot be assigned, freeze runner expansion and
document its experimental status rather than imply production support or ask
applications to recreate isolation with an unrestricted process launcher.

## What belongs in the framework in an AI-assisted workflow

Prioritize explicit APIs, typed errors, safe defaults, version-matched executable
documentation, inspectable generators and predictable upgrades. Keep reusable
authentication, tenant authorization, payment verification and data-integrity
contracts centrally tested. AI-generated application code must still satisfy
those contracts; an assistant is not a replacement for their review.

Application-specific forms, reports, pages and business workflows are good
candidates for generated or manually written application code. Provide extension
points and small tested examples rather than maintaining every possible product
inside the framework. Application owners still maintain the resulting code.
See the [AI maintainability roadmap](ai-maintainability-roadmap.md) for proposed
task fixtures and evaluation boundaries; this decision claims no model benchmark.

## Transition sequence and acceptance

1. **Preserve stable maintenance.** Keep the published v12.1.1 release immutable
   and finish the active test-hardening follow-up without structural changes.
   Keep `v12` for stable maintenance and
   `main` for v13. Continue assessing applicable fixes in both directions.
2. **Make support scope observable.** For each proposed expansion or extraction,
   record the product user, maintained operations/platforms, dependencies,
   evidence gaps, responsible reviewer and estimated recurring work. Start with
   current SaaS and Academy journeys. The selected v13 billing-provider set is
   Stripe and Paddle, with crypto excluded. Other current providers need a
   reviewed compatibility transition; the selection does not remove code or
   waive current validation requirements.
3. **Prioritize separation and expansion freezes.** Assess the Labs runner's
   independent lifecycle first, then IoT hardware expansion, specialized
   educational monitoring and Capital's fiscal domain. Keep their existing
   bounded implementations and security duties visible. Reuse existing package
   boundaries where they already solve the problem.
4. **Review one bounded migration at a time.** Demonstrate the benefit before
   extracting modules or changing defaults. Trace facade features, generated
   applications, direct package consumers, serialization and data migration.
   Do not create a collection of nominally separate but unsupported packages.
5. **Validate and integrate normally.** Preserve workspace tests, strict Clippy,
   formatting, MSRV, feature-boundary, package-consumer and generated-application
   checks. A smaller future support matrix requires an explicit compatibility
   decision; it is not permission to skip today's required evidence. Update the
   SST, manifests, release inventory, support documents and migration guide
   together when an actual boundary changes.

The current synchronized release train remains authoritative. Labs and its
runner are already excluded from normal publication; other crates in the
inventory are not removed by calling them extensions. Their possible future
independent release lifecycles must be explicitly implemented and validated.
Unfinished optional roadmap work need not block a bounded v13 release, but
unresolved safety or admission requirements for what it ships still do.

Retirement is appropriate only after a concrete review finds insufficient
benefit, no sustainable maintainer or a better supported replacement. Follow
the existing deprecation policy: retain a deprecated stable API for at least
one released minor version before removal in the next major, subject to the
documented security exception. Preserve migration guidance and history.

At each review, record actual maintenance effort where available: incident and
dependency-update work, CI execution versus queue time, repeated failures,
supported combinations and downstream acceptance. Keep unknown measurements
unknown. Prefer freezing low-demand expansion over weakening required tests or
transferring critical security logic into unreviewed application code.
