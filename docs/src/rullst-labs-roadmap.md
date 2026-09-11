# Rullst Labs and isolated runner roadmap

> **Status:** v13 research and design proposal. Neither package described here
> exists in the workspace yet, and this document is not evidence that Rullst can
> safely execute untrusted code in production.

Rullst should make interactive programming exercises, deterministic graders and
bounded learning games easier to build without placing untrusted code inside an
application server. The proposed design deliberately separates the trusted
control plane from the untrusted execution plane.

## Package and deployment boundaries

Both packages are planned for the Rullst monorepo so their contracts,
compatibility tests, security reviews and versioning can evolve together. They
must remain opt-in and must not become default dependencies of `rullst`.

### `rullst-labs`

`rullst-labs` is a safe Rust library for the trusted application side. Its
planned responsibilities are:

- versioned `LabSpec`, `ExecutionPolicy`, request, receipt and grading types;
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
installation operates many workers. Its planned responsibilities are:

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
| 1 | Wasmtime/component-model/WASI backend for suitable exercises | Filesystem, environment, clock, network, fuel/memory and output boundaries tested |
| 2 | Rootless OCI backend for workloads that require native toolchains | Pinned runtime choice, kernel hardening, resource-exhaustion and escape regressions |
| 3 | Optional microVM or independently operated cloud sandbox tier | External operational evidence, image provenance, isolation review and incident procedures |

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

## v13 delivery checklist

- [ ] Complete the Phase 0 threat model and architecture decision record.
- [ ] Stabilize the versioned request, policy, receipt and grader schemas.
- [ ] Implement `rullst-labs` policy validation and deterministic mock backend.
- [ ] Implement authenticated runner transport with idempotency and cancellation.
- [ ] Select and prove the Phase 1 Wasm backend against adversarial fixtures.
- [ ] Evaluate the Phase 2 rootless OCI backend without exposing a control socket.
- [ ] Add Rust/Rullst compile, test and lint language packs with pinned toolchains.
- [ ] Add tenant isolation, minimized audit and retention/deletion evidence.
- [ ] Document reference deployment, capacity planning and failure recovery.
- [ ] Keep offensive CTF infrastructure experimental until independently reviewed.

The packages join the Cargo workspace and the release-order manifest only after
Phase 0 is approved. If the runner's security or operational lifecycle later
requires a separate repository, the versioned protocol must allow that move
without coupling applications to its implementation.
