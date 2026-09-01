# Simple capability status

This is the compact view of Rullst's canonical M1–M38 programme. It is derived
from the root [ROADMAP](../../ROADMAP.md); that roadmap and the
[capability ledger](capability-ledger.md) retain the evidence and limitations.
The labels here deliberately do not turn partial foundations into completed
features.

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
