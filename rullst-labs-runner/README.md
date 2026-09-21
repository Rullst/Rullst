# Rullst Labs Runner

Unpublished experimental v13 candidate. The controller/worker implementation and
local contract tests exist; real isolated acceptance, independent security review
and release admission remain outstanding. Do not present this as a production
sandbox or a general Rust/Rullst application hosting service.

This package is a separately deployed Linux x86-64 executable. It must never be a
dependency of the HTTP application. `rullst-labs` owns the application-side
exercise, authorization, encrypted job, receipt and exact-grading contracts.

## Selected profile

The first profile accepts only bounded Rust 1.96.0 pure-function exercises with
`solve(i64, i64) -> i64`, compiles to `wasm32-unknown-unknown` and interprets the
validated module with pinned Wasmi 2.0.0. There are no Cargo dependencies/build
scripts, arbitrary shell strings, WASI, imports or native server processes.
The pinned standard library requires reference-type instruction decoding;
only bounded `funcref` tables are accepted and host imports remain prohibited.

Every attempt requires:

- An owned delegated cgroups v2 subtree with memory, pids and CPU controllers.
  Each job has 1 GiB native memory, no swap, 32 processes, one CPU's bandwidth
  and a controller wall deadline. An inert bootstrap is attached before release.
- Bubblewrap with distinct user/PID/mount/network/IPC/UTS/cgroup namespaces,
  disabled further user namespaces, no capabilities, no-new-privileges, a new
  session and parent-death handling.
- Pinned regular, non-shared-writable tools/runtime files; a read-only root,
  isolated `/proc`, read-only limits and a 64 MiB disposable workspace. No host
  home, application files, database, secret environment or control sockets.
- Reviewed seccomp restrictions and fully enforced Landlock ABI v3 filesystem
  rights. After trusted preflight, descendants cannot reopen `/proc` or cgroup
  files. Only the fixed tool/runtime tree can execute; the workspace cannot.
- Actual namespace, resource, capability, filesystem, descriptor, compiler and
  network-denial observations before accepting student-controlled input.
- Bounded compilation/output/artifact/translation, store/table/stack/fuel limits,
  a fresh Wasm instance per case and mandatory whole-group teardown.

Neither accepted configuration options nor a successful launcher exit proves
isolation. Unsupported machines refuse queued work after the real no-source
preflight; no fallback removes a required control. The development workstation
is currently unsupported for this profile.

## Controller commands

```text
rullst-labs-runner describe-profile ROOTFS BWRAP CGROUPS RECEIPT-SEED
rullst-labs-runner doctor CONFIG.json
rullst-labs-runner run-once CONFIG.json
```

`describe-profile` records tool identities and the dedicated public receipt key;
it does not validate runtime enforcement. `doctor` runs real probes without
student source and requires teardown. `run-once` repeats preflight, reconciles
expired/cancelled work, leases at most one job and performs isolated execution.
A supervisor may schedule this bounded command. The internal bootstrap/worker
commands are implementation details, not application or public network APIs.

The operator supplies a private configuration with `linux` and `plane` objects.
`linux` contains canonical `rootfs`, `launcher`, `cgroups` paths and the pinned
`LinuxExperimental` profile. `plane` contains the dedicated database path,
namespace/capacities and paths to separate 32-byte content and receipt keys.
Key files must be owned private regular files, mode 0600. The application receives
the content key and pinned public verification key; only the controller receives
the signing seed. Neither secret enters the untrusted worker.

The rootfs contains `runner`, the selected `toolchain` and minimal `runtime/lib`
and `runtime/lib64` trees. Its toolchain has rustc, the Wasm standard library,
Rust linker and required shared libraries. Trusted installation/provenance and
filesystem ownership are operator responsibilities; arbitrary downloaded tools
are not trusted merely because a hash can be computed.

The repository's disposable preparation helper creates fixture keys/configuration
and these exact trees from already trusted installed tools. The hosted acceptance
helper runs inside its own delegated unprivileged systemd service and never
modifies global namespace policy. These test helpers do not deploy a production
runner or enroll a real provider/account.

## Results and failure recovery

Expected answers stay in the trusted grader. Worker responses contain bounded
values/traps, never a passing-grade authority. The controller signs only the
exact attempt output and observed profile, after confirmed teardown. The job
plane validates the pinned key, current nonce/revision, source/profile/exercise,
receipt timing, withdrawal and cancellation before grading.

OS exhaustion, malformed output, cancellation, timeout and uncertain cleanup do
not become successful grades. Guest fuel/memory/stack traps are explicit failing
case feedback. The first controller fences and cleans abandoned work without an
automatic retry; the dedicated API supports one deliberate retry after cleanup.

Logs omit source, expected answers, keys and raw worker stderr. Diagnostic text
is still untrusted and can quote the learner's source; escape it for rendering.
Operators own process supervision, supported kernel/runtime updates, backups,
physical erasure and incident handling. Independent isolation review remains a
requirement before any production-ready hostile-code claim.
