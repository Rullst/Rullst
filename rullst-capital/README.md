# Rullst Capital 💳

`rullst-capital` provides provider-neutral billing contracts, normalized
billing types, bounded webhook verification helpers and application-supplied
revenue snapshots, with a supported **Stripe** adapter and an **experimental**
InfinitePay adapter. Provider method coverage is not uniform; inspect the
selected adapter and test it in the provider sandbox. Other gateways are
integrated by implementing the provider traits in application code.

## 🚀 Core Features

- **Provider-neutral contracts:** `BillingProvider` and `MeteredBillingProvider`
  define checkout, subscription, webhook and usage operations. Capabilities vary
  by adapter, and unsupported live operations fail closed.
- **Revenue snapshot (`/studio/capital`):** Displays metrics supplied explicitly
  by the application to a process-local `RevenueDashboardManager`; it is not an
  accounting ledger and does not infer money from event names.
- **Webhook event inspector:** Holds records explicitly passed to the local
  manager. Capital does not connect every webhook route to Studio automatically.
- **Webhook verification:** One canonical signature/freshness/replay verifier
  behind the Axum and Actix adapters. The opt-in SQL replay ledger shares
  bounded payload digests or semantic provider event keys across processes on
  SQLite, PostgreSQL, MySQL, and MariaDB. Reconciliation and authorization
  remain application responsibilities.
- **Payment-bound invoices:** The opt-in `invoice-pdf` feature validates money
  into exact minor units, renders bounded paginated PDF and binds delivery to a
  final receipt matching recipient, amount and currency.
- **Provider-specific metered billing:** Current Stripe Meter Events
  request/response contract, bounded protocol parsing, deterministic non-live
  mocks and explicit retry evidence; custom adapters reuse the same receipt type.
- **Shared team/workspace quotas:** Bounded subject identities, idempotent
  reservations, replay-safe execution and an opt-in transactional SQL store for
  SQLite, PostgreSQL, MySQL and MariaDB.
- **v13 plan-gate candidate:** `entitlements` checks current tenant/subject-bound
  subscription state against an explicit feature/plan policy on every action.
  It rejects mocks, mismatched modes, non-active status, expiry and stale reads.
  Trusted storage/reconciliation adapters remain application-owned; success is
  a read-time decision, not a reusable payment or authorization token.
- **Coupons and relative trials:** A bounded/redacted coupon value, current
  Stripe discount binding, and 1–730-day Stripe trial updates with
  explicit-clock retries and fail-closed provider capability boundaries.
- **Offline fixtures:** Empty or `mock_*` credentials select deterministic,
  visibly non-live behavior for tests and local development.

---

## Removed in v13

v13 reduces Capital to a maintainable base plus two payment adapters. The
following were removed; their code remains in the repository history:

- the Paddle, Lemon Squeezy, Polar, Razorpay, Mercado Pago, Alipay, Coinbase
  Commerce and PicPay adapters, with their typed checkout, portal and usage
  types (including `LemonSqueezyUsageRecord`/`LemonSqueezyUsageAction`);
- the Wise payout adapter and the payout contracts it alone used
  (`PayoutProvider`, `PayoutStatus`, `PayoutEvent`, `init_payout_provider`,
  `try_init_payout_provider` and `payout_provider`);
- the NFS-e/fiscal preparation module (`fiscal`, the `nfse` feature and the
  umbrella `capital-nfse` feature, `Invoice::to_dps` and
  `CapitalError::FiscalError`). It was never validated with a real
  municipality; NFS-e support may return later as a separate product outside
  the framework.

See the "Capital providers and NFS-e removed" row of the
[v13 migration guide](https://github.com/Rullst/Rullst/blob/main/docs/src/migration-v13.md).
To keep using another gateway, implement `BillingProvider` for it in your
application, as shown in
[Writing your own payment provider](https://github.com/Rullst/Rullst/blob/main/docs/src/capital-custom-provider.md).

---

## ✨ Supported Providers

The 12.1 SaaS/`make:billing` Stripe integration persists authorized
customer bindings, immutable attempts, Checkout Session IDs and atomic event
receipts. It reconciles current provider state under database revision fencing
and resumes existing open sessions. Configure the account, credentials, recurring
price allowlist and HTTPS return URL as described in generated `BILLING.md`.
Mixed credentials remain unavailable. Stripe is the only generated provider.
Updating Capital does not rewrite existing controllers or apply new migrations.

| Provider | Status | Current boundary |
| :--- | :--- | :--- |
| **Stripe** | Supported | Typed customer/subscription checkout, customer-ID portal, current-state reads and signed events; generated durable SQLx/Turso integration. Immediate Payment Intent charge is separate. |
| **InfinitePay** | Experimental | Offline fixtures only until validated against a live account; live plan-only checkout and body-only callback verification fail closed. |

The shared `create_customer_portal(email, return_url)` methods do not have a
reviewed live provider-session contract and return `UnsupportedOperation` for
live credentials. Their deterministic empty/`mock_*` examples are offline
fixtures, not authenticated portal sessions.
Legacy checkout and portal fixtures use reserved `https://mock.<provider>.invalid/`
hosts and carry only the plan ID, never the customer email or return URL, so a
deployment started without credentials cannot send a browser or personal data
to a real provider domain. InfinitePay cancellation likewise rejects live calls,
and its pause, usage, coupon and trial operations are unsupported, until an
actual provider operation is implemented.

The legacy `create_checkout_session(email, plan_id, return_url)` accepts a
provider-managed plan/price identity, not an amount or currency. InfinitePay
does not have an implemented reviewed mapping for that contract; its live method
returns `UnsupportedOperation` before HTTP dispatch. It never invents a price,
currency, buyer identity or subscription from a plan label. Empty/`mock_*` API
credentials preserve its deterministic offline fixture, while `handle_*` values
are not mock credentials. An authoritative typed pricing/provider contract is
required before enabling this live checkout path.

| Reviewed legacy method boundary | Stripe | InfinitePay (experimental) |
|---|---|---|
| Plan/price-based checkout request | adapter | unsupported |
| Customer portal by email | unsupported | unsupported |
| Immediate evidence-bound charge | adapter | unsupported |

`adapter` means a bounded request implementation exists, not that this audit
validated acceptance or every response schema against a live provider account.
Offline fixtures are deliberately excluded from the live-method matrix.

InfinitePay's [checkout callback and payment lookup](https://www.infinitepay.io/checkout-documentacao)
do not establish the HMAC/subscription contract assumed by the old adapter.
The live body-only verifier and handler return `UnsupportedOperation`;
explicit mock-secret verification remains available for offline fixtures.
Enabling a real callback needs reviewed authentication, merchant/order/amount
binding and authoritative reconciliation. A locally signed fixture does not
prove that the provider emits that protocol. InfinitePay stays experimental
until those live paths are implemented and validated against a real account.

Applications that need another gateway implement `BillingProvider` (and, for
metered billing, `MeteredBillingProvider`) themselves. Such an adapter must
declare its webhook verification mode explicitly before the canonical
middleware accepts it; see
[Writing your own payment provider](https://github.com/Rullst/Rullst/blob/main/docs/src/capital-custom-provider.md).

### Customer-bound Stripe subscription checkout (12.1)

`StripeProvider::create_customer` accepts a `StripeCustomerRequest` containing
an opaque local owner reference, a persisted retry key and optional contact
email. It requires the returned customer metadata, object identity, creation
time and test/live mode to match the request contract. Email is never used to
discover or claim an existing customer. Persist the intent before calling and
the resulting customer/owner/account binding before checkout:

```rust,no_run
use rullst_capital::{StripeCustomerRequest, StripeCustomerStatus, StripeProvider};

async fn provision() -> Result<(), rullst_capital::CapitalError> {
    let provider = StripeProvider::new("mock_key", "mock_webhook");
    let intent = StripeCustomerRequest::new("owner_opaque", "provision_unique")?;
    // Persist the authorized intent and digest before dispatch.
    let customer = provider.create_customer(&intent).await?;
    assert_eq!(customer.status(), StripeCustomerStatus::Mock);
    // Persist the returned ID and matching digest before creating a checkout.
    Ok(())
}
```

Changing optional email changes the provisioning digest. Retrying an uncertain
outcome must preserve the original input; it must not silently create a second
customer after provider idempotency retention expires. See Stripe's
[customer creation contract](https://docs.stripe.com/api/customers/create?api-version=2025-03-31.basil).

`StripeProvider::create_subscription_checkout` accepts an existing Stripe
customer ID and an immutable `StripeCheckoutRequest`. Persist the customer's
authenticated owner/tenant binding and the attempt key/digest before dispatch:

```rust,no_run
use rullst_capital::{StripeCheckoutRequest, StripeProvider, StripeCheckoutStatus};

async fn checkout() -> Result<(), rullst_capital::CapitalError> {
    let provider = StripeProvider::new("mock_key", "mock_webhook");
    let attempt = StripeCheckoutRequest::new(
        "cus_existing", "price_monthly", "owner_opaque", "attempt_unique",
        "https://app.example/billing/success", "https://app.example/billing/cancel",
    )?;
    let session = provider.create_subscription_checkout(&attempt).await?;
    assert_eq!(session.status(), StripeCheckoutStatus::Mock);
    // Store the session ID and compare its input digest with the persisted attempt.
    // A Created session or a browser redirect never proves payment.
    Ok(())
}
```

The operation pins Stripe API `2025-03-31.basil`, sends the customer and opaque
reference, copies owner/attempt references into session and subscription metadata and forwards
`Idempotency-Key`. Expanded line items must match the requested recurring price
and quantity. Response customer/reference/redirects/mode must also match before
returning an open session. Stripe's hosted URL is preserved, including its
documented opaque fragment. Request/receipt debug output omits identifiers,
URLs and keys; mocks have a distinct status and no provider test/live mode.

The generated SaaS/`make:billing` modules provide account/test-live namespaces,
durable provisioning and attempts, signed Checkout/subscription event handling,
atomic completion and revision-fenced reconciliation. Hosts calling the low-level
adapter directly must supply those same application boundaries. Do not retry an old key indefinitely: Stripe may discard idempotency
records after its retention period. An unknown outcome requires reconciliation,
not a newly generated attempt key. The recovery reads (`verify_account`,
`retrieve_bound_customer`, `find_bound_customer`, `retrieve_checkout` and
`find_checkout`) return `ConfigurationError` for malformed local IDs or a
live/test mode that differs from the key, and `UnsupportedOperation` for empty
or `mock_*` keys, which have no offline recovery fixture. Only a provider
response that fails its bindings is a contract mismatch. See Stripe's
[checkout contract](https://docs.stripe.com/api/checkout/sessions/create?api-version=2025-03-31.basil)
and [idempotency semantics](https://docs.stripe.com/api/idempotent_requests).
The legacy email-based trait method remains available for source compatibility;
existing application-owned controllers require an explicit code/database migration.

### Provider verification levels

Treat every provider and operation as a separate conformance target. Evidence
for one Stripe checkout, for example, does not validate its portal, refund,
metering or webhook paths and says nothing about another provider.

1. **Deterministic offline:** Validate input bounds, redaction, failure classes,
   idempotency material and mock behavior without network access.
2. **Protocol fixtures:** Exercise exact signed payloads, replay/freshness
   rejection and bounded response parsing against retained provider examples.
3. **Provider test environment:** Run checkout, webhook, cancellation and
   reconciliation cases in the provider's official sandbox or test mode.
4. **Controlled live acceptance:** Only after account, legal, secret, refund,
   observability and reconciliation controls are ready, perform the smallest
   provider-permitted real transaction and retain redacted evidence.

The generated SaaS blueprint currently provides a durable Stripe application
boundary only. It is not a conformance application for every Capital adapter.
A release claim should name the exact provider, operation, environment and
observed result rather than saying that “payments work.” See the official
[Stripe testing](https://docs.stripe.com/testing) and
[sandbox](https://docs.stripe.com/sandboxes) guidance.

Stripe's v12 normalized path accepts subscription `created`, `updated`,
`deleted`, `paused` and `resumed` events. It requires a subscription object,
bounded subscription/customer IDs and exactly one non-truncated price item;
multi-item subscriptions need an application-specific integration. Missing
event type, payment-status aliases, confused IDs and contradictory lifecycle
states are rejected. `incomplete` and `incomplete_expired` map to the legacy
non-entitled `Unpaid` value, not proof of a failed invoice payment.
The billing-period end comes from the single item on Basil payloads, with a
legacy subscription-level fallback; conflicting values fail. This follows
Stripe's [billing-period API change](https://docs.stripe.com/changelog/basil/2025-03-31/deprecate-subscription-current-period-start-and-end).
Email is optional contact data. Signed subscription state alone still does not
bind a local owner, order events, commit an inbox or prove invoice settlement.

`StripeProvider::verify_subscription_event` supplies an additive immutable
envelope for a caller-owned inbox transaction. It retains the signed event
ID/type/API version, creation time, matching event/subscription test-live mode,
optional connected account and `rullst_owner_reference`, and exact Stripe
status alongside the legacy normalization. `require_real()` rejects explicit
mock verification before production processing. The verifier does not claim
the event, so a failed application transaction can retry verification.

`mutation_digest()` binds the fields exposed for subscription processing and
ignores contact email, signature timestamp and unrelated delivery/JSON fields.
`payload_digest()` separately hashes the exact bytes. Persist the event ID and
mutation digest under a namespace containing the configured account and mode,
and commit with domain changes. Compare customer/owner references with saved
application bindings. A changed mutation under the same event ID requires
reconciliation; creation timestamps alone cannot order all updates. Neither
digest encrypts data or supplies storage, authorization or settlement evidence.
An absent connected-account field does not identify the platform account.
See Stripe's [event envelope](https://docs.stripe.com/api/events/object).

`StripeProvider::retrieve_subscription` reads current single-price state using
a `StripeSubscriptionLookup` bound to the saved subscription/customer/reference,
expected price and test/live mode. The response must match those bindings and
the same bounded item/state parser as signed events. `StripeSubscriptionSnapshot`
retains the exact provider status; `require_real()` rejects its deterministic
non-entitled mock. This follows Stripe's pinned
[subscription read contract](https://docs.stripe.com/api/subscriptions/retrieve?api-version=2025-03-31.basil).

Stripe [does not guarantee event delivery order](https://docs.stripe.com/webhooks#event-ordering).
Serialize reconciliation reads with their database updates; fetching two
snapshots before unrelated transactions still permits the older read to commit
last. The retrieval API does not implement that serialization, retry a request,
grant entitlements or prove invoice settlement. Unknown outcomes and changed
customer/price/reference bindings require explicit reconciliation.

With `webhook-sql`, `SqlStripeEventInbox` owns the transaction that commits an
event and a caller-supplied SQL mutation. `StripeInboxScope::platform` or
`::connected` fixes the application namespace, configured account, endpoint
kind and test/live mode; mock and mismatched events are rejected before SQL.
The host must establish which account owns its credentials and endpoint secret.

Construct the inbox with the ORM's selected relational pool and matching SQL
dialect. Run `prepare_schema()` only during explicit setup/migration, then call
`process(&verified_event, |transaction, event| Box::pin(async move { ... }))`.
Inside that callback, validate the saved customer/owner binding and event order,
write through the supplied transaction, and return `StripeInboxOutcome::Applied`
or `Ignored`. External effects belong in an outbox in that same transaction.

An exact committed retry returns its saved outcome without invoking the callback.
Reusing an event ID with a changed mutation digest fails with `EventConflict`.
Domain errors and cancellation before commit roll back uncommitted SQL; an
uncertain commit returns `CommitUncertain`, which requires retrying the same event
to discover its recorded outcome. Do not combine this path with middleware that
claims replay admission before the handler.

Capacity is bounded, persisted and immutable per scope. Entries never expire
automatically; a full inbox rejects new events while retaining exact retries.
The application owns retention, reconciliation, transactional domain tables,
customer provisioning and authorization. This API does not migrate existing or
generated handlers, support Turso's remote batch transport, order snapshots or
make HTTP effects atomic. Stored event/scope/mutation hashes minimize identifiers;
they are not encryption or a complete billing audit history.

---

## Outbound Provider Safety and Retry Evidence

Reviewed live adapters share one pooled client with finite connect and
whole-request timeouts. It does not follow redirects or inherit ambient proxy
environment variables. JSON responses are capped at one MiB, checkout
locations must be bounded credential-free HTTPS URLs without fragments, and
public errors never include a request URL, credential, provider body, or raw
transport diagnostic.

Transport, HTTP, size, JSON, and response-binding failures use
`CapitalError::Provider(ProviderFailure)`. The stable
`ProviderFailureClass::{Permanent, Transient, RateLimited}` disposition and
optional bounded numeric `Retry-After` value support alerting and
application-owned scheduling:

```rust
use rullst_capital::{CapitalError, ProviderFailureClass};

fn may_consider_retry(error: &CapitalError) -> bool {
    matches!(
        error,
        CapitalError::Provider(failure)
            if matches!(
                failure.class(),
                ProviderFailureClass::Transient | ProviderFailureClass::RateLimited
            )
    )
}
```

This classification never authorizes an automatic retry. Repeat a billing
mutation only after proving that the exact adapter operation forwards a stable
persisted idempotency key, and retain webhook reconciliation. The built-in
client intentionally offers no ambient corporate-proxy escape hatch; an
explicit reviewed configuration boundary is future work.

---

## 🚀 Quickstart

Add `rullst-capital` to your `Cargo.toml`:

```toml
[dependencies]
rullst-capital = "12.1.0"
```

Native invoice PDF is independently opt-in. One-call Mail delivery uses the
downstream `rullst-mail/capital-invoice` feature, or `rullst/capital-mail` when
using the umbrella crate:

```toml
rullst = { version = "12.1.0", features = ["capital-mail"] }
```

Durable relational quota accounting is separately opt-in:

```toml
rullst = { version = "12.1.0", features = ["capital-quota-sql"] }
# Or directly: rullst-capital = { version = "12.1.0", features = ["quota-sql"] }
```

Durable cross-process webhook replay claims are independently opt-in:

```toml
rullst = { version = "12.1.0", features = ["capital-webhook-sql"] }
# Or directly: rullst-capital = { version = "12.1.0", features = ["webhook-sql"] }
```

Applications using the umbrella crate can derive the bounded billing facade on
any named struct with an `email: String` field. Optional
`subscription_id: Option<String>` and `tier: Option<String>` fields enable the
corresponding helpers; provider initialization remains explicit:

```rust
use rullst::capital::Billable as _;

#[derive(rullst::Billable)]
struct Workspace {
    email: String,
    subscription_id: Option<String>,
    tier: Option<String>,
    grace_period_starts_at: Option<i64>,
    grace_period_ends_at: Option<i64>,
}

fn has_pro_access(workspace: &Workspace) -> bool {
    workspace.can_access("pro")
}
```

`Billable::subscription_with(&provider)` returns a statically dispatched
`SubscriptionHandle` with `cancel()` and `pause()`. When both grace-period
fields are present, the derive exposes a validated half-open window of at most
366 days; an incomplete pair is a compile error. `Billable` does not persist or
authorize provider state, schedule provider changes, infer membership, or
choose currency/payment methods. Its explicit `quota_request` helper can derive
a limit from the model's `tier_limit`; a separately configured quota store
performs the accounting. Applications must establish identities and policies
before invoking either boundary.

### Shared Team and Workspace Quotas

Use one `BillingSubject` for the authenticated tenant/workspace so every member
consumes the same limit. `Billable::quota_request` derives the limit from the
subscription owner's tier rather than a client payload; a tier limit of zero
returns `QuotaError::LimitExceeded`, like a used-up limit. `QuotaGate` atomically
reserves before calling the application operation, skips exact idempotent
replays and releases a fresh reservation when the callback returns an error.
`QuotaExecution::Replay` means only that the key is already claimed: the first
call may still be running and later fail and release it, or may have been
dropped without releasing it. Do not report a replay as completed work without
checking the application's own record.

The always-available `InMemoryQuotaStore` is deterministic and process-local.
With `quota-sql`, `SqlQuotaStore` persists a unique event claim and conditionally
increments the shared counter on SQLite, PostgreSQL, MySQL or MariaDB. For a
relational create that must be atomic with accounting, open a transaction from
`store.pool()`, call `reserve_with_transaction`, execute the domain insert on
that same transaction and commit once. See the
[SaaS billing tutorial](https://rullst.github.io/Rullst/book/tutorials/19-saas-billing-capital.html#8-enforce-one-shared-workspace-quota-before-creation)
for the complete flow.

Subject kinds and IDs, features and event keys are case-sensitive on every
backend, so tenants such as `aB3x` and `Ab3X` keep separate counters. New
MySQL/MariaDB tables declare those columns `CHARACTER SET ascii COLLATE
ascii_bin`. `prepare_schema` never alters an existing table: while a key column
of either quota table still folds case, `prepare_schema` and every store
operation return `QuotaError::StorageUnavailable`.

#### Upgrading MySQL/MariaDB quota tables

Tables created by an earlier release use the server's case-insensitive default
collation, so keys that differ only by letter case shared one counter or claim.
The migration cannot split rows merged that way; review subjects and event keys
that differ only by case first. Stop quota writers, back up both tables, then
convert the key columns:

```sql
ALTER TABLE rullst_capital_quota_counters
  MODIFY subject_kind VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  MODIFY subject_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  MODIFY feature VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL;
ALTER TABLE rullst_capital_quota_claims
  MODIFY subject_kind VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  MODIFY subject_id VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  MODIFY feature VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  MODIFY event_key VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL;
```

Any binary or case-sensitive (`_bin`/`_cs`) collation also passes the check.
The MySQL 8.0 and MariaDB contract tests run this migration on legacy tables.

Membership/authentication, tier persistence and webhook reconciliation,
migrations, cleanup policy for abandoned standalone reservations, and
Turso/NoSQL adapters remain application responsibilities. Writes outside the
gate are not intercepted automatically.

An immediate charge is available without exposing a raw-card field. It requires
minor units, currency, authoritative provider customer/payment-method IDs and a
unique application retry key:

```rust
use rullst::capital::{Billable as _, CapitalError, StripeProvider};

async fn collect(
    workspace: &impl rullst::capital::Billable,
    stripe: &StripeProvider,
) -> Result<(), CapitalError> {
    let receipt = workspace
        .charge_with(
            stripe,
            4_990,
            "BRL",
            "cus_provider_owned",
            "pm_provider_tokenized",
            "order_42-attempt_1",
        )
        .await?;
    assert_eq!(receipt.amount_minor(), 4_990);
    Ok(())
}
```

Stripe is the only reviewed live direct-charge adapter. Exact `mock_*` retries
are deterministic but carry the distinct non-success `ChargeStatus::Mock`;
other adapters return `UnsupportedOperation`. Mandate/SCA,
durable idempotency, webhook reconciliation and entitlement changes remain
application responsibilities.

### Coupons and Relative Trials

The provider-bound handle validates coupon IDs before dispatch. Stripe uses the
current expanded subscription-discount update and checks that the response
contains both the requested subscription and coupon. Unreviewed live adapters,
including InfinitePay, return `UnsupportedOperation`.

```rust,no_run
use rullst_capital::{Billable as _, CapitalError, StripeProvider};

async fn retention_offer(
    workspace: &impl rullst_capital::Billable,
    stripe: &StripeProvider,
    command_created_at: i64,
) -> Result<(), CapitalError> {
    let subscription = workspace.subscription_with(stripe)?;
    subscription.apply_coupon("RETENTION_25").await?;
    subscription
        .extend_trial_days_at(15, command_created_at)
        .await
}
```

`extend_trial(15)` uses current UTC for convenience; persist a trusted command
time and use `extend_trial_days_at` for retry stability. `set_trial_end` remains
the explicit absolute operation. Stripe binds trial-update responses, but
authorization, concurrent-command serialization, billing-cycle policy,
signed-webhook reconciliation and real-account acceptance remain host or
release work.

### Provider-Specific Metered Usage

Use `MeteredBillingProvider` instead of the legacy uniform `report_usage`
method. Stripe requires a customer, configured event name, timestamp and
provider-forwarded identifier:

```rust,no_run
use rullst_capital::{
    CapitalError, MeteredBillingProvider as _, StripeMeterEvent, StripeProvider,
};

async fn report_lesson_minutes(stripe: &StripeProvider) -> Result<(), CapitalError> {
    let event = StripeMeterEvent::new(
        "cus_from_authoritative_state",
        "lesson_minutes",
        15,
        "usage:school-7:attempt-99",
    )?;
    let receipt = stripe.report_metered_usage(&event).await?;
    if receipt.is_live_accepted() {
        // Reconcile the provider meter; do not infer an entitlement from this alone.
    }
    Ok(())
}
```

Stripe's identifier is provider-forwarded but only has a rolling deduplication
guarantee. A custom adapter whose provider has no equivalent key returns a
receipt marked `UsageDeduplication::ApplicationOutboxRequired`: atomically claim
its event key in a durable outbox before submission. Empty or `mock_*` keys
return a stable `UsageStatus::Mock`, never a live acceptance.

### Payment-Bound Invoice Delivery

`Invoice::bind_succeeded_charge` accepts only final `Succeeded` evidence with
an exact recipient, minor-unit amount and currency match. Invoice amounts are
scaled by the currency's ISO 4217 exponent, so `total: 2500.0` in JPY binds a
2,500-yen receipt and `12.34` KWD binds a 12,340 minor-unit receipt. The resulting
`PaidInvoice` can be rendered as escaped HTML or a bounded A4 PDF. Mail's opt-in
`PaidInvoiceDelivery` bridge attaches both formats, runs mandatory pre-flight
and sends through the configured facade, a tenant route or an explicit static
driver.

Persist and atomically claim `PaidInvoice::delivery_key()` in an application
outbox before retryable delivery. The bridge does not infer webhook state or
promise provider acceptance/exactly-once behavior. See the
[SaaS billing tutorial](https://rullst.github.io/Rullst/book/tutorials/19-saas-billing-capital.html#4-render-and-deliver-the-invoice-only-after-final-success).

### Initializing a Provider

```rust
use rullst_capital::{init_provider, StripeProvider};

fn configure_billing() -> Result<(), std::env::VarError> {
    let api_key = std::env::var("STRIPE_SECRET_KEY")?;
    let webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")?;
    init_provider(Box::new(StripeProvider::new(api_key, webhook_secret)));
    Ok(())
}
```

The global billing provider can be set once per process: a later
`init_provider` call is ignored and the first provider stays active. The v13
`try_init_provider` returns `ConfigurationError` in that case, so a live
configuration cannot be silently shadowed by an earlier mock. Middleware can
also take an explicit provider through
`WebhookMiddlewareState::production_with_provider`.

### Creating Checkout Sessions

```rust
use rullst_capital::provider;

async fn checkout_handler() -> Result<String, String> {
    if let Some(p) = provider() {
        let checkout_url = p.create_checkout_session(
            "customer@example.com",
            "plan_pro_monthly",
            "https://mysaas.com/billing/success",
        ).await?;
        
        Ok(checkout_url)
    } else {
        Err("No billing provider configured".to_string())
    }
}
```

### Intercepting and Verifying Webhooks

`rullst-capital` includes Axum and opt-in Actix Web middleware adapters over one canonical [`webhook` verifier](https://github.com/Rullst/Rullst/blob/main/rullst-capital/src/webhook.rs). Both bound the body, require the provider to declare its verification mode, verify the provider signature (Stripe also enforces timestamp freshness), reject duplicate Standard Webhooks envelope headers, restore the exact body, insert a normalized event, and reject replayed payloads through a bounded TTL store. A custom provider whose protocol signs only the body without a checked timestamp lets an exact captured body verify again once its replay entry expires (24 hours by default) or, with the in-memory store, after a restart; persist and order its state changes in the application. Live InfinitePay verification is unavailable. Empty webhook secrets are configuration errors. `mock_*` secrets are explicit local fixtures and are rejected by the production-safe entry points. The in-memory store now fails closed when full instead of discarding an unexpired replay proof. The default store behind `verify_webhook`, `verify_webhook_mock_local` and their Actix equivalents holds at most 10,000 proofs for 24 hours each, so one process admits about 10,000 verified deliveries per rolling day before it answers 503. For busier endpoints, mount `verify_webhook_with_state` with `InMemoryWebhookReplayStore::new(capacity, ttl)` (up to 1,000,000 proofs and 30 days) or a shared `SqlWebhookReplayStore`.

The webhook route must receive a narrowly scoped CSRF exemption in the application router; never disable CSRF for browser routes. The exemption is safe only when this signature/freshness/replay middleware remains mandatory on that exact route. An outer blanket CSRF layer will reject legitimate provider callbacks before Capital can verify them.

```rust
use axum::{Router, routing::post, Extension};
use rullst_capital::{verify_webhook, WebhookEvent, SubscriptionStatus};

async fn handle_webhook_event(Extension(event): Extension<WebhookEvent>) {
    match event.status {
        SubscriptionStatus::Active => {
            println!("✅ Subscription active for customer: {}", event.customer_email);
        }
        SubscriptionStatus::Canceled => {
            println!("⚠️ Subscription canceled: {}", event.subscription_id);
        }
        SubscriptionStatus::PastDue => {
            println!("🚨 Payment past due for customer: {}", event.customer_email);
        }
        _ => {}
    }
}

pub fn router() -> Router {
    Router::new()
        .route("/webhooks/billing", post(handle_webhook_event))
        .layer(axum::middleware::from_fn(verify_webhook))
}
```

For Actix Web, enable the crate's `actix` feature (or umbrella
`rullst/capital-actix`) and mount
`actix_web::middleware::from_fn(verify_webhook_actix_with_state)` with a
`web::Data<WebhookMiddlewareState>`. The state can bind an explicit provider
through `WebhookMiddlewareState::production_with_provider`, avoiding global
configuration. See the
[payment guide](https://rullst.github.io/Rullst/book/payment-gateways-guide.html#actix-web-adapter)
for a complete example.

The default store is process-local. With `webhook-sql`,
`SqlWebhookReplayStore` persists an immutable capacity/TTL profile and
provider-scoped SHA-256 claims in SQLite, PostgreSQL, MySQL, or MariaDB. Schema
setup is explicit, active claims are never evicted to make room, configuration
drift/corruption/storage failure fail closed, and the same backend can be
passed to `WebhookMiddlewareState` through `Arc`. TTL decisions use the
database clock inside the claim transaction so process clock skew cannot expire
another node's proof early. An in-memory SQLite URL (`sqlite::memory:` or
`mode=memory`) keeps its single pooled connection for the pool's lifetime, as
`SqlQuotaStore` does, because a replacement connection would open an empty
database; its claims are still lost on restart.

That middleware path records the payload before calling the handler, so it is
a replay firewall rather than an exactly-once delivery protocol. A crash after
admission can still occur before a business mutation.

When a verified provider protocol supplies a stable event ID, prefer
`check_and_record_event_key` over payload-only replay detection. A relational
handler that uses the provider's low-level signature contract can call
`check_and_record_event_key_with_transaction` and write its domain mutation
through the same transaction before one commit. The claim may follow earlier
reads in that transaction: on MySQL/MariaDB a duplicate committed after the
transaction's REPEATABLE READ snapshot is still rejected by the claim insert's
duplicate key. Do not pre-claim the same event through SQL middleware on this
atomic path. This is atomic only inside that
database: provider API calls, e-mail, queues, and other systems still require
an outbox, idempotent consumers, and reconciliation.

---

## 🔐 Security Invariants

- **Constant-Time Verification**: Supported HMAC/token signatures use cryptographic verification or `subtle::ConstantTimeEq`.
- **Fail-Closed Configuration**: Empty webhook secrets never authenticate a request; mock credentials require a deliberate `mock_*` value.
- **Explicit Verification Mode**: A provider without an explicitly declared webhook verification mode cannot be mounted behind the canonical middleware.
- **Freshness and Replay Protection**: Timestamped protocols have a configurable five-minute window, and middleware records provider-scoped payload hashes in a bounded 24-hour TTL store. `webhook-sql` adds bounded durable claims; stable semantic event IDs can share a transaction with the domain mutation.
- **InfinitePay Containment**: The experimental adapter's live checkout and callback verification return `UnsupportedOperation`; only empty or `mock_*` API keys and `mock_*` webhook secrets operate offline.
