# Rullst Labs

Unpublished v13 implementation candidate for trusted exercise orchestration and
exact grading. The named Linux journey passed [targeted hosted acceptance](https://github.com/Rullst/Rullst/actions/runs/35582398251)
at `977e40a3` and workspace/archive source admission in [PR #228](https://github.com/Rullst/Rullst/pull/228).
Final release admission and independent isolation review remain outstanding. Neither the candidate name nor a signed
receipt establishes production readiness.

The default feature provides bounded, versioned contracts with no executor or
network/runtime dependency. `sqlite` adds a dedicated encrypted shared-local job
plane; `receipt-signing` belongs to the separately deployed trusted controller.
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
  expected values or raw returned values. Compiler diagnostics are bounded
  untrusted text and must be escaped when rendered.
- Terminal source/snapshot removal and explicitly authorized retention cleanup.
  Status/idempotency records must be retained for at least 24 hours before purge.
  After purge, use new submission IDs; permanent deduplication is not promised.
- An explicit simulation mode whose results are `Simulated`, never `Completed`
  or execution evidence. It does not evaluate source.

## Application and controller boundary

The application registers an `Exercise`, submits a `Submission` through
`SqliteLabs::submit`, reads `get_job` and records `cancel`. The dedicated controller
uses `claim_next`, monitors `lease_status`, performs isolated execution and submits
a `SignedReceipt` to `complete`. Cancellation and expiry fence late results.

A lost worker first requires a fenced attempt and confirmed whole-group teardown.
`cleanup_candidates`/`abandon_attempt` and `reconcile_cleanup` provide that durable
boundary. Only then may a controller deliberately request one bounded retry.
The supplied first controller cancels abandoned work after cleanup by default.

Schedule `expire_queued` with current course-management authorization to clear
expired queued source even if the runner is unavailable. It never clears a
running lease or claims worker teardown. `purge_terminal` later removes eligible
status records. `remove_exercise` permits removing a withdrawn grader only after
all referencing jobs have been purged; do not reuse removed revision IDs.

Use a dedicated random content key and a separate controller signing seed. The
application receives only the controller's pinned public key. The untrusted worker
receives neither key, the database nor expected answers. Keep these resources
separate from application identity/session credentials and database state.

The `course_app` example is a runnable **local operator** application fixture for
registration/submission/status/cancel/withdrawal and authorized retention. Its stdin-selected actor and
small fixture policy are not web authentication. A web application must supply
its real authorization and protected transport. This example never starts an
executor; the independently deployed runner consumes the shared job plane.

## First selected profile and limits

The experimental Linux x86-64 runner compiles a fixed Rust 1.96.0
`solve(i64, i64) -> i64` exercise to import-free Wasm and uses Wasmi 2.0.0.
Source is limited to 32 KiB, artifacts to 256 KiB, execution to 5–60 seconds,
guest memory to 2–16 MiB and fuel to at most one million units per case.
Native OS resource limits also cover compiler/translation work. No Cargo build
scripts, extra packages, shell commands, WASI or native Rullst servers are included.

The [runner](../rullst-labs-runner/README.md) requires observed namespaces,
cgroups v2, fixed read-only mounts, seccomp and a fully enforced Landlock policy.
Unsupported environments refuse work; there is no weaker execution fallback.
Independent security review and hostile-input acceptance are still required.

Application policy, instructor review, verified enrollment, privacy notices,
backup retention/erasure and accessibility remain host responsibilities. This
crate does not certify legal compliance or award academic credit automatically.
