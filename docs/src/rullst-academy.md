# Rullst Academy product programme

> **Status:** proposed reference product, not a shipped framework capability.
> Academy must live in a separate repository and depend only on published
> Rullst packages. Its operation is evidence for the framework; its product
> content and deployment are not part of the `rullst` crate.

Rullst Academy is a web-first platform for learning Rust and Rullst through
short explanations, authoritative interactive exercises, review sessions and
small practical projects. The target is an enjoyable learning product, not a
claim that a framework can automatically produce good pedagogy or a universally
better alternative to an existing course.

## Product boundary

The browser-accessible web application is canonical. Server-side code owns
identity, school scope, authorization, course versions, grading, progress,
achievements and audit. Omni shells may later add narrowly scoped native
capabilities without becoming a second security or business model.

The Academy repository must:

- pin one exact Rullst RC or stable release and contain no monorepo path
  dependencies;
- use public CLI scaffolds and escape hatches instead of private framework
  modules;
- keep curriculum, content, product UI, deployment and learner support in the
  application;
- contribute only proven, reusable abstractions back to optional framework
  crates;
- act as the first real upgrade, backup/restore and recovery consumer before a
  Rullst stable release is declared ready.

## Focused first release

The first useful Academy release should complete one narrow journey well:

1. A learner creates an account, joins the appropriate school and enrolls in a
   published course.
2. The learner reads or watches an accessible lesson and resumes recorded
   progress.
3. A server-authoritative quiz, matching or typed-recall activity records an
   idempotent result without accepting client-authored points.
4. The review queue schedules a later activity and an achievement or
   certificate is derived from persisted rules.
5. A practical project is submitted to a constrained runner and receives
   deterministic test feedback.
6. An instructor drafts, reviews and publishes content; an administrator can
   audit the resulting events without crossing school boundaries.

Polish should concentrate on this journey: responsive SSR, keyboard operation,
visible focus, reduced motion, clear errors, fast navigation and useful empty
states. Payment, social features, advanced gamification, native-store releases,
offline synchronization and a large course catalogue are not prerequisites for
the first release.

## Initial curriculum and projects

Two short tracks are enough to validate the product:

| Track | Initial lessons | Practical outcome |
| :--- | :--- | :--- |
| Rust foundations | ownership and borrowing, structs/enums, pattern matching, errors, iterators, async and tests | a tested command-line application that persists bounded data |
| Rullst web development | project creation, routes/SSR, forms, ORM/migrations, authentication/authorization, queues and deployment preparation | a secure small web application with owner-only CRUD and background work |

Every exercise must have a versioned ruleset, bounded input/output and retained
test identity. Curriculum authors, not an LLM, define the expected concepts,
tests, hints and completion conditions.

## Safe practical-code runner

Learner code is hostile input. It must never execute inside the Academy web
process or through a mounted host Docker socket. The application-owned runner
must use a disposable rootless container or stronger microVM boundary with:

- no network by default and no cloud/application credentials;
- read-only base images and a fresh writable workspace per attempt;
- explicit CPU, memory, process, file, disk, output and wall-clock limits;
- a reviewed Rust toolchain/dependency policy and immutable image digest;
- bounded compile/test logs with control-character handling and secret
  redaction;
- an idempotent submission identity, queued execution, cancellation and
  terminal retry/dead-letter policy;
- cleanup after success, failure, timeout and worker restart;
- adversarial escape, fork-bomb, filesystem, network and output-amplification
  tests before public use.

The runner returns structured test evidence. It does not grant points directly;
the Academy service binds that evidence to the authenticated learner, project
version and server-owned scoring policy.

## Local AI mascot

The mascot can be a friendly tutor backed by the existing Ollama path in
`rullst-ai`, with a deterministic offline fallback for tests. It should use a
bounded RAG corpus containing version-pinned official Rust material, Rullst
documentation and Academy-authored hints.

The mascot may explain an error, ask a guiding question, retrieve a relevant
lesson or suggest the next exercise. It must not:

- authoritatively grade code, invent completion or change persisted points;
- execute arbitrary tools or learner code outside the sandbox boundary;
- retrieve another learner's conversation, submission or school data;
- receive secrets, raw session values or unnecessary personal data;
- present an uncited generated statement as official Rust or Rullst behavior.

Responses should carry source references and a visible “local AI tutor” label.
Provider/model unavailability must leave the curriculum and deterministic
grader usable. Prompt-injection regressions, context limits, tenant binding,
PII masking and secret-minimized audit remain mandatory.

## What exists and what remains application work

| Boundary | Current reusable foundation | Academy must still prove |
| :--- | :--- | :--- |
| Learning domain | Generated curriculum, enrollment, progress, activities, quizzes, review, completion, certificates, leaderboard and automation foundations | coherent product UX, content quality, complete authorship and browser E2E |
| Identity and schools | Session/RBAC helpers and persisted school-scoped LMS contracts | account recovery, invitations, device/session policy and every cross-school negative |
| AI tutor | guarded providers, Ollama fallback, bounded tenant-aware RAG and audit contracts | curated corpus, pedagogy, model evaluation, capacity and user-facing failure behavior |
| Practical projects | queues, outbox and bounded messaging foundations | isolated runner, immutable images, resource policy and escape testing |
| Media | bounded accessible lesson metadata, captions and transcripts | upload, storage, scanning, transcoding, caption quality and retention |
| Operations | health/readiness, telemetry, deploy scaffolds and upgrade assistant | production topology, TLS/proxy identity, backup/restore, rollback, alerts and incident response |

## Repository and release boundary

Academy will be developed in a separate repository and conversation. This
document records only the intended boundary and the reusable framework
foundation that already exists; it does not authorize further Academy work in
the Rullst repository and it is not a framework release gate.

- **v12.0:** close the framework independently through its coverage,
  CI/package/security, upgrade and release-candidate gates. Academy does not
  need to exist or run against the RC.
- **v12.0.x:** maintenance only for confirmed framework defects and security
  fixes, without an Academy capability programme.
- **v13:** the next feature line. Reusable improvements discovered by the
  future external Academy may be proposed with their own bounded contracts;
  research-heavy or breaking work remains explicitly governed by v13 criteria.

The 32 canonical milestones that are not fully closed belong to the long-term
v13 horizon. They are not release blockers for v12.0, and Academy itself is not
part of that milestone denominator.

## Acceptance evidence

Before Academy can be treated as release evidence, record all of the following
against immutable application and framework SHAs:

- the complete learner/instructor/admin journey on PostgreSQL, with SQLite kept
  as a local profile only when its declared limits are acceptable;
- anonymous, cross-user, cross-role and cross-school denial tests;
- browser accessibility and responsive-layout checks for the primary journey;
- sandbox abuse tests and bounded compile/test output;
- backup restoration, forward migration, rollback and framework-upgrade drills;
- dependency-only installation from crates.io with no Rullst path overrides;
- load and failure tests with unavailable AI, mail, cache and worker services;
- a documented human GO/NO-GO decision and remaining product risks.

Related reusable guides include [accessible Academy media](tutorials/45-accessible-academy-media.md),
[server-authoritative activities](tutorials/46-server-authoritative-learning-activities.md),
[durable spaced review](tutorials/47-spaced-review-queue.md),
[tenant-bound RAG](tutorials/41-tenant-bound-rag.md), and the
[assisted upgrade workflow](tutorials/36-assisted-framework-upgrades.md).
