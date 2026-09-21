# Labs first execution profile: decision and threat model

Status: implementation decision for an **unpublished experimental candidate**.
No runner acceptance, independent security review or production readiness is
claimed by this document. The September 23 freeze and September 24–26 validation
window still apply. Contracts and an offline simulation are intermediate work.

## Supported journey

An authorized instructor creates an immutable exercise revision with a bounded
exact grader. A currently authorized learner submits Rust source to a dedicated
durable job. A separately deployed runner compiles and evaluates that submission
under one named Linux profile. The application exposes authorized status, result
and cancellation, with durable restart/lease recovery and bounded retention.

The first exercise is a pure function with the fixed ABI
`solve(i64, i64) -> i64`, evaluated over at most 64 instructor-defined cases.
Rust 1.96.0, the `wasm32-unknown-unknown` standard-library/toolchain files and
the runner artifact and minimal dynamic runtime tree must be pinned and checked. Submission compilation uses
fixed arguments and no Cargo build scripts, package downloads, external
dependencies, arbitrary shell commands or student-selected toolchain paths.
Rust source, compiler output and resulting Wasm are all untrusted bounded data.
Native Rullst applications, arbitrary-language workspaces and network exercises
remain later profiles rather than implicit capabilities of this one.

Wasmi 2.0.0 is selected for the first interpreter implementation: validation,
extra runtime checks, explicit structural/store/stack limits and deterministic
fuel are required; WAT parsing, WASI, host imports, shared memories, threads,
start functions and unsupported proposals are excluded. A fresh instance per
test case avoids hidden state between cases. Reference-type decoding is enabled
for the pinned Rust standard library's `call_indirect` encoding; tables are
restricted to bounded `funcref`, and imports remain prohibited. The interpreter is one boundary
inside process isolation, not a substitute for it. Wasmtime/component/WASI
support remains a separate profile decision.

## Ownership and process boundaries

| Component | Authority and data |
| --- | --- |
| Application and `rullst-labs` | Current tenant/course/enrollment policy; exercise/grader revision; durable job submission/status/cancel/result. No execution engine, learner process or control socket. |
| Dedicated shared-local job store | Versioned, integrity-checked encrypted source/grader content; bounded jobs, leases, cancellation and minimized results. Separate key and database from application identity/session state. |
| Trusted runner controller | Dedicated job-store and receipt authority; pinned tool paths; leased work, compiler/executor launch, bounded output and teardown. No general application credentials. |
| Per-job compiler/interpreter process | Only the selected source, fixed toolchain, case inputs and disposable output/workspace. No expected answers, job database, runner signing key, host home, environment credentials or control sockets. |

Shared-local means one trusted local filesystem with explicit process separation;
this profile does not supply a remote broker, network filesystem or distributed
runner service. Application authorization never follows from a client-provided
learner, course or provider identifier. File possession/configuration is not a
student capability. Neither signed requests nor an AI evaluation can weaken the
execution policy or manufacture a passing result.

## Mandatory Linux execution boundary

The implementation must apply and test all of the following before accepting
untrusted input into the worker:

- A dedicated delegated cgroups v2 subtree for each job, with hard memory,
  swap, process and CPU limits plus controller-owned wall-clock deadlines.
  Place the inert child in its bounded group before releasing input; cover
  compiler initialization/translation as well as guest execution.
- Unprivileged user, PID, mount, IPC and network namespaces; capability drop,
  no-new-privileges, no inherited terminal/session or additional user namespaces.
  The launcher is a reviewed, pinned configuration of Bubblewrap, not an
  arbitrary command string supplied by an application request.
- A minimal read-only toolchain/runtime tree, a fresh bounded workspace and
  explicit descriptors. No host home, D-Bus/container sockets, job-store mount,
  writable toolchain/cache, application files or metadata endpoint reachability.
  Tool files/directories and their canonical ancestors must be owned by root or
  the dedicated controller UID. Shared-writable ancestors are refused except
  sticky parents protecting the next owned component; tool trees themselves
  must never be shared-writable. Host administrators/controller ownership remain
  trusted, and digest checks do not replace trusted installation or custody.
- Before source is released, require fully enforced Landlock filesystem
  restrictions (ABI v3 rights) for fixed tools, runtime files and the bounded
  workspace. Spawn the trusted compiler helper before the interpreter applies
  Landlock, then restrict each in a separate domain. The helper checks its own
  actual OS boundary and denied access to its parent's descriptors/memory before
  acknowledging readiness. Only then may the interpreter receive source. Regular
  `/proc` and cgroup files are denied by the filesystem policy; the kernel does
  not mediate a process's own anonymous pipes through `/proc`, so cross-process
  protection must use Landlock's ptrace/domain hierarchy. Compiler stdin/stdout
  become null at exec; only capped stderr diagnostics return. Inherited
  descriptors are checked against the exact owned protocol handles.
- Reviewed syscall restrictions in addition to namespaces; no extra privileges
  or unbounded resource allowance on unsupported kernels/platforms.
  The compiler selects the pinned linker by its absolute toolchain path. Rust's
  changed-`PATH` spawn fallback uses a socket pair, so a bare linker name would
  fail under the unchanged no-sockets policy. A real seccomp regression verifies
  absolute-path spawning while socket-pair creation remains denied.
- Structural Wasm validation, bounded compilation, memory/stack/table/fuel
  limits, no guest imports and bounded, normalized output. No deserialization
  of untrusted native/precompiled engine caches.
- Mandatory teardown of the entire job group and workspace. Uncertain cleanup,
  memory/process exhaustion, malformed output and worker loss cannot be graded
  as successful execution.

Runtime probes must check actual restrictions, including namespace separation,
denied filesystem/network/control-socket access, capability state and effective
cgroup limits. Property names, process exit zero or a launcher version alone are
not admission evidence. The development machine accepted a user-service
`PrivateNetwork` request while retaining the same network namespace; a subsequent
namespace launch failed. Those probes are evidence of an unsupported environment,
not permission to remove network restrictions or execute learner code there.
Automated acceptance must provision a supported disposable Linux host explicitly.

The Ubuntu 24.04 acceptance host needs an AppArmor namespace permission for its
trusted launcher. The hosted test provisions a root-owned private copy of the
distro Bubblewrap executable and a separately named, checksum-reviewed copy of
Ubuntu's `bwrap-userns-restrict` profile, attached only to that launcher path.
Its child profile denies capabilities. This setup is confined to the disposable
host; it never disables AppArmor or changes a global namespace sysctl. The runner
still requires its own namespace, capability, seccomp, Landlock and cgroup probes.
Ordinary workstations are not reconfigured by the runner or local test suite.

The domain split addresses the kernel's documented [special-filesystem
limitation](https://docs.kernel.org/userspace-api/landlock.html#special-filesystems).
A failed denial probe blocks source release; removing that probe without a
replacement boundary is not an acceptable compatibility fix.

## Integrity, grading and recovery

An immutable request binds protocol/profile version, tenant, course, learner,
exercise/grader revision, source digest, pinned tools, resource policy and local
submission ID. Reusing an ID with different content fails. Database content is
authenticated before use and bound to indexed identity/configuration. Source and
expected answers do not appear in Debug, routine events, metric labels or errors.

Claiming a job creates a bounded durable lease and execution nonce before launch.
Worker output contains bounded case values/traps, never authoritative expected
answers or a caller-controlled `passed=true`. Trusted grading uses the exact
stored exercise revision. Result acceptance binds the current lease/request and
refuses stale, expired, cancelled, cross-tenant or incompatible receipts. A
replayed receipt cannot award credit twice. Compiler diagnostics are bounded,
treated as untrusted text and escaped for any HTML/terminal presentation.

Cancellation records denial before process termination and fences any late
result. On restart, reconcile persisted work and owned job groups; do not assume
that a missing connection or process means compilation/execution succeeded.
Pure functions permit deliberate bounded re-execution after confirmed teardown;
there is no exactly-once external side-effect claim. Retention covers encrypted
source, workspaces, artifacts, diagnostics and result/idempotency tombstones.
Restored state needs clock/configuration/lease reconciliation, not silent reuse
of stale authority. Operators own encrypted backups and physical storage erasure.
Authorized application maintenance expires queued source independently of runner
availability, without touching leased work. A withdrawn grader can be removed
only after all referencing job records have passed retention and been purged.
Removed immutable revision identifiers must not be reused.

## Adversarial acceptance required

The executable test plan includes malformed/oversized source and Wasm, incompatible
ABI/imports, infinite loops, recursion, memory/table/output/disk/process exhaustion,
compiler denial of service, filesystem traversal/symlinks, environment and file
descriptor leakage, network/metadata/control-socket denial, concurrency, worker
restart/loss, stale-result fencing, cancellation/expiry, idempotency collisions,
retention and actual application submission/status/result access. Include both
working Rust examples and failing submissions; a mocked receipt cannot pass the
execution/isolation gate.

The platform matrix and named profile remain experimental until the required
tests and independent isolation review exist. No claim of risk-free hostile-code
execution, complete Rust/Rullst language support or generic secure CTF hosting
follows from the contracts or this decision.

## Primary implementation references

- [Wasmi 2.0 API and feature boundaries](https://docs.rs/wasmi/2.0.0/wasmi/)
- [Wasmi configuration and validation defaults](https://github.com/wasmi-labs/wasmi/blob/v2.0.0/crates/wasmi/src/engine/config.rs)
- [Bubblewrap policy responsibilities and limitations](https://github.com/containers/bubblewrap/blob/main/README.md)
- [Wasmtime security boundaries, including compilation denial of service](https://docs.wasmtime.dev/security-what-is-considered-a-security-vulnerability.html)
- [Linux cgroups v2](https://docs.kernel.org/admin-guide/cgroup-v2.html)
- [Landlock Rust bindings and enforcement status](https://docs.rs/landlock/0.4.7/landlock/)
- [Linux seccomp filter](https://docs.kernel.org/userspace-api/seccomp_filter.html)

Reviewed September 20, 2026. Runtime/toolchain updates require fresh policy and
compatibility review; benchmark claims from runtime projects are not Rullst
performance or isolation evidence.
