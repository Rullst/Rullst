# Rullst Labs

Unpublished v13 implementation candidate for trusted exercise orchestration and
exact grading. **Bring your own runner:** this crate never compiles, interprets
or spawns learner code. An application-owned, separately deployed and isolated
runner claims leased jobs and returns signed receipts through the documented
[controller contract](https://github.com/Rullst/Rullst/blob/main/docs/src/labs-runner-contract.md).

The experimental `rullst-labs-runner` candidate (a Linux Rust-to-Wasm/Wasmi
executor) passed source admission in [PR #228](https://github.com/Rullst/Rullst/pull/228)
but was removed from the workspace for 13.0; its source remains in git history.
Final release admission for this crate remains outstanding. Neither the
candidate name nor a signed receipt establishes production readiness.

The default feature provides bounded, versioned contracts with no executor or
network/runtime dependency. `sqlite` adds a dedicated encrypted shared-local job
plane; `receipt-signing` belongs to the runner's trusted controller.
The application must never spawn learner code or expose a container control socket.

## Implemented candidate surface

- Validated tenant/course/learner/job identities, bounded Rust source and immutable
  instructor exercise revisions with at most 64 exact cases.
- Static application authorization for current course access, submission,
  management, own-status access and cancellation. The host supplies authenticated
  identities and current enrollment; a client body cannot supply a hidden grader.
- Encrypted source, grader snapshots and integrity-bound status records in a
  dedicated SQLite database. File possession is a trusted operator capability,
  not student authorization. Capacity, lock waits and clock rollback fail closed.
- Durable idempotent submission, independent-process leases, nonce/revision
  fencing, explicit cancellation, abandoned-work cleanup and at most one retry.
- Ed25519 controller receipts bound to the source/request/profile/current lease.
  Exact trusted grading compares worker values with stored answers. Public
  feedback contains pass/wrong-answer/trap categories, never hidden case inputs,
  expected values or raw returned values. Status views identify the exercise
  snapshot with a store-keyed digest, not the raw `Exercise::digest`, which
  hashes the hidden cases. Compiler diagnostics are bounded
  untrusted text and must be escaped when rendered.
- Terminal source/snapshot removal and explicitly authorized retention cleanup.
  Status/idempotency records must be retained for at least 24 hours before purge.
  After purge, use new submission IDs; permanent deduplication is not promised.
- An explicit simulation mode whose results are `Simulated`, never `Completed`
  or execution evidence. It does not evaluate source.

## Application and runner boundary

The application registers an `Exercise`, submits a `Submission` through
`SqliteLabs::submit`, reads `get_job` and records `cancel`. Submission IDs are
idempotency keys shared by the course, so generate unpredictable random IDs;
another learner's ID is a `Conflict` on submit and `NotFound` from `get_job`
and `cancel` unless the caller may manage jobs. Your runner's trusted controller
uses `claim_next`, monitors `lease_status`, has its isolated worker execute the
attempt and submits a `SignedReceipt` to `complete`. Cancellation and expiry
fence late results.
A job expires at the earlier of its `ttl_seconds` and the Submit permission's
expiry, and is claimed only while more than the exercise's wall limit plus 5
seconds remain. Grant Submit for longer than expected queueing plus that time;
`submit` refuses a job that could never run (`InvalidInput` for a too-short
TTL, `Expired` for a too-short permission).

A lost worker first requires a fenced attempt and confirmed whole-group teardown.
`cleanup_candidates`/`abandon_attempt` and `reconcile_cleanup` provide that durable
boundary. Only then may a controller deliberately request one bounded retry.
The removed candidate and the example controller cancel abandoned work after
cleanup instead of retrying. Cleanup is attested with the same `SignedReceipt`
type, reporting `Rejected(WorkerLost)`; sent to `complete`, that outcome is a
terminal `Failed` job without the retry. `complete` accepts a receipt only when `started_at` is
not before the claim (sample it after `claim_next`, on a clock synchronized
with the store), `finished_at` is not after the store's time and precedes the
lease expiry; otherwise it returns `Protocol`.

Schedule `expire_queued` with current course-management authorization to clear
expired queued source even if the runner is unavailable. It never clears a
running lease or claims worker teardown. `purge_terminal` later removes eligible
status records. `remove_exercise` permits removing a withdrawn grader only after
all referencing jobs have been purged; do not reuse removed revision IDs.

`max_jobs` (at most 1,000) is shared by every tenant using the store and counts
terminal jobs until `purge_terminal`, which requires at least 24 hours, so it
bounds submissions per rolling day. One learner may retain at most 100 jobs per
course, or `max_jobs` when lower; `StoreConfig::learner_jobs` selects another
bound that every opener must share. Rate-limit submissions in
`Authorization::check` for `Submit`, and consider one store per tenant.
`max_exercises` (at most 1,000) is store-wide too: every registered revision,
enabled or withdrawn, counts until `remove_exercise`, so one tenant's
instructors could otherwise register all of it. A store shared by several
tenants should add `StoreConfig::tenant_exercises(n)`, which bounds the
revisions one tenant holds and is persisted like the other capacities.

Use a dedicated random content key and a separate controller signing seed. The
application receives only the controller's pinned public key. The untrusted worker
receives neither key, the database nor expected answers. Keep these resources
separate from application identity/session credentials and database state.

## Examples

The `course_app` example is a runnable **local operator** application fixture for
registration/submission/status/cancel/withdrawal and authorized retention. Its stdin-selected actor and
small fixture policy are not web authentication. A web application must supply
its real authorization and protected transport. This example never starts an
executor; a separately deployed runner consumes the shared job plane.

The `byo_runner_controller` example is a minimal **non-executing** controller.
It recovers leftover leases, claims a job, polls `lease_status` under the wall
limit, signs a fixed `Rejected(Isolation)` verdict for `complete` and fences
every failure after the claim, clock and signing errors included, through
`abandon_attempt`/`reconcile_cleanup`. It never compiles or
runs learner code; `PLUG-IN POINT` comments mark where an isolated worker and
its teardown belong. It runs as a test with the crate's suite:

```bash
cargo run -p rullst-labs --example byo_runner_controller --features sqlite,receipt-signing
```

## Profile and limits

Protocol version 1 defines one profile, `rust-function-wasm-v1`: a pure
`solve(i64, i64) -> i64` function evaluated over at most 64 instructor cases.
Its identifiers name Rust 1.96.0 `wasm32-unknown-unknown` and Wasmi 2.0.0
semantics and are bound into every exercise and profile digest. Source is
limited to 32 KiB, execution to 5–60 seconds, guest memory to 2–16 MiB and fuel
to at most one million units per case. A runner must enforce these limits; this
crate only validates and binds them.

## What your runner must provide

The application never executes submissions. Your runner's isolated worker must
receive only the bounded `WorkerInput`, without the job database, keys, expected
answers, application secrets, network or control sockets, under enforced CPU,
memory, process, disk and output limits and observed syscall/filesystem
restrictions. Unsupported environments must refuse work with no weaker
fallback. The removed candidate's [first-profile threat model](https://github.com/Rullst/Rullst/blob/main/docs/src/labs-first-profile.md)
records one Linux design (namespaces, cgroups v2, seccomp, Landlock) as a
reference. Independent security review and hostile-input acceptance of your
runner are required before any production claim.

Application policy, instructor review, verified enrollment, privacy notices,
backup retention/erasure and accessibility remain host responsibilities. This
crate does not certify legal compliance or award academic credit automatically.
