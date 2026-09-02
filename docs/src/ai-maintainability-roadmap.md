# AI Maintainability and Project-Building Roadmap

Rullst aims to be understandable to humans and coding assistants without making
model-independent quality claims. Explicit Rust types, conventional project
structure and generated context can reduce ambiguity, but no framework can
guarantee that an arbitrary model will produce correct, secure or maintainable
software.

This document records the measurable work required to make Rullst excellent for
AI-assisted framework maintenance and application development. It is a
**post-v12-RC roadmap**, not a v12 release gate and not a description of
capabilities that have already shipped.

## Current foundation

The repository already provides useful controls:

- `AGENTS.md` defines the framework architecture, coding invariants, security
  boundaries, validation commands and release order.
- [`spec.md`](spec.md) is the architectural Single Source of Truth.
- [`capability-status.md`](capability-status.md) and the
  [`quality-scorecard.md`](quality-scorecard.md) separate implementation claims
  from engineering evidence.
- `cargo rullst make:*` emits conventional, inspectable Rust source instead of
  hiding application behavior behind runtime reflection.
- `cargo rullst generate:ai-context` emits `.llms.txt` with selected application
  source and dependency context.
- Generated-project tests, compile-fail tests, deterministic provider mocks and
  the workspace validation gates catch classes of mistakes independently of the
  assistant that proposed a change.

These controls are a good foundation. They do not yet prove that a smaller or
less capable model can maintain the framework or build a complete application
at a defined quality level.

## Known limitations

The current AI context generator primarily concatenates `Cargo.toml` and Rust
files from selected conventional directories. That format can become noisy,
does not rank information by task relevance and does not provide a complete
project contract for routes, configuration, migrations, tests, authorization
or operational commands.

The framework documentation is extensive. Breadth helps difficult work, but a
model with weaker retrieval or reasoning can select an obsolete example, miss a
more specific invariant or consume too much context unless it receives a short
task-oriented map first.

Maintaining framework internals is also substantially harder than building an
application from stable public APIs. Cross-crate feature unification, procedural
macros, database dialects, authentication, cryptography and release engineering
require stronger review and broader gates than ordinary application CRUD.

## Post-v12-RC workstreams

### 1. Project-specific agent instructions

Generate a concise `AGENTS.md` for each new application, derived from the
selected blueprint and feature set. It must describe:

- the chosen database, rendering mode and enabled Rullst subsystems;
- the canonical locations for routes, models, controllers, policies,
  migrations, jobs and tests;
- the exact format, lint and test commands for that application;
- mandatory ownership, CSRF, headers, WAF and secret-handling boundaries;
- which generated files are application-owned and which commands may refresh
  them; and
- links to version-matched Rullst documentation.

Acceptance requires snapshot tests for every supported blueprint and a
generated-project compile gate. The generated document must not claim that an
optional integration is active merely because its crate is available.

### 2. Structured and bounded AI context

Evolve `generate:ai-context` from a source concatenator into a deterministic,
versioned project map. The output should present summaries and file paths before
including bounded source excerpts. It should cover:

- dependency and feature selections;
- routes and their authentication/ownership policies;
- models, relationships and migrations;
- controllers, middleware, jobs and external-provider boundaries;
- configuration keys by name, never secret values;
- tests and the commands that exercise each subsystem; and
- freshness metadata sufficient to detect stale generated context.

The generator must enforce an explicit size budget, stable ordering, secret and
binary exclusions, path confinement and deterministic output. Large projects
should receive an index plus task-selectable context shards rather than one
unbounded prompt payload.

### 3. Golden application tasks

Create small, executable reference tasks that represent common real work:

1. add a validated CRUD resource with tenant ownership;
2. add session authentication and a role-protected route;
3. add a migration and repository query without SQL injection;
4. enqueue an idempotent background effect through an outbox;
5. verify a signed webhook and reject replay or invalid signatures;
6. add a provider integration with a deterministic offline mock;
7. diagnose and repair a deliberately broken generated application; and
8. perform an assisted framework upgrade and verify rollback.

Each task needs a fixed starting fixture, an executable acceptance suite,
security-negative cases and a reference solution. Tutorials alone are not
evidence; the fixture must compile and its assertions must prove the behavior.

### 4. Reproducible AI evaluation harness

Evaluate candidate assistants using the same repository revision, task fixture,
tool permissions, time budget and acceptance tests. Record at least:

- functional correctness and hidden-test pass rate;
- formatting, strict Clippy and test results;
- security-invariant violations;
- unsupported or hallucinated APIs and dependencies;
- unnecessary diff size and changes outside the requested scope;
- ability to recover from compiler and test failures;
- documentation truthfulness; and
- elapsed time, model/tool configuration and cost when publishable.

Results must name the exact model/version, agent harness, reasoning setting,
date and commit SHA. A vendor description or one successful demonstration is
not compatibility evidence. Preview and mutable model aliases must be reported
as such.

The first matrix should distinguish at least two profiles:

- **application builder:** works through documented, stable public APIs and
  generated projects; and
- **framework maintainer:** changes crate internals, feature combinations,
  macros, security boundaries or release infrastructure.

A model may qualify for one profile without qualifying for the other.

### 5. Task-oriented documentation routing

Add a short machine-readable and human-readable entry map that answers “which
document should I read for this task?” before exposing the full manual. It must
route architecture changes to the SST, release work to the release programme,
security changes to the relevant threat model and ordinary application work to
the smallest applicable tutorial and API reference.

Where documents overlap, one must be declared authoritative and the others must
link to it rather than restating mutable facts. Documentation examples should
continue to be compiled or exercised wherever practical.

### 6. Governance and review boundaries

AI assistance never replaces contributor accountability or independent review.
The disclosure and verification rules in [`CONTRIBUTING.md`](../../CONTRIBUTING.md)
apply to every model and tool. Exact prompt disclosure remains optional context,
not proof of authorship, completeness or safety.

Routine documentation, test and mechanical dependency work may use a faster
model when the gates prove the result. Authentication, cryptography, ORM/macro
contracts, release signing and cross-crate security changes require heightened
human review regardless of the model used. No evaluation score authorizes
unattended merge or production mutation.

## Delivery sequence

| Phase | Delivery | Promotion evidence |
| :--- | :--- | :--- |
| A | Generated application `AGENTS.md` and context format v2 | Blueprint snapshots, deterministic output, secret-exclusion and generated-project gates |
| B | Golden task fixtures and evaluation runner | Fixed inputs, hidden assertions, security-negative tests and reproducible metadata |
| C | Public model/harness results | Exact versions, commit SHA, limitations and repeatable commands |
| D | Continuous regression programme | Scheduled or manual reruns, versioned baselines and reviewed score changes |

Phases begin after the v12 RC is cut. Compatible documentation corrections may
land in v12 maintenance, but new generator formats, fixtures and public support
profiles belong to the v13 feature line unless a separate release decision says
otherwise.

## Completion criteria

This roadmap may be called complete only when:

- every supported blueprint produces accurate, tested agent instructions;
- context generation is bounded, deterministic, secret-safe and task-routable;
- the golden application and maintenance suites run from clean fixtures;
- published results are tied to exact models, harnesses, commits and dates;
- at least one lower-cost model completes the application-builder profile at the
  project-defined quality threshold without security-critical failures; and
- failure cases and unsupported maintenance classes are documented as clearly as
  successful ones.

Until then, the honest claim is narrower: Rullst is intentionally structured to
support AI-assisted work and provides useful safeguards, but model suitability
must be evaluated for the concrete task and reviewed like any other contribution.

## Non-goals

- guaranteeing correct output from every current or future model;
- detecting all AI-generated code or proving the complete prompt history;
- replacing human ownership, security review or coordinated disclosure;
- auto-merging changes based on a model name or benchmark score; or
- claiming superiority over other frameworks without dated, reproducible
  comparative evidence.
