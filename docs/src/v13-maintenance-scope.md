# v13 maintenance scope and extension strategy

**Decision adopted on 22 September 2026:** concentrate Rullst's investment on
secure, productive application development and keep specialized capabilities
within explicit maintenance boundaries. Reduce scope before removing useful
software. Code generation by an AI assistant does not remove the need for
maintenance, integration evidence or security review.

This is an approved planning direction, not an implemented package split or a
deprecation notice. It leaves the v12.1.1 candidate, supported APIs, defaults,
MSRV and current publication inventory unchanged. Specific extractions,
provider selections and retirements need their own reviewed changes. The
[SST](spec.md) describes the current architecture; the
[compatibility policy](compatibility-policy.md) governs transitions.

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

1. **Preserve stable maintenance.** Complete the existing v12.1.1 review work
   without adding structural changes. Keep `v12` for stable maintenance and
   `main` for v13. Continue assessing applicable fixes in both directions.
2. **Make support scope observable.** For each proposed expansion or extraction,
   record the product user, maintained operations/platforms, dependencies,
   evidence gaps, responsible reviewer and estimated recurring work. Start with
   current SaaS and Academy journeys; no provider shortlist is selected here.
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
