# Simple capability status

This is the compact view of Rullst's canonical M1–M38 programme. It is derived
from the root [ROADMAP](../../ROADMAP.md); that roadmap and the
[capability ledger](capability-ledger.md) retain the evidence and limitations.
The labels here deliberately do not turn partial foundations into completed
features.

## v12 RC engineering snapshot — 3 September 2026

This functionality inventory is deliberately separate from release quality.
All 15 non-IoT crates currently meet the approved A floor and `rullst-iot`
meets its approved B exception. Twelve of the 15 active crates have also reached
their higher audited local ceiling. The remaining ceiling work is
`rullst-connect`, `rullst-ai`, and the `rullst` facade: **1,493/1,509 local
campaign points are evidenced (98.9%), with 16 points remaining**. The broader
[v12 release programme](v12.md) estimates RC readiness at 70.2%; therefore this
is still a **NO-GO**, not a release announcement.

Coverage is a separate open RC gate. The complete public Codecov checkpoint
used for this snapshot (`28e2cea9`) records 84.97% across the whole repository
and 91.26% for the `framework_libraries` component. Both now target at least
90%; the overall gap must be closed through behavior-asserting tests,
especially around CLI paths, rather than metric-only execution.

| Coverage view | Audited checkpoint | RC meaning |
| :--- | :---: | :--- |
| Whole repository | **84.97%** | **Failing / release blocker.** This is the primary public number and includes the CLI and proc-macro production sources. |
| Framework libraries | **91.26%** | Passing its separate component gate, but it cannot compensate for or replace the whole-repository result. |
| v12 RC requirement | **at least 90% in both views** | Must be reproduced by Codecov on the exact frozen RC commit, together with at least 90% patch coverage. |

The two percentages are neither conflicting measurements nor values to
average: they answer questions about different path sets. Until the first row
reaches 90%, Rullst must not present the second row as the repository's total
coverage.

The latest bounded M12 gain is the opt-in `AuthenticatedSiemSpool`: a
single-writer HMAC-SHA256-chained local journal with explicit active/historical
key rotation and exact forgery/order/restart negatives. M12 remains partial
because trusted whole-tail checkpoints, multi-writer operation, external SIEM
delivery/acknowledgement, independent audit and certification are not supplied
by that local contract.

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

Current planning snapshot: **5 implemented, 24 partial, and 8 not started**
inside the 37-milestone web-framework horizon. M31 is excluded because it is a
separately governed safety-critical programme. The weighted planning estimate
is 45.9% complete and 54.1% remaining; this is not v12 release readiness and
the 32 milestones without strict closure are not 32 blockers for v12.0. The
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
