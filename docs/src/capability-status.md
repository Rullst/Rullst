# Simple capability status

This is the compact view of Rullst's canonical M1–M41 programme. It is derived
from the root [ROADMAP](https://github.com/Rullst/Rullst/blob/v13/ROADMAP.md); that roadmap and the
[capability ledger](capability-ledger.md) retain the evidence and limitations.
The labels here deliberately do not turn partial foundations into completed
features.

## v12 stable snapshot — 15 September 2026

Stable `12.0.0` was published from commit `eb11f892`. See the
[release record](v12.md) for the immutable tag and publication receipt.

The [Rust CI scorecard](https://github.com/Rullst/Rullst/actions/runs/34895536764)
for that commit awarded **94/100 (A)** to the repository, with all fifteen
non-IoT packages at A and IoT at its approved B exception. These are
repository-owned audit scores constrained by test results, not a percentage
of all planned functionality or an independent certification. The
[quality scorecard](quality-scorecard.md) explains the scale.

The [stable coverage run](https://github.com/Rullst/Rullst/actions/runs/34895523751)
recorded the following local LLVM/LCOV gates before the Codecov upload:

| Coverage view | Stable-source measurement | Required floor |
| :--- | :--- | :--- |
| Whole repository | **90.3220%** (79,813/88,365 lines) | **90%** |
| Framework libraries | **90.6186%** (60,014/66,227 lines across 436 files) | **90%** |

These are distinct path sets, not values to average. The public Codecov badge
tracks the current branch and may change as its source and report processing
change. Do not relabel an older checkpoint as the latest measurement.

The [release audit](v12-release-audit.md) links the stable platform, property,
Miri, Kani, fuzzing, sanitizer and publication results. New maintenance commits
must satisfy their applicable gates; the published release cannot certify
future source changes.

## `rullst` facade versus `rullst-core`

| Package | Role | Typical user choice |
| :--- | :--- | :--- |
| `rullst-core` | Low-level runtime engine: HTTP server, routes, lifecycle, queue/realtime, storage/cache and the default browser-security baseline. It deliberately does not aggregate every domain crate. | Use directly when a library/application wants only the runtime primitives and explicit dependencies. |
| `rullst` | Ergonomic umbrella facade. Cargo features re-export Core plus selected ORM, Auth, Security, AI, Mail, Capital, Studio, Nexus, Messaging and IoT APIs through one dependency. It also exposes the browser/WASM surface used by web-first applications; Omni packaging itself is a CLI workflow, not a re-exported crate. Its maturity cannot exceed the crates selected underneath it. | Use for most Rullst applications and enable only the required features. |

## Documentation maintenance gate

Stable v12 completed the documentation release gate: the mdBook and Rust
snippets sourced from public tutorials built, local links and anchors were
validated, commands/features/version examples were reconciled with the frozen
manifests, and upgrade/provider boundaries were reviewed. Compatible v12
maintenance must keep those checks green. A green documentation build proves
structural consistency, not that every external service or store workflow was
homologated.

## Canonical milestones

| ID | Capability | Simple status |
| :---: | :--- | :--- |
| M1 | CLI and `make:*` generator matrix | 🟡 Still to implement — partial |
| M2 | Fast linkers and measured build-time improvements | 🟡 Still to implement — partial |
| M3 | Escape hatches, granular features, diagnostics, and ejection | 🟡 Still to implement — partial |
| M4 | `make:resource` and local error console | ✅ Implemented — scoped |
| M5 | mdBook, OpenAPI, and typed client generation | 🟡 Still to implement — partial |
| M6 | ORM parity and Turso/libSQL profile | 🟡 Still to implement — partial |
| M7 | Portable edge runtime, distributed data, and safe upgrades | 🟡 Still to implement — partial |
| M8 | Explain-and-approve index recommendations | ⏳ Still to implement — not started |
| M9 | Local auth, OAuth/OIDC, TOTP, passkeys, and WebAuthn | 🟡 Still to implement — partial |
| M10 | Mail, DTO validation, distributed rate limits, and Shield | 🟡 Still to implement — partial |
| M11 | Nexus, Omni, billing, and entitlements | 🟡 Still to implement — partial |
| M12 | Defence-in-depth security programme | 🟡 Still to implement — continuous/partial |
| M13 | Audited PQC protocols and sandboxed Wasm extensions | ⏳ Still to implement — not started |
| M14 | HTMX-first SSR and real Leptos/Dioxus interoperability | 🟡 Still to implement — partial |
| M15 | Runtime queues/cache/scheduler plus brokered messaging | 🟡 Still to implement — bounded local messaging foundation; remote adapters open |
| M16 | Wasm islands and `#[client_component]` protocol | 🟡 Still to implement — partial |
| M17 | Realtime, object storage, media, and packages | 🟡 Still to implement — partial |
| M18 | LiveView-style server-driven UI | 🟡 Still to implement — partial |
| M19 | Radar, agent schemas, spans, and Prometheus | ✅ Implemented — bounded |
| M20 | Persistent event stream and verifiable ledger semantics | ⏳ Still to implement — not started |
| M21 | Omni frontend protocol and mobile bridge | 🟡 Still to implement — partial |
| M22 | Human-reviewed agentic DevOps recommendations | 🟡 Still to implement — partial |
| M23 | Diagnostic auto-healing recommendations | 🟡 Still to implement — partial |
| M24 | `no_std` IoT frames/packet encoders, signed OTA gate, and durable-counter CAS boundary | 🟡 Still to implement — partial |
| M25 | Embassy-based async embedded integration | ⏳ Still to implement — not started |
| M26 | Guided PaaS/VPS deployment | 🟡 Still to implement — partial |
| M27 | Kubernetes scaffolding and health/readiness probes | ✅ Implemented — scaffolding scope |
| M28 | Compile-time DI and `Inject<T>` | ✅ Implemented — foundation |
| M29 | Scalar playground and complete OpenAPI generation | 🟡 Still to implement — partial |
| M30 | Tonic/gRPC and Protobuf support | 🟡 Still to implement — partial |
| M31 | Aerospace/autonomous/defence systems | ⏳ Separate safety-critical programme; outside the general framework suite |
| M32 | Axum/Tower escape hatches and proc-macro diagnostics | ✅ Implemented — bounded |
| M33 | Server-side declarative SaaS entitlements | ⏳ Still to implement — not started |
| M34 | Schema-driven TypeScript/React/Dart/Swift SDKs | ⏳ Still to implement — not started |
| M35 | Distributed OpenTelemetry waterfall in Studio | 🟡 Still to implement — partial |
| M36 | Read-only explainable natural-language SQL assistant | ⏳ Still to implement — not started |
| M37 | Reviewable one-click error-console patch workflow | 🟡 Still to implement — partial |
| M38 | Vendor-specific SQLite replica/synchronization profile | ⏳ Still to implement — not started |
| M39 | Optional self-hosted `rullst-gateway` load balancer | ⏳ Still to implement — separate v13 research/foundation; no managed-cloud parity claim |
| M40 | `rullst-labs` contracts and an isolated `rullst-labs-runner` | ⏳ Still to implement — separate v13 research/foundation; full offensive CTF arenas require external isolated infrastructure |
| M41 | Privacy controls and proportional age assurance | 🟡 Unpublished foundation — policy, signed evidence, asynchronous replay and shared-local SQLite; consumer journeys and broader privacy work remain open |

Planning labels checked on 20 September 2026: **5 implemented, 25 partial, and
10 not started** inside the 40-milestone framework programme. M31 is excluded
because it is a separately governed safety-critical programme. The 35 milestones
without strict closure have different sizes and overlap existing published
capabilities; their count does not measure remaining engineering effort or
release blockers. Assigning every partial row half credit would not establish
a completion percentage. The
[v12 stable record](v12.md) preserves the completed release identity, while the
root roadmap assigns compatible v12 fixes to maintenance and new capability
work to v13 by default. The
[safe update experience](https://github.com/Rullst/Rullst/blob/v13/ROADMAP.md#safe-update-experience) is a top v13
priority whose compatible opt-in 12.1.0 scope is already published. Version
12.1 does not establish 12-to-13 migration acceptance. The
[26 September delivery plan](v13-delivery-plan.md) selects concrete increments
from this wider programme and records their acceptance requirements.

## Claims that are impossible as framework guarantees

No useful capability above is dismissed merely because it is difficult. The
`impossible` label is reserved for absolute wording that code in this repository
cannot honestly establish:

| Absolute claim | Status |
| :--- | :--- |
| 100% uptime or zero data loss in every deployment | 🚫 Impossible as a framework guarantee |
| Exactly-once arbitrary external side effects | 🚫 Impossible without destination-level idempotency/transactions |
| Zero latency, zero overhead, zero allocations, or universal sub-100ms builds | 🚫 Impossible as a universal guarantee |
| Total memory safety or universal panic-freedom across dependencies, FFI, generated apps, and every input | 🚫 Impossible to prove from this repository alone |
| Automatic fiscal, security, privacy, App Store, or hardware certification | 🚫 Requires independent authorities, environments, and evidence |
| Universal one-click zero-downtime deployment | 🚫 DNS, credentials, migrations, providers, and rollback remain operational inputs |
| “Best/fastest/most secure framework in the world” | 🚫 Not a technical property without dated, reproducible comparative evidence |
| Unattended production mutation that is always safe | 🚫 Approval, scoped authority, audit, recovery, and application policy cannot be removed |

Use the [quality scorecard](quality-scorecard.md) for per-commit engineering
evidence. It is intentionally separate from this functionality view.
