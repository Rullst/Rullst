# Labs runner contract: bring your own runner

> **Status:** `rullst-labs` is an unpublished v13 candidate. The experimental
> `rullst-labs-runner` candidate (a Linux Rust-to-Wasm/Wasmi executor) was
> removed from the workspace for 13.0. Its source remains in git history, for
> example at `main` revision `f7909033`. Rullst no longer ships an executor.
> Applications deploy their own runner against the contract below.

`rullst-labs` owns trusted exercises, authorization, encrypted durable jobs,
fenced leases, exact grading, cancellation, recovery and retention. It never
compiles, interprets or spawns learner code. A **runner** is the application's
separately deployed program that takes leased work, executes it in an isolated
environment and reports an Ed25519-signed receipt. This page states what that
runner must do with the existing `rullst-labs` API, what the library verifies and
what the host must guarantee. Nothing here certifies a runner as safe.

## Roles

| Component | Holds | Never holds |
| --- | --- | --- |
| Application (web process) | `SqliteLabs` handle, content key, pinned receipt **public** key; `register_exercise`, `submit`, `get_job`, `cancel`, `set_exercise_enabled`, `expire_queued`, `purge_terminal`, `remove_exercise` | Receipt signing seed, an executor, a container control socket |
| Trusted controller (your runner's trusted half) | The same job database file and content key, the receipt signing seed; `claim_next`, `lease_status`, `complete`, `abandon_attempt`, `cleanup_candidates`, `reconcile_cleanup` | Application identity/session credentials |
| Untrusted worker (your isolated execution) | One `WorkerInput`: source, case inputs, limits and an opaque binding | Job database, content key, signing seed, expected answers, application secrets, network, control sockets |

The controller operations are not student or HTTP capabilities. Never wire them
to an application route. The shared-local store means one trusted local
filesystem with explicit process separation; it is not a remote broker or a
network-filesystem database.

## Controller prerequisites

- Depend on `rullst-labs` with the `sqlite` and `receipt-signing` features.
- Open the application's store with `SqliteLabs::open(path, config, key, clock)`.
  `StoreConfig` (namespace, `max_jobs`, `max_exercises`, profile and any
  `learner_jobs` quota) and the `ContentKey` must be identical to the
  application's. The store binds them; any difference returns `Configuration`.
- Use `ExecutionProfile::LinuxExperimental { tools, receipt_key }`. It is the
  only profile whose signed receipts `complete` and `reconcile_cleanup` accept;
  `Simulation` returns `Unsupported` there. Despite the name, `rullst-labs`
  checks neither Linux nor isolation. `ToolIdentity` holds seven SHA-256 digests
  (`runner`, `compiler`, `wasm_toolchain`, `runtime`, `launcher`,
  `syscall_policy`, `filesystem_policy`) that the operator pins for the
  runner's installed files. They are bound into every job, not verified.
- Create the signer with `ReceiptSigner::from_seed` from a dedicated random
  32-byte seed (all zeroes is refused). Its `public_key()` is the profile's
  `receipt_key`. Never reuse the content key or an application secret as the
  seed, and never give the seed to the application or the worker.
- The profile, including tool digests and receipt key, is part of the store's
  configuration binding. Rotating the key or changing a pinned tool therefore
  requires a new store: drain and retire the old one first.
- Use a clock synchronized with the store's. The store refuses time that moves
  backwards (`Clock`) and receipts outside the lease window (`Protocol`).

## Lifecycle

### 1. Recover before claiming

On startup, and periodically, call `cleanup_candidates(limit)` (1–32). It
returns leased jobs that were cancelled, withdrawn or expired, or whose lease
ran out. A still-`Running` record is fenced to `Uncertain` (or `Expired`) before
it is returned. For each `CleanupJob`, tear down everything your runner owns for
`binding.nonce`, then attest it as described in step 7. Recovery never releases
source and can never award a grade.

### 2. Claim

`claim_next()` returns at most one `LeasedJob`, or `None`. It skips up to 32
queued jobs that are withdrawn (marked `Cancelled`), too close to expiry or out
of attempts (marked `Expired`). A job is claimed only while more than its wall
limit plus 5 seconds of lifetime remain, and at most twice. Each claim creates a
fresh random nonce and a lease that ends at the claim time plus the wall limit
plus 15 seconds, capped at the job's expiry. `LeasedJob` exposes `scope()`,
`id()`, `revision()`, `expires_at()` (the lease end) and `input()`.

### 3. Hand the worker its input

`job.input()` is a `WorkerInput`:

| Field | Bound |
| --- | --- |
| `binding()` | `AttemptBinding`: request, profile and source digests plus the attempt nonce |
| `source()` | `RustSource`, at most 32 KiB; `expose_source()` is for the isolated boundary only, never logs |
| `inputs()` | 1–64 `[i64; 2]` case inputs, in grader order |
| `limits()` | `wall_seconds` 5–60, `fuel_per_case` 1,000–1,000,000, `memory_pages` 32–256 (2–16 MiB) |

It carries no tenant or learner identity and no expected answers. Its JSON form
is decoded by `WorkerInput::from_bytes`, which refuses more than 131,072 bytes.
Protocol version 1 (`PROTOCOL_VERSION`) defines one profile, `PROFILE`
(`rust-function-wasm-v1`): a pure `solve(i64, i64) -> i64` function evaluated
once per input. `TOOLCHAIN` and `INTERPRETER` name the removed candidate's
Rust 1.96.0 `wasm32-unknown-unknown` and Wasmi 2.0.0 semantics; all three
constants are bound into exercise and profile digests. Another language or
native profile needs a new `rullst-labs` protocol, not a runner-side extension.

### 4. Monitor the lease

Before releasing source, and while the worker runs, call `lease_status(&job)`.
`Active` requires an enabled exercise, a `Running` job, the same revision, lease
and binding, and an unexpired lease. `Stop` or an error means stop now. A status
read never extends the lease. Enforce `limits().wall_seconds()` yourself. The
removed candidate polled every 200 ms; choose an interval that bounds wasted work.

### 5. Validate the worker's reply

Decode untrusted worker bytes with `WorkerOutput::from_bytes` (at most 20,480
bytes) and require `output.binding == *job.input().binding()`. Outcomes:

- `Executed { artifact, cases }`: one `CaseOutput` per input, in order. Each is
  `Value(i64)` or `Trap(Fuel | Memory | Stack | Guest)`. A case count different
  from the exercise's is `Protocol`. `artifact` is an opaque digest.
- `CompileRejected { diagnostics }`: `Diagnostic` text of at most 8,192 bytes,
  no control characters except newline and tab. It is untrusted and can quote
  source; escape it when rendering.
- `Rejected(failure)`: `Compile`, `InvalidModule`, `ResourceLimit`,
  `WorkerProtocol`, `WorkerLost` or `Isolation`.

The worker never sends expected values or a pass flag. Sign `Executed` only for
code that actually ran: `complete` grades it against the hidden answers.

### 6. Sign and report

Build an `ExecutionReceipt` and sign it with `ReceiptSigner::sign`:

- `started_at`: sampled **after** `claim_next` returns; it must not precede the
  claim time the store recorded.
- `finished_at`: at most 90 seconds after `started_at`, not later than the
  store's time when `complete` runs and strictly before `job.expires_at()`.
- `observation_digest`: a digest of your actual per-job isolation and resource
  observations. `rullst-labs` stores it as evidence but cannot interpret it.
- `teardown`: `Confirmed` only after the worker group and workspace are gone.
  `complete` refuses `Uncertain`.

Send it to `complete(job.scope(), job.id(), &signed)`. `Executed` becomes a
`Completed` job with per-case `Passed`/`WrongAnswer`/`Trapped` feedback; any
other outcome becomes `Failed`. Serialized receipts are decoded by
`SignedReceipt::from_bytes` (at most 20,480 bytes, lowercase hex signature). An
exact replay of an accepted receipt returns the same view; a different receipt
for a finished job is `Conflict`.

### 7. Abandon, clean up and reconcile

On any failure, stop signal, timeout or rejected `complete`:

1. `abandon_attempt(&job)` fences the attempt immediately (`Running` becomes
   `Uncertain`, or `Expired`) and returns a `CleanupJob`. An old nonce cannot
   fence a newer attempt.
2. Tear down everything owned for `cleanup.binding.nonce`. If absence cannot be
   confirmed, sign nothing; the job stays fenced with `cleanup_pending` and
   `cleanup_candidates` returns it on a later pass.
3. Sign a cleanup receipt whose output is
   `Rejected(ExecutionFailure::WorkerLost)` for `cleanup.binding`, with
   `Teardown::Confirmed`, whatever the attempt did before teardown.
4. Call `reconcile_cleanup(scope, id, &receipt, retry)`. Cancelled and expired
   jobs stay terminal. An `Uncertain` job becomes `Queued` again only with
   `retry = true`, an enabled exercise, fewer than two attempts and enough
   lifetime (a fresh nonce on the next claim); otherwise `Cancelled`, or
   `Expired` when too little time remains. Reconciliation never grades.

A `WorkerLost` output sent to `complete` instead becomes a terminal `Failed`
job without a retry. The removed candidate never retried automatically.

### 8. Retention

`complete` and terminal reconciliation erase the encrypted source and grader
snapshot from the job record. The application schedules `expire_queued` and
`purge_terminal` (at least 24 hours) with `ManageJobs` authorization; neither
requires the runner or claims worker teardown. The runner must delete its own
workspaces, artifacts, worker output and logs, and must not persist source or
diagnostics beyond the attempt. Backups, WAL pages and physical erasure remain
operator responsibilities.

## Verified by `rullst-labs` versus guaranteed by the host

| `rullst-labs` verifies | The host and its runner must guarantee |
| --- | --- |
| Ed25519 signature against the pinned `receipt_key`, never a key inside the receipt | Custody of the signing seed; only the trusted controller can sign |
| Binding to the current lease: request, profile and source digests and nonce, plus job revision | That the worker actually executed that source under that profile |
| Lease and job expiry, receipt timing window, exercise still enabled | Clock synchronization; enforcing the wall limit and stopping on `Stop` |
| `Teardown::Confirmed` is present | That teardown really happened before signing |
| Case count, bounded wire sizes and decoded invariants | Fuel, memory, process, disk, output and network limits actually enforced |
| Exact grading against stored expected answers; minimized public feedback | Expected answers, keys and the database never reach the worker |
| Idempotent replay, single award and fencing of stale, cancelled or withdrawn results | Real isolation evidence behind `observation_digest`; tool provenance behind `ToolIdentity` |
| Encrypted, integrity-checked job content and erasure at terminal states | Deletion of runner-side workspaces, artifacts, logs and backups |

## Isolation the host owns

Treat a container, a separate crate or a separate process alone as insufficient.
The minimum baseline, from the [roadmap](rullst-labs-roadmap.md#isolation-baseline):

- run the worker outside the web application process, preferably on a separate
  host or VM, with no job database, content key, signing seed, expected answers,
  application credentials, inherited environment, cloud metadata access or
  container/orchestrator control socket;
- deny network access; run unprivileged with no new privileges, dropped
  capabilities and a read-only root plus a fresh, bounded, disposable workspace;
- enforce CPU, wall-clock, memory, process, file, disk and output limits
  (cgroups v2 or an equivalent) covering compilation as well as execution;
- apply syscall and filesystem restrictions and **observe** them in effect
  before releasing source; refuse work on unsupported hosts with no weaker
  fallback;
- pin toolchains and images by digest, tear down each attempt completely and
  treat compiler output and worker bytes as untrusted.

The removed candidate's [first-profile threat model](labs-first-profile.md)
records one detailed Linux design (namespaces, cgroups v2, seccomp, Landlock)
and its historical hosted evidence. It is a reference, not shipped software.
Describe a runner as sandboxed only with a published threat model, platform
requirements and adversarial evidence for that runner, plus independent review
before any production claim about hostile code.

## Minimal non-executing example

[`rullst-labs/examples/byo_runner_controller.rs`](https://github.com/Rullst/Rullst/blob/main/rullst-labs/examples/byo_runner_controller.rs)
opens the store as a separate controller, recovers leftover leases, claims a
job, polls `lease_status` under the wall limit, signs a fixed
`Rejected(Isolation)` verdict and submits it to `complete`. It also fences
failures through `abandon_attempt` and `reconcile_cleanup`. It never compiles or
runs learner code; two `PLUG-IN POINT` functions mark where an isolated worker
and its teardown belong.

```bash
cargo run -p rullst-labs --example byo_runner_controller --features sqlite,receipt-signing
cargo test -p rullst-labs --all-features --example byo_runner_controller
```

The [`course_app` example](https://github.com/Rullst/Rullst/blob/main/rullst-labs/examples/course_app.rs)
shows the application side: registration, submission, status, cancellation,
withdrawal and retention.

## Compatibility

There is one wire protocol (version 1) and one profile. All wire types reject
unknown fields. A runner must use these exact types; there is no adapter layer
or runner version negotiation, and interchangeable external executors are not
provided. New profiles, languages or a remote job transport are `rullst-labs`
roadmap work and require their own threat model and acceptance evidence.
