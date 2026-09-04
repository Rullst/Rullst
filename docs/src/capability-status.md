# Simple capability status

This is the compact view of Rullst's canonical M1–M39 programme. It is derived
from the root [ROADMAP](../../ROADMAP.md); that roadmap and the
[capability ledger](capability-ledger.md) retain the evidence and limitations.
The labels here deliberately do not turn partial foundations into completed
features.

## v12 RC engineering snapshot — 4 September 2026

This functionality inventory is deliberately separate from release quality.
All 15 non-IoT crates currently meet the approved A floor and `rullst-iot`
meets its approved B exception. All 15 active crates have also reached their
higher audited local ceiling: **1,509/1,509 local campaign points are backed by
repository evidence (100%), with zero planning points remaining**. The exact
SHA still earns those dimensions only when its conditioning gates pass. The
broader [v12 release programme](v12.md) estimates RC readiness at 86.5%; therefore this
is still a **NO-GO**, not a release announcement.

Coverage is a separate RC gate. Codecov measured candidate `704b6d4d` at
90.03% across the whole repository and 91.30% for the `framework_libraries`
component, so both candidate views exceed their zero-tolerance 90% targets.
This is valid candidate evidence, not evidence for a future commit or tag; the
same gates must pass again on the exact frozen RC SHA.

The latest committed checkpoint `30c3251a` completed all **23/23 automatic
workflow files** successfully, including the full all-feature workspace suite
on Linux, macOS and Windows. That is strong cross-platform candidate evidence,
but the manually dispatched heavy gates and the exact future RC commit still
remain separate requirements.

| Coverage view | Audited checkpoint | RC meaning |
| :--- | :---: | :--- |
| Whole repository | **90.03%** (74,032/82,227) on `704b6d4d` | **Passing on the candidate.** This primary public number includes CLI and proc-macro production sources and is 27 covered lines above the exact cut. |
| Framework libraries | **91.30%** (55,932/61,265) on `704b6d4d` | Passing its separate component gate; it does not replace the whole-repository result. |
| v12 RC requirement | **at least 90% in both views** | Candidate requirement met; must be reproduced by Codecov on the exact frozen RC commit, together with at least 90% patch coverage. |

The two percentages are neither conflicting measurements nor values to
average: they answer questions about different path sets. Rullst must keep the
whole-repository result primary and must not reuse this candidate's passing
result as proof for a later SHA that Codecov has not measured.

The latest ceiling gain is the umbrella `rullst` facade's dedicated shared-local
SQLite profile. It composes Auth revocation, Capital quota, encrypted Connect
tokens, Mail suppression, encrypted Messaging and Core queueing behind
aggregate lifecycle readiness, then proves restart/idempotency, plaintext
secret exclusion and isolated fail-closed corruption. It deliberately does not
claim a cross-subsystem transaction, whole-file online consistency, key/backup
operations or multi-host coordination.

`rullst-ai` has now earned its audited 95/A local ceiling. In addition to strict
opt-in OpenAI-compatible SSE/cancellation, `AuditDeliveryClient` supplies
bounded HMAC-authenticated export and `AdaptiveAiEvaluator<P>` supplies
bounded multi-turn feedback, explicit pass/fail/inconclusive results and a
raw-content-free JSON report. Receiver operations, non-compatible provider
protocols, exact live-model results and corpus quality remain external or v13
work rather than being mislabeled as v12 guarantees.

## `rullst` facade versus `rullst-core`

| Package | Role | Typical user choice |
| :--- | :--- | :--- |
| `rullst-core` | Low-level runtime engine: HTTP server, routes, lifecycle, queue/realtime, storage/cache and the default browser-security baseline. It deliberately does not aggregate every domain crate. | Use directly when a library/application wants only the runtime primitives and explicit dependencies. |
| `rullst` | Ergonomic umbrella facade. Cargo features re-export Core plus selected ORM, Auth, Security, AI, Mail, Capital, Studio, Nexus, Messaging and IoT APIs through one dependency. It also exposes the browser/WASM surface used by web-first applications; Omni packaging itself is a CLI workflow, not a re-exported crate. Its maturity cannot exceed the crates selected underneath it. | Use for most Rullst applications and enable only the required features. |

## Documentation release gate

Before the v12 RC is tagged, the complete repository documentation remains an
explicit review gate: build the mdBook, compile the Rust snippets sourced from
all public tutorials, validate local links and anchors, reconcile commands,
features and version examples with the frozen manifests, and manually review
the upgrade guides and external-provider boundaries. A green documentation
build proves structural consistency, not that every external service or store
workflow was homologated.

| Label | Meaning |
| :--- | :--- |
| ✅ **Implemented** | The stated bounded scope exists and has automated evidence. |
| 🟡 **Still to implement — partial** | A useful foundation exists, but the complete milestone does not. |
| ⏳ **Still to implement — not started** | No implementation sufficient for the milestone exists. |
| 🚫 **Impossible as promised** | An absolute outcome cannot be established by framework code alone or is not a responsible technical guarantee. |

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
| M31 | Aerospace/autonomous/defence systems | ⏳ Separate safety-critical programme; outside the web framework |
| M32 | Axum/Tower escape hatches and proc-macro diagnostics | ✅ Implemented — bounded |
| M33 | Server-side declarative SaaS entitlements | ⏳ Still to implement — not started |
| M34 | Schema-driven TypeScript/React/Dart/Swift SDKs | ⏳ Still to implement — not started |
| M35 | Distributed OpenTelemetry waterfall in Studio | 🟡 Still to implement — partial |
| M36 | Read-only explainable natural-language SQL assistant | ⏳ Still to implement — not started |
| M37 | Reviewable one-click error-console patch workflow | 🟡 Still to implement — partial |
| M38 | Vendor-specific SQLite replica/synchronization profile | ⏳ Still to implement — not started |
| M39 | Optional self-hosted `rullst-gateway` load balancer | ⏳ Still to implement — separate v13 research/foundation; no managed-cloud parity claim |

Current planning snapshot: **5 implemented, 24 partial, and 9 not started**
inside the 38-milestone web-framework horizon. M31 is excluded because it is a
separately governed safety-critical programme. The weighted planning estimate
is 44.7% complete and 55.3% remaining; this is not v12 release readiness and
the 33 milestones without strict closure are not 33 blockers for v12.0. The
[v12 programme](v12.md) owns release gates, while the root roadmap assigns
confirmed v12 defects to `12.0.x` maintenance and all additive capability work,
research or major contracts to v13.

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
