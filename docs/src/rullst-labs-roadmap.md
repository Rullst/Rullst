# Rullst Labs and isolated runner roadmap

> **Status:** v13 implementation candidate; the first-profile decision is recorded,
> and [source admission passed in PR #228](labs-first-profile.md#recorded-linux-acceptance).
> Final release admission and independent review remain outstanding. The recorded
> patch-coverage gap remains a quality limitation. This document is not evidence that Rullst can
> safely execute untrusted code in production.

Rullst should make interactive programming exercises, deterministic graders and
bounded learning games easier to build without placing untrusted code inside an
application server. The proposed design deliberately separates the trusted
control plane from the untrusted execution plane.

## Package and deployment boundaries

Both unpublished packages are implementation candidates in the Rullst monorepo so their contracts,
compatibility tests, security reviews and versioning can evolve together. They
must remain opt-in and must not become default dependencies of `rullst`.

### `rullst-labs`

`rullst-labs` is a safe Rust library for the trusted application side. The first
candidate implements `Exercise`, `ExecutionLimits`, `Submission`, worker/receipt
contracts and an optional encrypted shared-local SQLite job plane. Broader
profile-independent contracts remain roadmap work. Its responsibilities are:

- versioned exercise, bounded execution, request, receipt and grading types;
- tenant-, learner- and submission-bound authorization inputs;
- resource, language, toolchain, network and artifact policy validation;
- idempotent submission orchestration, cancellation and result reconciliation;
- static-dispatch backend traits and a deterministic offline mock; and
- minimized audit events that exclude submitted source and secrets by default.

It must never interpret raw shell strings, spawn learner code, grant execution
permissions or treat an AI-generated assessment as authoritative evidence.

### `rullst-labs-runner`

`rullst-labs-runner` is a separately deployed binary/service for the untrusted
execution plane. The singular name describes one runner service even when an
installation operates many workers. The first Linux Rust/Wasmi implementation
requires actual namespaces/cgroups/seccomp/Landlock enforcement. The named Linux
journey and workspace/archive source admission passed in PR #228; final release
admission and independent review remain outstanding. Its responsibilities are:

- an authenticated, versioned and bounded request protocol;
- queue leases, cancellation, retry, idempotency and stale-job recovery;
- pinned images and toolchains, disposable workspaces and capped artifacts;
- exact compile, test, lint and grader outcomes; and
- signed or otherwise integrity-bound, privacy-minimized execution receipts.

The runner must never be embedded in the HTTP application process. The
application must not mount a Docker, Podman, containerd or Kubernetes control
socket merely to operate it. Deployment credentials, host secrets and cloud
metadata must be absent from the execution environment.

## Isolation baseline

Every real backend must apply defence in depth rather than treating a container
as a complete security boundary:

- deny network access by default; explicit destinations require a reviewed
  per-lab allowlist;
- run rootless and unprivileged, drop capabilities, set `no_new_privs`, and use
  a read-only root filesystem with a fresh bounded workspace;
- enforce CPU, wall-clock, memory, process, file, disk, output and artifact
  limits, preferably with cgroups v2 on Linux;
- use namespaces plus supported seccomp, Landlock or equivalent platform
  restrictions;
- pin immutable image digests, compiler versions, dependencies and grader
  revisions;
- tear down each workload completely and quarantine abnormal results; and
- never expose other tenants' jobs, caches, network namespaces or artifacts.

“Sandboxed” may be used in product documentation only for a named backend whose
threat model, platform requirements and executable adversarial evidence are
published. No backend may promise that arbitrary hostile code is risk-free.

## Backend progression

| Phase | Scope | Required evidence before promotion |
| :--- | :--- | :--- |
| 0 | Protocol, threat model, ADR, policy validation and offline mock | Contract, abuse-case and compatibility tests; no untrusted execution claim |
| 1 | Restricted Wasm exercises; the selected first profile uses pinned Rust compilation and Wasmi in a separate restricted Linux process | Compiler and interpreter resource bounds, filesystem, environment, network, fuel/memory, output and recovery tested; WASI/component capabilities need their own later profile |
| 2 | Rootless OCI backend for workloads that require native toolchains | Pinned runtime choice, kernel hardening, resource-exhaustion and escape regressions |
| 3 | Optional microVM or independently operated cloud sandbox tier | External operational evidence, image provenance, isolation review and incident procedures |

The [first-profile decision](labs-first-profile.md) selects a bounded Rust
pure-function journey; native Rullst server exercises remain outside it.
The first supported language pack should be Rust/Rullst. Additional languages
must reuse the same policy and receipt contracts instead of adding ad-hoc shell
execution paths.

## Learning and game scope

The foundation may support compilation exercises, unit and property-based
graders, lint challenges, bounded command-line programs and server-authoritative
learning games. AI may explain a result or suggest the next exercise, but it
must not weaken an execution policy, grant network access, reveal secrets or
convert an unverified run into a passing grade.

General-purpose realtime games still require application-specific gameplay,
assets, moderation, anti-cheat, state synchronization and operational design.
`rullst-labs` is not intended to become a graphics or physics engine.

## Offensive security and CTF boundary

Ordinary secure-coding exercises may run under the standard restricted profile.
A narrowly scoped offensive exercise may be explored only through a distinct,
experimental profile with a dedicated target and deny-by-default egress.

A complete offensive CTF arena is external deployment infrastructure integrated
with Rullst, not an in-process framework feature. It requires separate network
segments and accounts, disposable machines or hardened workloads, vulnerable
targets created solely for the exercise, rate and abuse controls, monitoring,
cleanup, incident response and applicable legal authorization. It must not
share hosts, clusters, databases, credentials or control planes with the Rullst
application or other production workloads.

Rullst may provide typed provisioning contracts, challenge metadata, identity,
scorekeeping and receipts for such an arena. Operators remain responsible for
deploying and governing the isolated infrastructure. A local crate alone cannot
make a full offensive CTF safe.

## Acceptance gates

Before either package can be described as production-ready for untrusted code:

- approve a threat model and architecture decision record;
- obtain an independent security review of the selected isolation backend;
- test fork/process, memory, CPU, disk and output exhaustion;
- test path traversal, symlink races, environment leakage and artifact escapes;
- test network denial, cloud-metadata denial and cross-tenant replay/isolation;
- test cancellation, worker loss, duplicate delivery and bounded recovery;
- publish the supported Linux/runtime matrix and fail closed elsewhere; and
- define retention, deletion, observability and incident-response contracts.

## Explicit non-goals

- a claim that arbitrary code can be executed with zero escape risk;
- embedding an executor or container-control socket in the web server;
- a hosted Rullst service being implied by the open-source packages;
- a malware-analysis environment or unrestricted offensive range;
- letting an LLM decide security policy or authoritative grading; and
- store, certification or platform guarantees that require external evidence.

## v13 usable journey target

On September 20 the owner prioritized completing Bunny Stream and then Labs
before opening more optional implementation streams. Phase 0 remains necessary
for architecture and protocol review, but contracts and a mock are an
intermediate milestone. The intended Labs outcome is a usable exercise and
grading journey through a separately deployed runner:

1. An authorized instructor defines a versioned exercise, exact toolchain/grader
   revision and execution policy; tenant and learner permissions are checked by
   the trusted application before submission and result access.
2. An authorized learner submits bounded input to a durable, idempotent job.
   Retries cannot silently create a new execution or apply a result twice.
3. One named isolated backend executes the supported exercise profile with
   explicit resource, filesystem, environment, network and output limits. The
   initial candidate is the Phase 1 Wasm backend; supported Rust exercise and
   compilation requirements must be demonstrated before advertising a language
   pack. Rust/Rullst native server exercises are not implied by Wasm support.
4. The runner returns an integrity-bound receipt with exact grader revision,
   outcomes and bounded diagnostics. Validate request/result binding and reject
   stale, cross-tenant, forged or incompatible results before applying them.
5. Cancellation, timeout, duplicate delivery, worker restart/loss, cleanup and
   reconciliation have executable recovery tests. Retention and deletion cover
   source, workspaces, outputs and artifacts rather than only the job record.
6. A focused application consumer exercises submission, status, cancellation
   and result access. Package/deployment guides explain installation, supported
   platforms, capacity limits, required configuration and failure recovery.

Use real automated disposable-runner tests for execution and isolation; no
manual or paid provider account is required to develop those fixtures. Local
disk/build limits still apply, so use hosted acceptance for large runtime builds.
Passing protocol mocks alone cannot establish isolation. The production-ready
criteria above, including independent review of the selected backend, remain
distinct requirements; automated tests do not substitute for that evidence.

The September 23 feature freeze and September 24–26 validation/publication
window remain unchanged. Pursue this outcome before other optional features,
but do not promise the full runner before its acceptance evidence exists. If
incomplete at freeze, retain an explicit experimental boundary and carry the
missing journey forward. Additional languages, OCI/microVM backends and full
CTF infrastructure do not block completion of a supported first profile and
must not be bundled into its claim.

## v13 delivery checklist

Checked items describe the implemented first-profile candidate, not stable
publication or production readiness. Broader schemas/backends retain their own
acceptance requirements.

- [x] Record the Phase 0 threat model and first-profile architecture decision.
- [x] Implement bounded versioned exercise/request/worker/receipt types and decoding for the selected profile.
- [x] Implement policy validation and an explicit simulation that cannot produce real execution evidence.
- [x] Implement application-authorized shared-local SQLite transport, encrypted content, idempotency, fenced leases and cancellation.
- [x] Exercise the pinned Rust-to-Wasm/Wasmi Linux profile against the named hosted execution and denial fixtures.
- [x] Exercise tenant/learner denial, minimized results, terminal source removal and bounded retention.
- [x] Document candidate host preparation, shared kernel capacity, protected keys and failure recovery.
- [x] Pass workspace/platform and extracted-package source admission (PR #228).
- [ ] Address the recorded non-required patch-coverage gap without weakening isolation.
- [ ] Complete independent isolation review and final release/package admission.
- [ ] Evaluate rootless OCI or microVM backends as separate later profiles.
- [ ] Add native Rust/Rullst application, general Cargo, test and lint language packs; the pure-function profile does not provide them.
- [ ] Reconsider offensive CTF infrastructure only through its separately governed experimental deployment and review.

The candidates belong to the Cargo workspace but remain `publish = false` and
outside the release-order manifest until their separate admission requirements
pass. If the runner's security or operational lifecycle later
requires a separate repository, the versioned protocol must allow that move
without coupling applications to its implementation.
