# v12 release audit follow-up

Status: **in progress; RC is NO-GO while the findings and final gates below are
open**. This report supersedes blanket readiness interpretations of the earlier
local-ceiling campaign. That campaign and hosted coverage measurements remain
historical evidence for their recorded commits, not proof of the current tree.

## Baseline and method

The integration baseline is `7743bab3` on
`fix/cli-logo-animation-speed`, including the second-computer report in
[CLIFIX.md](../../CLIFIX.md). Its earlier “uncommitted” wording describes the
remote review session; that delivery is now committed and fetched here.

Review covers every published crate. IoT receives only a light triage under the
owner's explicit v12 exception. Each deep review traces public inputs through
validation, state and side effects, compares documented behavior, and adds
negative regressions for reproduced defects. Existing tests alone do not close
a finding. Regressions are run before and after corrections when feasible.

This is repository-owned code review and testing, not an independent security
assessment, provider certification, or a claim that every line or deployment
configuration has been exhaustively analyzed. Tests are serialized on the
memory-limited local machine. The final local workspace gate completed after
the correction batch.

## Coverage ledger

| Crate / surface | Review scope | Current status |
| --- | --- | --- |
| `rullst-orm` | Projection identifiers, empty-set predicates, tenant/global scopes, transactions, policy mutations, nested queries and search | Reproduced isolation/transaction defects corrected; focused default/strict-SQLite/Redis regressions and the final all-feature workspace gate are green; live external-backend matrices remain release evidence |
| `rullst-orm-macros` | Generated SQL bindings, parser diagnostics, portable identifiers and scope generation | Corrected generated contracts; 41 unit tests, one smoke test and 24 compile-fail cases green |
| `rullst-core` | HTTP security composition, CSRF, lifecycle and development state ownership | CSRF/security composition 21 tests green; four reload tests and actual Node client behavior tests green |
| `cargo-rullst` | Remote CLI handoff, public profile accuracy, supervised restart, generated contracts | Supervisor, dashboard, command-behavior, public-profile and materialized blueprint gates are green; snapshot launch now retries bounded transient Linux executable-busy races |
| `rullst-auth` | JWT expiry/revocation, encrypted sessions, role guards, passkey/SQLite cancellation | Corrections green: 60 library tests and five durable JWT integrations |
| `rullst-security` | WebSocket origin enforcement, middleware readiness, bounded redaction, crypto/input policies | 159 library tests and two Tower tests green; final rate-limit run passed 11 tests including two added afterward (161 library cases now) |
| `rullst-connect` | OIDC claims/nonce, refresh semantics, callback state and token lifetimes | Corrections green: 204 library tests with Axum-session and SQLite features |
| `rullst-capital` | Provider side effects, pricing, charge binding, authenticated payload schema and signature protocols | 99 library plus 22 integration tests green with Actix; one later Actix duplicate-header regression also green |
| `rullst-nexus` | Admin transport/origin/authorization and tenant/audit boundaries | Corrections green: 50 library plus 11 integration tests, including real SQLite tenant/audit cases |
| `rullst-studio` | Local operator boundary, handoff layout/telemetry changes, dynamic HTML | Static boundary review found no additional reproduced defect; 48 library tests, integrations and the final workspace gate are green |
| `rullst-macros` | Escaping/raw HTML, generated handler/runtime contracts | Static trust-context review completed; no new reproduced defect; final all-feature workspace tests and doctests are green |
| `rullst-ai` | Provider/mock separation, redirect handling, response limits and tool-policy boundaries | 102 library tests green, including real local HTTP regressions for all five native transports |
| `rullst-mail` | Provider side effects, transport limits, attachments/headers, suppression and delivery evidence | 98 library tests green with SQLite, including actual HTTP and cancelled-write regressions |
| `rullst-messaging` | Publication/lease/retry/idempotency, local durability and outbox composition | Cancellation defect corrected; focused SQLite evidence and final all-feature workspace coverage, including the optional ORM outbox relay, are green |
| `rullst` | Facade feature wiring, composed subsystem and tutorial contracts | Static facade/feature review; existing verified-TLS composition retained; final composed all-feature workspace tests and doctests are green |
| `rullst-iot` | Manifest/public capability honesty only; no hardware or deep audit | Light review complete; README/manifest agree on helper/simulator/transport boundaries; approved scope exception |

Counts above describe separate focused runs and overlap; do not sum them into an
invented coverage metric. In addition, the complete workspace test suite passed
with all features, and workspace Clippy passed with all targets, all features
and `-D warnings`.

## Reproduced defects and correction boundaries

| Area | Observed failure | Correction and remaining boundary |
| --- | --- | --- |
| ORM projections | Safe-looking `select`/`pluck` accepted SQL expressions | Validate safe projection identifiers; deliberate raw SQL remains a caller-owned escape hatch |
| ORM membership and scopes | Empty `IN` matched all rows; `OR` escaped tenant/global/soft-delete constraints | Empty sets are false predicates; group mandatory scopes separately and preserve nested query errors |
| ORM search | Local and external Scout paths bypassed model scopes; empty external results could select ID zero | Start from the scoped query, reject missing context before provider calls and bind provider IDs through empty-aware membership |
| ORM transactions | `pluck`, streaming/eager paths or mutation callbacks could bypass/wait on an already borrowed transaction | Use the managed executor; release query locks before callbacks/eager work where supported; unsupported mutation-callback reentry returns a typed validation error instead of hanging |
| ORM mutations/macros | Bulk delete bypassed model policy; keyset iteration could escape its cursor; unsupported identifiers reached malformed generated code | Reject unauthorized bulk mutation, group keyset predicates, propagate errors and emit compile diagnostics for unsupported identifiers/scopes |
| Durable local stores | Cancellation during manual `BEGIN` left uncommitted state in a pooled connection | SQLx RAII transactions in Auth JWT/passkey, Mail suppression and Messaging; cancellation racing dispatched commit still requires reconciliation |
| Authentication | Expired revoked JWT could become accepted during skew allowance | Enforce hard expiry independently of permitted clock skew; no distributed revocation claim |
| OAuth/OIDC | Missing claims, nonce downgrade, refresh-token confusion, empty callback state and invalid lifetimes | Strict provider-specific validation and checked positive bounded lifetimes; real accounts and distributed one-shot callback storage still need external evidence |
| Security middleware | HTTP/2 CONNECT bypassed WebSocket origin policy; cloned Tower services lost acquired readiness | Apply the origin guard to the extended method and call the ready service instance |
| Abuse controls/logs | Reset zero-limit admission, counter overflow, concurrent capacity escape and redaction suffix leakage | Checked bounded admission, atomic capacity/reclamation and bounded fail-closed redaction; controls remain process-local |
| Core CSRF | Empty proofs accepted; valid split Cookie fields rejected; duplicate proofs ambiguous | Nonempty bounded unique tokens, multi-field cookie parsing and exact supported form media type; unsigned double-submit is not a session-signed CSRF scheme |
| Nexus operator access | An absolute HTTPS URI impersonated verified TLS; local Host/Origin boundary incomplete | Require the private verified-transport capability and validate local browser Host/Origin; deployment proxies must supply the correct trusted adapter |
| Capital live operations | Fabricated portals/no-op mutations, four hardcoded prices and undocumented mock aliases | Explicit unsupported errors for unimplemented live behavior; deterministic mocks only through documented mock credentials; consult the crate's provider-method matrix |
| Capital receipts/webhooks | Incomplete authenticated payloads inferred active/paid; charge identity insufficiently bound; Polar/MP body-only signatures did not represent their protocols | Validate required event/status/charge bindings; bounded Polar header-based Standard Webhooks verification; incompatible legacy live signature paths fail closed, including MP until its full provider verification is implemented |
| AI/Mail transports | Redirects forwarded private request content; AI JSON unbounded; suppression cancellation leaked state | Pooled redirect-disabled clients, connection/request budgets, bounded native AI responses and SQLx rollback ownership; native custom endpoints remain trusted operator configuration |
| Public DLL reload | Windows LMS loaded an independent ORM/runtime state and unsafe cross-runtime workarounds were proposed | Remove public DLL generation and use directly linked supervised restart; retained legacy loader is experimental and not a stable Rust ABI |

The adversarial regressions use local databases, mock keys, signed synthetic
tokens and loopback HTTP servers—not real credentials or real payment requests.
Provider capability corrections are observable behavior changes: callers must
handle explicit errors where previous code returned misleading success.

## Website, README and first-run documentation

The organization root website and the framework Pages site were different
deployments. The old organization site still described `main` as v5 and `dev`
as v12, and its privacy page asserted unverified worldwide legal compliance.
Both entry points now have prepared matching source, with separate deployment
receipts still required. The new copy keeps v12 unreleased and v5 end-of-life.

The landing uses local CSS/JavaScript/images, finite reduced-motion-aware
animation, thirteen owner-supplied social links and a concrete privacy notice.
No analytics, social embeds or browser storage were added. Benchmark templates
replace remote fonts with system fonts and pin Chart.js with integrity metadata;
the remaining jsDelivr request is disclosed. The README preserves the top and
bottom dedication, workflow dashboard, genuine coverage/Scorecard badges and
evidence boundaries. It corrects the obsolete frontend-profile advertisement.

The [beginner learning page](start-here.md) links existing authoritative
tutorials instead of creating another competing API reference. Initial guides
clarify matching CLI installation, optional persistence, first-build time,
actual generator paths and how to verify a visible result.

Verified locally: `mdbook build docs`,
`python3 .github/validate-site.py`, `node --check docs/site.js`, and
`node .github/site-browser-smoke.mjs`. The Chromium test passed desktop,
390/320-pixel layouts, keyboard/mobile menu behavior, clipboard success/denial,
privacy disclosure, reduced motion, no-JavaScript navigation and no external
landing requests or browser storage. The exported organization site also passed
with `--organization-site`; this is not a WCAG or cross-browser certification.

## Development reload decision

The public v12 development loop uses **supervised process restart**. Both
`cargo rullst dev` and `cargo rullst dash` enable it automatically; plain
`cargo run` remains ordinary execution. The wizard no longer asks for a DLL
profile, and the legacy scaffold flag fails with migration guidance.

A successful build precedes stopping the existing child. Compilation errors
leave that child serving. Each process runs an owned executable snapshot, so a
Windows executable lock does not prevent the next Cargo build. The browser
refreshes through a same-origin generation probe after a new server responds.
State in memory resets; this does not promise zero-downtime deployment.

The [tutorial](tutorials/51-authenticated-hot-reload.md) explains the contract.
The v13 decision is evidence-driven: compare measured reload time, failure
recovery, process cleanup, memory and state ownership across databases and
operating systems before considering a different architecture.

## Evidence still required

- Review all changed paths together and freeze the candidate; the broad review
  above is bounded repository-owned evidence, not an independent audit.
- Finish the real HTTP/browser acceptance pass for representative generated
  blueprints; materialized compile/test contracts are already green.
- Review CI/dependency/security alerts and run the applicable manual release
  matrices on the actual candidate commit.
- Repeat package/preflight, site/browser and documentation checks on that
  candidate.
- Reassess quality/readiness using these results; do not carry forward 91.8%
  readiness or 100% local-ceiling completion as current audited facts.

## Residual limitations for the next reviewer

- HTML escaping does not make `RawHtml`, custom escaping implementations,
  JavaScript contexts or URL policies safe automatically. RPC parameters are
  untrusted inputs, not identity assertions.
- Some direct session/passkey helpers depend on caller input-size bounds even
  when HTTP middleware limits requests. Cookie isolation matters for the
  unsigned double-submit scheme. Local rate controls are not distributed limits.
- OIDC fixtures do not establish live-account acceptance, JWKS stampede
  resistance or multi-host atomic callback consumption.
- NFS-e remains preparation/offline evidence, not official fiscal authorization.
  Payment fixtures do not homologate providers or fully model every event schema.
- SQLite cancellation tests prove rollback ownership before commit; they cannot
  make cancellation during a dispatched commit into exactly-once knowledge.
- Some older route smoke tests accept unavailable-database responses and prove
  route presence only. Real SQLite behavioral tests are identified separately.
- IoT received only the agreed manifest/README review. No physical devices,
  browser WebAuthn ceremony, store signing, live provider or deployment proxy
  certification was performed.

## Evidence handling

Confirmed security defects remain local until corrections and regression
evidence are ready for a coordinated commit. Findings are classified by impact,
not by whether the previous scorecard happened to be green. Provider behavior
that cannot be completed within the current v12 contract must fail explicitly
and be documented as unsupported; mock success is not live-provider evidence.

## Repeatable focused verification receipts

These focused commands and the final local preflight succeeded during the
September 5–6 correction batch:

```bash
CARGO_BUILD_JOBS=1 cargo test -p cargo-rullst --lib -- --test-threads=1
CARGO_BUILD_JOBS=1 cargo test -p rullst-core --lib server::dev_reload -- --test-threads=1
node rullst-core/src/server/dev_reload/client_tests.cjs
CARGO_BUILD_JOBS=1 cargo test -p rullst-nexus --lib --tests -- --test-threads=1
CARGO_BUILD_JOBS=1 cargo test -p rullst-security --lib rate_limit::tests -- --test-threads=1
CARGO_BUILD_JOBS=1 cargo test -p rullst-ai --lib
CARGO_BUILD_JOBS=1 cargo test -p rullst-mail --features sqlite --lib
CARGO_BUILD_JOBS=1 cargo test -p rullst-connect --features axum-session,sqlite --lib
CARGO_BUILD_JOBS=1 cargo test -p rullst-capital --features actix --lib --tests
CARGO_BUILD_JOBS=1 cargo test -p rullst-capital --features actix --lib middleware_rejects_duplicate_standard_webhook_headers
CARGO_BUILD_JOBS=1 cargo test -p rullst-messaging --features sqlite -- --test-threads=1 --quiet
CARGO_BUILD_JOBS=1 cargo clippy -p cargo-rullst -p rullst-core --lib -- -D warnings
CARGO_BUILD_JOBS=1 cargo clippy --workspace --all-targets --keep-going -- -D warnings
CARGO_BUILD_JOBS=1 cargo test --workspace --all-features
CARGO_BUILD_JOBS=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
git diff --check
bash .github/check-historical-roadmap-ledger.sh
bash .github/check-crate-architecture.sh
```

ORM default/strict-SQLite/Redis and macro compile-fail results are recorded in
the coverage ledger. Live-service backend verification remains distinct from
the passing all-feature compile, lint and local workspace gates.
