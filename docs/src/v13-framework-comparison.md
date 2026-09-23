# v13 framework references and comparative evaluation

**Planning direction adopted on 23 September 2026.** Study useful designs from
other frameworks and measure comparable application journeys to guide Rullst
investment. This document specifies future evaluation, not measured results or
a claim that Rullst is superior. Finish the active v12.1.1 maintenance admission
before beginning this work. The [SST](spec.md) still governs architecture.

## Relationship to the priorities

The [25 A priorities](v13-priorities.md) are broad themes to invest in or maintain,
not 25 unfinished features that must all ship before evaluation. Some already
have bounded implementations; others contain substantial optional follow-up.
Start a small baseline once the selected application journeys are usable, then
repeat relevant measurements after meaningful changes. Findings should help
select the next investment instead of arriving only after the entire roadmap.

This work refines existing themes such as M1/M2/M3/M5/M9/M11/M12/M15 and the
downstream fixtures, recovery and authorization proposals S1-S3. It adds no
top-level milestone and does not change the 41-theme or 25/16 counts. It is not
a new release gate, a promise to complete all themes for v13, or authorization
to publish. Preserve the [maintenance scope](v13-maintenance-scope.md): common
security contracts remain maintained, while specialized extensions need owners.

## Reference frameworks

The following official documentation was consulted on 23 September 2026.
These are design-study recommendations, not a ranked list or independent
verification of the projects' claims. Pin actual versions when executing a
comparison; mutable documentation links alone do not identify tested source.

| Reference | Study focus | Rullst application |
| :--- | :--- | :--- |
| [Loco](https://loco.rs/docs/) | Integrated Rust models, controllers, authentication, mail, workers and CLI | Closest initial application-productivity comparison; evaluate complete generated journeys. |
| [Laravel](https://laravel.com/starter-kits) | Authenticated starter applications and a coherent initial experience | Reduce manual setup and improve the supported SaaS path. |
| [Django security](https://docs.djangoproject.com/en/stable/topics/security/) and [admin](https://docs.djangoproject.com/en/stable/ref/contrib/admin/) | Integrated protections, documented limitations and administration | Review safe defaults, authorization recipes, documentation and Nexus boundaries. |
| [Axum](https://docs.rs/axum/latest/axum/) | Explicit HTTP APIs and Tower composition | Technical baseline for overhead and interoperability; Rullst already depends on Axum. |
| [Actix Web](https://actix.rs/docs/server/) | HTTP workers, connections and graceful shutdown | Targeted server lifecycle and load-behavior comparison. |
| [Ruby on Rails](https://guides.rubyonrails.org/getting_started.html) | Coherent generators, models, migrations and deployment journey | Learn from conventions while preserving explicit, inspectable Rust contracts. |
| [ASP.NET Core](https://learn.microsoft.com/en-us/aspnet/core/test/integration-tests?view=aspnetcore-10.0) | Application-host and infrastructure integration testing | Make realistic HTTP, identity and persistence acceptance easier to run. |
| [Phoenix LiveView](https://hexdocs.pm/phoenix_live_view/welcome.html) | Interactive server-rendered applications | Study the supported realtime/UI journey when product demand selects it. |
| [Leptos](https://book.leptos.dev/) | Reactive and full-stack Rust interfaces | Evaluate a specific frontend requirement before expanding beyond SSR/HTMX. |

Begin with Loco for comparable Rust application work, Laravel for onboarding,
Django for security/documentation study, and Axum for a small HTTP baseline.
The other references are targeted follow-ups. Do not build and permanently
maintain nine parallel application suites merely because nine references are
listed. Adoption of an idea requires a demonstrated gap and a bounded change;
the list does not authorize copying each framework's architecture or features.

## First application journeys

Use synthetic data, disposable databases, local provider fixtures and supported
public APIs. Reuse existing SaaS/Academy regression fixtures where appropriate.
Write equivalent acceptance contracts before timing implementations.

| Journey | Required observations |
| :--- | :--- |
| Create a small SaaS application | Install, generate, migrate, start and implement an authenticated resource; reject anonymous and cross-tenant access and prove allowed access. |
| Recover an account | Capture mail locally; exercise expiry, purpose binding, single use, concurrent reuse and the documented session policy. |
| Process a billing event | Use the same signed offline event contract; reject tampering, replay and cross-tenant mutation, and handle duplicate/out-of-order delivery without duplicate domain effects. This is not live gateway certification. |
| Recover background work | Interrupt a disposable worker, restart it and check leases, retries, idempotency and documented recovery behavior. Record delivery guarantees explicitly. |
| Update an existing application | Upgrade a pinned previous supported version, resolve required changes and verify preserved data, access and jobs. Exercise a declared rollback or restore procedure; do not assume migrations are reversible. |

The first executable pilot should cover the small SaaS journey and one
authorization-negative case. Expand to account recovery, events, durable work
and upgrades only as the acceptance fixtures and available resources permit.
An unsupported journey is a recorded scope gap, not an invented successful run.

## Measurements

| Dimension | Record |
| :--- | :--- |
| Development effort | Setup and active implementation time, manual steps, relevant application code/configuration, documentation lookups, failed attempts and fixes. Report build/queue/wait time separately. Code size alone is not productivity. |
| Correctness and failure behavior | Acceptance and negative-case outcomes, reproducible defects, timeout/retry/recovery behavior, error clarity and diagnostic effort. More test files do not establish stronger protection. |
| Performance | Cold and warm builds, startup, representative authenticated/database requests, latency distributions including p50/p95/p99, throughput, errors, CPU and memory at declared loads. |
| Maintenance | A repeatable change such as adding an authorized field, updating a dependency or migrating a release; record time, touched components, regressions and downstream fixes. Longer-term incident/update effort remains unknown until observed. |

A fast implementation that fails the agreed security, durability or correctness
contract does not qualify as a performance/productivity winner. Report the
failure; do not disable protections to improve numbers. Internal Rullst
[scorecard grades](quality-scorecard.md) are not comparative measurements.

## Fairness and reproducibility

- Freeze task requirements, acceptance tests, effort budget and measurement
  method before the trial. Use idiomatic supported approaches on every stack.
  Give competing implementations a comparable debugging/tuning budget.
- Record framework/application commits, exact versions, dependency locks,
  toolchains, OS, machine/container limits, database version/schema/data, cache
  state and commands. Distinguish released software from unpublished candidates.
- Compare matching authentication, validation, persistence, durability and
  response semantics. A minimal Axum HTTP baseline is not a full SaaS competitor;
  disclose application code and libraries needed to assemble missing layers.
- Isolate the load generator, check for bottlenecks, warm up appropriately and
  repeat trials. Declare trial counts, load schedules, errors and variability;
  report inconclusive differences. Hosted CI timing is useful for regression
  detection but is noisy evidence for cross-framework performance rankings.
- Separate local fixture acceptance from provider/device/production evidence.
  Use only free resources and standard public CI runners where appropriate;
  respect the local disk reserve. No real provider accounts, production data,
  paid services or automatic deployments are authorized by this plan.
- For AI-assisted work, retain the actual model/version, tools, instructions,
  budgets and interventions. Keep the procedure comparable and disclose
  familiarity and tuning differences. One assistant's result measures that
  assisted workflow, not universal human productivity or model capability.
  Reuse the [AI evaluation protocol](ai-maintainability-roadmap.md), without
  changing models automatically or claiming independent human validation.
- Keep scripts, sanitized fixtures, raw results and a source-bound report.
  Retain failed trials and tradeoffs; do not publish only favorable workloads
  or turn a weighted internal score into a universal league table.

## Delivery sequence and acceptance

1. **After stable maintenance:** select the first bounded SaaS task, references,
   supported versions and free execution budget. Confirm fixture reuse and
   document the threat/behavior contract before adding implementation work.
2. **Baseline:** exercise usable Rullst source and the selected reference with
   the same acceptance contract. Record current gaps and uncertainty. Do not
   wait for every A priority to be completed and do not claim unrun results.
3. **One improvement:** choose a measured problem with a clear user benefit,
   check the SST and maintenance cost, implement it, and rerun affected cases.
   Existing release and compatibility requirements still apply.
4. **Release review:** when the selected v13 scope is ready, summarize the
   relevant measurements and remaining limits. Required release checks decide
   admission; this optional comparison cannot certify security or waive them.
5. **Maintenance checkpoints:** repeat relevant cases after upgrades or material
   changes. Keep focused regressions in normal CI when useful; run expensive
   cross-framework campaigns explicitly at checkpoints, not on every PR.

An evaluation deliverable needs reproducible commands and source identities,
actual acceptance results, comparable measurements, disclosed exclusions and a
short prioritized response to the findings. A defensible conclusion names the
specific workload and dimension in which Rullst improved, matched or lagged.
The outcome may be to fix a defect, simplify a journey, retain an existing
design or defer an expensive extension. There is no predetermined winner.
