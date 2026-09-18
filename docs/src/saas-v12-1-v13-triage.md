# SaaS findings: v12.1 maintenance and v13 contracts

**Status: SAAS-003 and SAAS-004 implemented in maintenance source on 18 September
2026; final candidate CI is in progress. Provider-account sandbox acceptance,
publication and deployment remain unverified.**

The input is the two reports from `Rullst/examples`, branch
`feat/saas-staging-ai-fixes`, pinned to commit
`d6094dbd67d12ee965ce73dfb539d3ada9b4195a`:

- [saas-improvements-needed.md](https://github.com/Rullst/examples/blob/d6094dbd67d12ee965ce73dfb539d3ada9b4195a/saas-improvements-needed.md)
- [rullst-errors.md](https://github.com/Rullst/examples/blob/d6094dbd67d12ee965ce73dfb539d3ada9b4195a/rullst-errors.md)

Those reports inspect published 12.0.0. This review compares their findings
with framework source at `de77430ccd86c847c98d67002f3478e57637180f` on `v13`.
The table records that initial source review. The implementation notes below
track subsequent corrections. The initial review was inspection, not a fresh
Windows build, resolved feature-graph test, provider sandbox run, or examination
of either deployed SaaS site. Application workarounds do not fix the packages.

The follow-up report is pinned to branch `fix/saas-staging-healthcheck-waf`,
commit `c49d6ee8f1b8df79dca8e9255a87b1a322fdc08b`:
[rullst-errors.md, RULLST-003](https://github.com/Rullst/examples/blob/c49d6ee8f1b8df79dca8e9255a87b1a322fdc08b/rullst-errors.md#rullst-003--shield-blocks-legitimate-machine-clients-by-default).
The report attributes an HTTPS reproduction to the examples team; this review
independently confirms the source cause and exercises a local production router.

The additional `fix/saas-nexus-trusted-tls` report at
[`ac9a6b0625b703de8ca84d0bd10906395f500033`](https://github.com/Rullst/examples/blob/ac9a6b0625b703de8ca84d0bd10906395f500033/saas-improvements-needed.md#app-saas-002--azure-nexus-mount-omitted-the-trusted-tls-capability)
adds APP-SAAS-002: the application omitted Nexus's explicit trusted-TLS
capability behind Azure ingress. The existing framework correctly rejects
Basic authentication without that evidence; forwarding headers alone must not
enable it. The report retains SAAS-001–015 and adds no new framework defect.

The subsequent report is on `examples/main`, pinned to
[`1af10ed4da60a5e101c9f0dd20b2c823f38ac5ee`](https://github.com/Rullst/examples/blob/1af10ed4da60a5e101c9f0dd20b2c823f38ac5ee/saas-improvements-needed.md):

- **APP-SAAS-003:** a rejected duplicate checkout consumed the application's
  new-session limit, and the owner could not resume an already-created session.
  The report describes a correction in the example's application orchestration,
  not the framework limiter. Generated Stripe billing now persists owner-bound
  attempts and retrieves an existing open session before creating another. It
  distinguishes complete, expired and unknown outcomes and tests retries and
  lost responses. Applications adding a new-session quota must count newly
  persisted sessions, return `Retry-After` on quota rejection, and keep any
  all-request abuse limit separate.
- **APP-SAAS-004:** Chromium blocked the local POST's hosted Checkout redirect
  under `form-action 'self'`. The maintenance SaaS generator now adds only
  `https://checkout.stripe.com` to its own CSP, matching its explicit default
  provider. Core's default is unchanged. Other providers need an exact reviewed
  store/custom origin; `make:billing` explains this without overwriting existing
  policy. Server-side URL/session ownership validation remains mandatory.
  Browser regression covers generated policy rendered by Core with a synthetic
  POST/303 and intercepted destinations. It is not deployed SaaS, authentication,
  provider, payment or full generated-application acceptance.

Both integration requirements carry into v13. The report's correction claims
are attributed to the examples repository; no deployed site or live session was
tested here. The CSP rule is described in the W3C
[form-action contract](https://www.w3.org/TR/CSP3/#directive-form-action).

## Completed SAAS-003/004 implementation

Maintenance commit `8c3c8391` and its v13 carry `d44f9520` provide:

- Polar's current product checkout, external customer identity, sandbox endpoint,
  optional typed trusted client IP, response/return-placeholder binding, and
  signed subscription matching. Legacy price-only callers migrate explicitly.
- Generated Stripe customer/checkout intents and session IDs, account/mode scope,
  signed Checkout and subscription events without email ownership, ID-bound
  portal, current-state reconciliation, database revision fencing and atomic
  event/subscription commits on SQLx and Turso. Unknown old outcomes have bounded
  read recovery and an explicit verified customer-recovery function for operators.
- Local evidence: 171 Capital tests with Actix; 341 CLI library tests; ten
  structural contracts; strict Clippy; materialized SQLite/Turso compilation,
  migrations, lost-response/replay/rollback, cross-owner and stale-read rejection,
  replacement subscriptions and process restart. Full candidate CI remains
  distinct from this targeted evidence and from live provider-account testing.

## Implementation checkpoints

The maintenance candidate is
[PR #206](https://github.com/Rullst/Rullst/pull/206). None of these checkpoints
means the complete release or a live-provider journey has passed. Pending items
in older checkpoint descriptions describe that historical point; the completed
implementation above supersedes their SAAS-003/004 containment:

- `20218538`: configured Lemon Squeezy store/variant binding; bounded Stripe
  rotated-signature verification; explicit unsupported Paddle/Polar checkout,
  Wise email transfer and generated live portals; generated HTTP 303 handoffs,
  committed application lockfiles and supported MSVC flags.
- `9976da5b`: shared AI provider configuration, including explicit Groq and
  custom OpenAI-compatible endpoints, for client and Nexus.
- `8b7761a2`: RULLST-003 removes generic clients from the default User-Agent
  blocklist. Local production-router tests retain configurable crawler denial,
  CSRF, payload limits and secure headers. Existing explicit configuration is
  not rewritten. Core library: 238 tests passed; strict default-feature Clippy
  for all Core targets passed. This is not an all-feature workspace result.
- `efb9518b`: Razorpay lifecycle normalization and signed-event negatives;
  127 default-feature Capital tests and strict Clippy for all targets passed.
- Materialized billing tests pass for SQLite and Turso, including 303 redirects,
  cross-owner denial and unavailable live portals before database access. The
  customer/subscription pair now commits atomically; fault injection in either
  table proves rollback and a successful handler retry on both backends.
  Stable provider customer binding, checkout idempotency, provider namespaces
  and atomic inbox/domain state still need implementation and acceptance.
- Additive `StripeCheckoutRequest`/`create_subscription_checkout` binds an
  existing customer, local reference, recurring price, redirects and retry key;
  validates the returned session and line item; and distinguishes local mocks.
  All 132 default-feature Capital tests and strict all-target Clippy passed.
  This is protocol/local evidence; the generated checkout still needs durable
  customer provisioning, attempt persistence and the verified event flow.
- Standalone ORM consumers can disable defaults and enable exactly one strict
  SQLx backend. Generated CRUD and transaction methods passed strict Clippy
  on all three profiles; their normal/build graphs contained only the selected
  SQLx driver. The default `drivers-all` profile retains existing convenience.
  Facade/Studio compositions still need their own opt-in isolation boundary.
- The follow-up full CI run at `e1d8fe4d` exposed unconditional enum codecs in
  backend-exclusive builds. Runtime-gated codecs fix the no-driver all-target
  check, and the expanded standalone consumers now compile enum bindings and
  decoding for PostgreSQL, MySQL and SQLite without importing other drivers.
  These targeted checks do not supersede the next full candidate matrix.
- Stripe subscription normalization now reads Basil item-level periods and
  rejects missing/confused event kinds, identities, states and price items.
  Legacy subscription-level periods and custom plan IDs remain supported;
  contradictory periods fail. Three real-HMAC fixture tests cover those
  branches; 139 Capital tests with Actix enabled and strict all-target Clippy
  passed. Subscription state is still not invoice-settlement evidence.
- The additive verified Stripe event envelope retains event/scope metadata,
  the exact provider status and separate mutation/payload digests without a
  pre-handler replay claim. Four signed-envelope contracts cover tampering,
  scope/mode confusion, mutation identity, delivery variation and explicit mock
  rejection. All 143 Capital tests with Actix and strict Clippy passed locally.
  The generated handler has not yet switched to a durable atomic inbox.
- Additive Stripe customer provisioning binds opaque owner metadata and an
  immutable retry request, with optional contact email and validated response
  mode/identity. Five protocol/negative/mock contracts pass; all 148 Capital
  tests with Actix and strict all-target Clippy pass. This supplies the provider
  operation, not durable intent, account/owner binding or generated integration.
- `SqlStripeEventInbox` atomically retains a verified event's scoped identity,
  mutation digest and outcome with caller-supplied domain SQL. SQLite tests
  cover restart and injected inbox-write failure; shared PostgreSQL, MySQL and
  MariaDB contracts cover retries, conflicting content, eight concurrent
  deliveries, domain rollback, cancellation, immutable capacity and scope/mock
  denial. These pass with both Any and the matching native ORM pools. This is
  database/protocol evidence; customer/attempt persistence, event ordering and
  replacement of the generated handler remain required.
- Subscription retrieval now binds the current Stripe response to a persisted
  customer/owner/price/subscription/mode and preserves its exact provider state.
  Four protocol, negative and mock contracts pass; all 152 Capital tests with
  Actix and strict all-target Clippy pass. The read must still be serialized
  with its domain update; it is not invoice settlement or event ordering.
- The SaaS blueprint now reuses the shared billing models, removing a raw
  PostgreSQL placeholder from subscription lookup. All ten scaffold contracts,
  the four materialized foundation applications and CLI all-feature/all-target
  strict Clippy pass locally; this is not provider sandbox acceptance.
- New SaaS/`make:billing` routes now contain the unfinished live flow: real or
  mixed credentials return HTTP 503 before provider I/O, replay claims or SQL.
  Offline development fixtures remain available. Existing applications require
  a reviewed controller migration; new typed provider/inbox APIs do not replace
  their owner binding automatically. Full live integration remains required
  before enabling that generated operation, as permitted by the P0 allocation.

All eleven v12 adapters are in scope: Stripe, Lemon Squeezy, InfinitePay,
Polar, Paddle, Razorpay, Mercado Pago, Coinbase Commerce, PicPay, Alipay and Wise.
Wise is a payout adapter; the others implement the billing trait. Test each
operation independently and record explicit unsupported boundaries. A passing
mock or a failure-before-dispatch test is not sandbox acceptance. The generated
SaaS currently only selects Stripe or Lemon Squeezy; extending that selector
requires provider-specific configuration and lifecycle contracts.

## Release allocation

P0 below means necessary before enabling the affected live operation, not that
every provider must become live before a framework maintenance release. An
explicit unsupported operation is acceptable containment; a malformed request
or fabricated success is not. Important compatible fixes can ship separately
on the v12 maintenance line without waiting for all v13 capabilities.

| Finding | Evidence in reviewed baseline | v12.1 maintenance target | v13 target / acceptance |
| :--- | :--- | :--- | :--- |
| **SAAS-001 · P0** Lemon Squeezy store | `providers/lemonsqueezy.rs` still sends store `1`. | Add validated store configuration without removing the existing constructor; an unconfigured real request must fail explicitly. | Bind response to store/variant, protocol fixtures and official sandbox evidence. |
| **SAAS-002 · P0** Paddle checkout | `providers/paddle.rs` sends nested `customer.email` and top-level `return_url`. | Correct the supported flow if it fits the contract; otherwise disable that live operation with a typed error and accurate matrix. | Typed transaction/customer/checkout boundaries, approved payment-link prerequisites and usable sandbox checkout evidence. |
| **SAAS-003 · P0** Polar checkout | `providers/polar.rs` uses `product_price_id` and `/v1/checkouts/custom/`. | Contain the obsolete path; do not silently reinterpret an existing price ID as a product ID. | Current products-based API, local-subject binding and reviewed webhook lifecycle. Any client IP forwarding requires a trusted proxy boundary and a stated purpose. |
| **SAAS-004 · P0** Stripe ownership | `providers/stripe.rs` lacks a local subject in checkout; the generated handler requires webhook email. | Add an opt-in typed checkout/receipt path and safe generator flow, or keep the affected live scaffold disabled. Do not claim an email lookup solves ownership. | Persist attempt/session/customer/subscription bindings; accept the relevant signed lifecycle events; test missing email, changed email and cross-owner/tenant denial. |
| **SAAS-005 · P0** POST redirect | `billing_controller.rs.template` uses `Redirect::temporary`. | Emit HTTP 303 after POST; assert status and `Location` in a materialized route test. Review the portal handoff too. | Preserve browser method semantics in every checkout flow. |
| **SAAS-006 · P0** Portal capability | Scaffold mounts a portal backed by a live `UnsupportedOperation`. | Expose an honest unavailable state and capability-gate route generation; no invented customer portal. | Provider-specific sessions bound to persisted customer identity, with their own sandbox tests. |
| **SAAS-007 · P0** Wise transfer | `providers/wise.rs` constructs recipient/quote/idempotency values from email/profile/amount. | Reject the unsupported live flow before dispatch; preserve deterministic offline behavior. | Separate recipient, quote, transfer and funding operations; UUID idempotency, corridor rules and reconciliation. Transfer creation is not funding confirmation. |
| **SAAS-008 · P0** Checkout idempotency | Generic checkout has no durable attempt/idempotency input. | Add an optional typed API without adding required methods to downstream trait implementations; legacy ambiguity must be documented and contained. | Persist attempt before dispatch, bind key to request digest, forward it only under a reviewed provider contract and reconcile unknown outcomes. |
| **SAAS-009 · P0 for multiple gateways** Provider namespace | Generated customer/subscription schemas omit provider; uniqueness is email/subscription ID only. | Keep the supported scope explicit and provide a reviewed opt-in migration; never guess the provider for existing rows. | Namespace by provider account and test/live mode as well as provider, with authenticated tenant/subject ownership; cover ID collisions and provider migration. |
| **SAAS-010 · P0** Atomic billing state | Generated customer and subscription saves are separate. | Couple event processing and billing mutation in a transaction; add the event-envelope boundary needed to do so compatibly. | Durable inbox identity/digest/outcome, transactionally updated entitlements and outbox effects; crash, cancellation, duplicate, out-of-order and retry tests. |
| **SAAS-011 · P1** MSVC flags | `project/env_config.rs` still emits `/DEBUG:FASTLINK`. | Remove the unconditional unsupported flag and validate generated Windows builds. The reported LNK4315 was not reproduced here. | Capability-tested linker choices without unmeasured performance claims. |
| **SAAS-012 · P1** Reproducible applications | Generator ignores `/Cargo.lock`; Docker build omits `--locked`. | Retain the binary application's lockfile and require it in deployment builds, including dependency-cooking stages. Document initial lockfile generation. | Generate/build/package from one committed lockfile; test missing/stale lockfile failures. |
| **SAAS-013 · P1** Strict backend graph | ORM unconditionally enables SQLx SQLite/PostgreSQL/MySQL/Any drivers; its own `default` and `strict-*` features are empty. Studio also requires `queue-sqlite`. | Audit all consumers and offer a backend-exclusive profile with migration guidance. Preserve existing default convenience deliberately rather than breaking it accidentally. | Explicit Studio queue feature and standalone PostgreSQL consumer graph excluding unrelated SQLx drivers. Confirm with `cargo tree`; a workspace `--all-features` graph is not an isolation test. |
| **SAAS-014 · P1** One-time checkout | Stripe checkout always uses `mode=subscription`; `charge()` is a different off-session contract. | Describe the existing method as subscription-only; add an opt-in one-time API only with full evidence. Never use a recurring plan for an advertised one-time purchase. | Explicit payment/subscription modes, server-owned prices, success/cancel URLs, durable receipts and the correct paid/refunded/disputed event sets. |
| **SAAS-015 · P0** Stripe key rotation | Verifier overwrites each earlier `v1` signature. | Bound header/candidate sizes and accept any valid candidate with constant-time HMAC verification, one valid timestamp and freshness enforcement. | Test valid first/middle/last, malformed neighbors, duplicate timestamps, all-invalid, stale and oversized headers; preserve the fix. |
| **RULLST-001 · P1** Nexus AI configuration | Nexus detection and `AiClient::auto()` both omit Groq configuration. | Add an explicit application-supplied client path or one consistent resolver, with documented precedence. | Reuse configuration across authorized panels; distinguish offline, configured and unavailable. Test Groq-only configuration, failure handling, authorization and safe rendering. |
| **RULLST-003 · P1** Machine-client WAF denial | Default blocklist rejects curl, Wget, Python and Go regardless of request intent. | Remove those defaults and preserve request inspection and configurable crawler policy; test the production baseline. | Treat User-Agent as a forgeable traffic preference, never authentication or authorization. |

Provider files above are under `rullst-capital/src/`. The shared billing
template is under `cargo-rullst/src/generators/` and feeds both SaaS and
`make:billing`; fixes must cover both. The SaaS schema is in
`cargo-rullst/src/blueprints/saas/models.rs`, with separate SQLx and Turso
generator templates that also require review.

The Nexus resolver mismatch is broader than Groq: detection advertises
`OPENAI_BASE_URL` while `AiClient::auto()` does not consume it, and the latter
consumes `DEEPSEEK_API_KEY` while the panel detector does not. One source of
configuration should replace this drift. An environment variable is never
connectivity evidence.

## Additional framework review: Razorpay lifecycle

The legacy adapter maps `subscription.authenticated` and `payment.captured`
to an active subscription and can substitute an order ID for a subscription ID.
The maintenance correction requires the subscription entity's own bounded
identities and matching lifecycle state. Authentication, standalone payments,
missing/confused identities and mismatched states fail instead of granting
access. Supported activation/charged/resumed and pending/halted/paused/cancelled
fixtures are signed with real HMACs. No live account was used; event ordering,
durable processing and provider/customer/tenant binding remain host work.
Razorpay documents distinct [states](https://razorpay.com/docs/payments/subscriptions/states/)
and [subscription events](https://razorpay.com/docs/webhooks/subscriptions/).

## Compatible maintenance boundaries

Paddle review found the same last-signature-only pattern as Stripe, plus
duplicate-timestamp acceptance. Its verifier now accepts any matching bounded
`h1` candidate and requires exactly one timestamp. Two real-HMAC regressions
failed before the correction and pass afterward, together with all 159 Capital
tests under Actix and strict all-target Clippy. The existing configured
freshness window remains in force. Paddle's
[signature contract](https://developer.paddle.com/webhooks/about/signature-verification/)
allows multiple `h1` values; this correction does not imply live checkout or
subscription-lifecycle acceptance.

Polar review reproduced discarded RFC3339 billing periods/current customer
contacts and acceptance of unrelated event kinds or conflicting identities.
A bounded subscription parser now separates lifecycle events from orders,
preserves current and unambiguous legacy fields, and distinguishes scheduled
cancellation from final revocation. Three signed regressions failed before
the fix; all 162 Capital tests with Actix and strict all-target Clippy pass
afterward. The reviewed [subscription schema](https://polar.sh/docs/api-reference/2026-04/subscription_updated)
and [event sequences](https://polar.sh/docs/integrate/webhooks/events) define
that boundary. The legacy event still omits account/mode, scheduling and
ordering metadata; hosts must retain verified raw evidence and reconcile
ownership/settlement. No sandbox or generated checkout is enabled by this fix.

InfinitePay's implemented HMAC/subscription payload does not match the reviewed
[checkout callback contract](https://www.infinitepay.io/checkout-documentacao).
The live verifier and handler are now explicitly unsupported pending reviewed
authentication and payment lookup bound to merchant, order and amount. Offline
fixtures still require an explicit mock secret. This contains the unsupported
framework path; it does not assert that the provider lacks other API products.

Additional Lemon Squeezy review found that the legacy parser accepted missing
event/object kinds, serialized absent customer/variant IDs as `null`, and mapped
`on_trial` to `Unpaid`. A dedicated subscription parser now validates lifecycle,
store/mode/identity and expiry, separately from invoice events. Three real-HMAC
regressions failed against the old code and pass after the correction. Provider
[object](https://docs.lemonsqueezy.com/api/subscriptions/the-subscription-object)
and [event](https://docs.lemonsqueezy.com/help/webhooks/event-types) contracts
define the boundary; this is not a live-account test or durable reconciliation.

- Preserve source compatibility in v12.1: additive constructors, builders,
  extension contracts and explicit deprecations. A mandatory trait method,
  changed public struct literal or silently changed identifier meaning can
  break applications even when the function name stays the same.
- Containing an invalid live operation may change runtime behavior; document
  the failure and upgrade path. Do not retain fabricated values for the sake
  of apparent compatibility.
- Generator fixes affect newly generated code. Existing applications need
  reviewable source/database migrations, collision reports and rollback or
  forward-recovery guidance. Do not regenerate over customized code.
- A webhook replay claim alone is not atomic domain processing. The database
  commit must determine whether processing completed; failed work must remain
  recoverable. A timeout after provider mutation remains an unknown outcome.
- Test-mode and offline mocks remain useful. Production configuration must
  reject a mock as payment evidence; a success redirect never grants access.
- Allocation to v12.1 is a target, not evidence of merge or publication. Carry
  reviewed fixes into v13 and verify their presence by commit, not branch name.

## Boundaries that are not framework defects

`RULLST-002` is the umbrella reference to SAAS-001–015, not a sixteenth
independent payment defect. Studio's disconnected AI playground is a documented
limitation. Adapters that explicitly reject unsupported live methods are also
documented capability boundaries, not automatically broken integrations.

The reports attribute Docker dependency-copy mistakes, assistant formatting,
proxy/origin behavior, unsafe example HTML and application error disclosure to
the examples repository. Keep their regression work there and contribute only
reusable fixes upstream. A paid file already committed publicly cannot become
exclusive through route authorization; private artifact storage is application
work. No private Academy implementation or learner data is copied into this
public planning document.

## Delivery slices and required evidence

1. **Compatible containment and small regressions:** SAAS-005/006/011/012/015;
   contain SAAS-001/002/007 until their request contracts are complete; SAAS-003
   now has its typed product contract.
2. **One complete Stripe journey:** SAAS-004/008/010 plus provider-scoped
   persistence from SAAS-009; opt-in SAAS-014 only when payment and subscription
   evidence are distinct. Test retries, refunds and disputes as well as success.
3. **Configuration consistency:** RULLST-001 and SAAS-013, with explicit migration
   and feature-consumer coverage. These can proceed independently of gateways.
4. **v13 capabilities:** finish additional providers one operation at a time,
   then multi-provider reconciliation, portals and Wise payouts. Retain the
   operation-level capability matrix and sanitized provider evidence.

Every implementation slice needs targeted negatives, materialized generator
coverage, compatible consumer checks and the repository release gates. Hosted
checks should handle cold native/all-feature matrices whose disk peak cannot be
bounded locally. Local tests must respect the disk reserve in `AGENTS.md`.
Provider sandbox evidence is separate from Rust tests; no live payment, payout,
deployment or credential change is part of this planning review.

## Provider contracts consulted on 17 September 2026

The request discrepancies above are consistent with the provider-owned
[Lemon Squeezy checkout](https://docs.lemonsqueezy.com/api/checkouts/create-checkout),
[Paddle transaction](https://developer.paddle.com/api-reference/transactions/create-transaction/),
[Polar checkout](https://polar.sh/docs/features/checkout/session) and
[Wise transfer](https://docs.wise.com/api-reference/transfer) contracts.
Stripe documents overlapping endpoint secrets and multiple signatures in its
[webhook guide](https://docs.stripe.com/webhooks).
These references support contract review; they do not prove account eligibility
or acceptance of a Rullst request.
