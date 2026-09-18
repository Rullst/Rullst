# Rullst Capital 💳

`rullst-capital` provides payment/payout adapter foundations, normalized billing
types, bounded webhook verification helpers, application-supplied revenue
snapshots, and a bounded National NFS-e preparation pipeline. Provider method
coverage is not uniform; inspect the selected adapter and test it in the
provider sandbox.

## 🚀 Core Features

- **Multi-Provider Architecture:** A unified billing surface across global, regional, Web3, and payout adapters. Capabilities vary by provider, and unsupported live operations fail closed.
- **Revenue snapshot (`/studio/capital`):** Displays metrics supplied explicitly
  by the application to a process-local `RevenueDashboardManager`; it is not an
  accounting ledger and does not infer money from event names.
- **Webhook event inspector:** Holds records explicitly passed to the local
  manager. Capital does not connect every webhook route to Studio automatically.
- **Webhook verification:** Provider-specific signature/freshness/replay
  foundations for documented adapters. The opt-in SQL replay ledger shares
  bounded payload digests or semantic provider event keys across processes on
  SQLite, PostgreSQL, MySQL, and MariaDB. Reconciliation and authorization
  remain application responsibilities.
- **Payment-bound invoices:** The opt-in `invoice-pdf` feature validates money
  into exact minor units, renders bounded paginated PDF and binds delivery to a
  final receipt matching recipient, amount and currency.
- **Provider-specific metered billing:** Current Stripe Meter Events and Lemon
  Squeezy Usage Records request/response contracts, bounded protocol parsing,
  deterministic non-live mocks and explicit retry evidence.
- **Shared team/workspace quotas:** Bounded subject identities, idempotent
  reservations, replay-safe execution and an opt-in transactional SQL store for
  SQLite, PostgreSQL, MySQL and MariaDB.
- **Coupons and relative trials:** A bounded/redacted coupon value, current
  Stripe discount binding, and 1–730-day trial updates for Stripe/Lemon Squeezy
  with explicit-clock retries and fail-closed provider capability boundaries.

---

## ✨ Supported Providers

The 12.1 SaaS/`make:billing` Stripe integration persists authorized
customer bindings, immutable attempts, Checkout Session IDs and atomic event
receipts. It reconciles current provider state under database revision fencing
and resumes existing open sessions. Configure the account, credentials, recurring
price allowlist and HTTPS return URL as described in generated `BILLING.md`.
Mixed credentials and other generated live providers remain unavailable.
Updating Capital does not rewrite existing controllers or apply new migrations.

| Provider | Adapter category | Current boundary |
| :--- | :--- | :--- |
| **Stripe** | Billing | Typed customer/subscription checkout, customer-ID portal, current-state reads and signed events; generated durable SQLx/Turso integration. Immediate Payment Intent charge is separate. |
| **Lemon Squeezy** | Billing | Checkout requires explicit `with_store_id`; store and variant response identities are checked. |
| **InfinitePay** | Billing | Offline fixtures; live plan-only checkout and body-only callback verification are unsupported. |
| **Polar** | Billing | Current typed product checkout, external customer binding and signed subscription events; legacy price-only checkout is unsupported. |
| **Paddle** | Billing | Typed customer/transaction checkout, approved Paddle.js payment page, bound signed subscription events and current-state reads; legacy email-only checkout is unsupported. |
| **Razorpay** | Billing | Adapter and signed-webhook foundation. |
| **Mercado Pago** | Billing | Offline checkout fixture; live plan-only checkout and body-only webhook verification are unavailable. |
| **Coinbase Commerce** | Billing | Signed-webhook foundation; live plan-only checkout is unsupported without authoritative pricing. |
| **PicPay** | Billing | Offline checkout fixture; live plan-only checkout is unsupported without authoritative pricing. |
| **Alipay** | Billing | Explicit mock credentials only; live checkout and RSA2 webhook verification are unsupported. |
| **Wise** | Payout | Status/webhook foundation; legacy email-based live transfer is unsupported. |

The shared `create_customer_portal(email, return_url)` methods do not have a
reviewed live provider-session contract and return `UnsupportedOperation` for
live credentials. Their deterministic empty/`mock_*` examples are offline
fixtures, not authenticated portal sessions. Live usage reporting through the
legacy uniform method is also unsupported for Paddle, Polar, Mercado Pago and
Razorpay; use the separate reviewed Stripe/Lemon Squeezy metered contracts when
applicable. InfinitePay, PicPay and Coinbase cancellation, plus Polar pause,
likewise reject live calls until an actual provider operation is implemented.

The legacy `create_checkout_session(email, plan_id, return_url)` accepts a
provider-managed plan/price identity, not an amount or currency. Mercado Pago,
Coinbase Commerce, InfinitePay and PicPay do not have an implemented reviewed
mapping for that contract; their live methods return `UnsupportedOperation`
before HTTP dispatch. They never invent a price, currency, buyer identity or
subscription from a plan label. Empty/`mock_*` API credentials preserve their
deterministic offline fixture, while `handle_*` and `picpay_token` are not mock
credentials. An authoritative typed pricing/provider contract is required
before enabling these live checkout paths.

| Reviewed legacy method boundary | Stripe | Lemon Squeezy | Paddle | Polar | Razorpay | Mercado Pago | Coinbase | InfinitePay | PicPay |
|---|---|---|---|---|---|---|---|---|---|
| Plan/price-based checkout request | adapter | adapter | unsupported | unsupported | adapter | unsupported | unsupported | unsupported | unsupported |
| Customer portal by email | unsupported | unsupported | unsupported | unsupported | unsupported | unsupported | unsupported | unsupported | unsupported |
| Immediate evidence-bound charge | adapter | unsupported | unsupported | unsupported | unsupported | unsupported | unsupported | unsupported | unsupported |

`adapter` means a bounded request implementation exists, not that this audit
validated acceptance or every response schema against a live provider account.
Offline fixtures are deliberately excluded from the live-method matrix.

InfinitePay's [checkout callback and payment lookup](https://www.infinitepay.io/checkout-documentacao)
do not establish the HMAC/subscription contract assumed by the old adapter.
The live body-only verifier and handler now return `UnsupportedOperation`;
explicit mock-secret verification remains available for offline fixtures.
Enabling a real callback needs reviewed authentication, merchant/order/amount
binding and authoritative reconciliation. A locally signed fixture does not
prove that the provider emits that protocol.

The v12.1 maintenance rejects legacy Paddle and Polar checkout before
network dispatch: their current provider contracts cannot be represented by the
old request shapes. Wise's email-based transfer method also fails explicitly;
it cannot infer a recipient account, authenticated quote or UUID idempotency
identity, and transfer creation is not funding. Their offline mocks remain
available. Polar and Paddle supply the explicit typed replacements below.
Wise still requires a dedicated recipient/quote/transfer/funding contract.
Provider-account sandbox acceptance remains separate from protocol tests.

Lemon Squeezy live checkout uses the merchant's explicit positive numeric store
ID: `LemonSqueezyProvider::new(key, webhook_secret).with_store_id(store_id)?`.
The plan argument must be a numeric variant ID belonging to that store. The
generated billing application reads `BILLING_STORE_ID`; missing configuration
fails before HTTP dispatch. Existing applications must adopt this setting.

### Transaction-based Paddle checkout

Configure a default payment-link page in the Paddle account and load Paddle.js
on that page. The domain must meet Paddle's approval rules. `checkout.url` is
this launcher, not a return URL after payment; setting a custom URL does not
remove the default-page prerequisite. `with_sandbox(true)` selects the sandbox
API explicitly.

```rust,no_run
use rullst_capital::{PaddleCheckoutRequest, PaddleCustomerRequest, PaddleProvider};

async fn checkout() -> Result<(), rullst_capital::CapitalError> {
    let provider = PaddleProvider::new("mock_key", "mock_secret").with_sandbox(true);
    let provision = PaddleCustomerRequest::new(
        "owner_opaque", "provision_unique", "customer@example.com",
    )?;
    // Authorize the owner and persist this intent before dispatch.
    let customer = provider.create_customer(&provision).await?;
    // Persist the customer binding before the separate checkout attempt.
    let attempt = PaddleCheckoutRequest::new(
        customer.id(), "pri_01h7vjes1v2y4d0v3t4b4e2q8s", "owner_opaque",
        "checkout_unique", "https://app.example/pay",
    )?;
    let session = provider.create_transaction_checkout(&attempt).await?;
    assert!(session.is_mock());
    // Persist the transaction ID before redirecting to an available session.url().
    Ok(())
}
```

The request binds one existing customer, one server-owned recurring price and
quantity one. Customer ownership is checked before transaction creation. The
response must match customer, owner, attempt, recurring price and payment page;
its only query parameter is `_ptxn` for that exact transaction. Receipts retain
the request digest and selected environment; mocks have no real environment.

Paddle does not support arbitrary client-supplied idempotency keys. The attempt
reference is correlation metadata. Persist it before dispatch and never blindly
retry an uncertain creation. `retrieve_bound_customer` and
`retrieve_transaction_checkout` reconcile independently recovered known IDs
without mutation or email-based ownership claims.

`verify_checkout_subscription` binds signed events to the request and persisted
transaction receipt. The first `subscription.created` must carry the matching
transaction ID. Later events require a receipt with the already-bound
subscription ID, obtained through the transaction read. Use
`retrieve_bound_subscription` for current-state reconciliation; commit event
receipts and domain changes atomically under a revision fence. Account scope,
entitlement policy and settlement remain application-owned. Cancellation and
pause respect the selected environment and validate the returned immediate or
scheduled change. See Paddle's [transaction creation](https://developer.paddle.com/api-reference/transactions/create-transaction/),
[payment-page setup](https://developer.paddle.com/build/transactions/pass-transaction-checkout/)
and [retry limitations](https://developer.paddle.com/sdks/libraries/).

### Product-based Polar checkout

Use product UUIDs and an opaque application-owned billing subject, not legacy
price IDs or email as identity. `with_sandbox(true)` selects the sandbox API.

```rust,no_run
use rullst_capital::{PolarCheckoutRequest, PolarProvider, CapitalError};
async fn checkout() -> Result<(), CapitalError> {
    let provider = PolarProvider::new("mock_token", "mock_secret").with_sandbox(true);
    let intent = PolarCheckoutRequest::new(
        "1dbfc517-0bbf-4301-9ba8-555ca42b9737", "opaque_billing_subject",
        "https://app.example/billing/return",
    )?;
    // Persist intent and authorize its owner before dispatch.
    let session = provider.create_product_checkout(&intent).await?;
    // Persist session.id() before redirecting to session.url().
    assert!(session.is_mock());
    Ok(())
}
```

Only supply `with_trusted_client_ip` from your socket/trusted-proxy resolver;
omit it when that boundary is unavailable. The adapter never trusts raw
`Forwarded`/`X-Forwarded-For`. Creation is not automatically retried: Polar's
checkout contract does not establish a provider idempotency guarantee. Use
`verify_checkout_subscription` with the persisted request to bind signed
subscription notifications to the external customer and product. Retain account,
environment, event receipts and entitlement/reconciliation policy in your app.
See [Polar's current checkout contract](https://polar.sh/docs/api-reference/checkouts/create-session).

### Customer-bound Stripe subscription checkout (12.1 working source)

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
not a newly generated attempt key. See Stripe's
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

The generated SaaS blueprint currently provides an application boundary for
the Stripe and Lemon Squeezy subset. It is not a conformance application for
all eleven Capital adapters. A release claim should name the exact provider,
operation, environment and observed result rather than saying that “payments
work.” See the official [Stripe testing](https://docs.stripe.com/testing) and
[sandbox](https://docs.stripe.com/sandboxes) guidance and Lemon Squeezy's
[test-mode](https://docs.lemonsqueezy.com/help/getting-started/test-mode) and
[webhook simulation](https://docs.lemonsqueezy.com/help/webhooks/simulate-webhook-events)
guidance.

Mercado Pago signs a manifest containing the original query data ID, request
ID and timestamp. Its notification also requires an authoritative resource
lookup before inferring payment/subscription state. The v12 body-only verifier
cannot establish that contract and therefore rejects live verification;
`with_webhook_tolerance` remains source-compatible but has no effect on that
unavailable path. Explicit `mock_*` webhook fixtures remain supported. See the
[official Mercado Pago webhook contract](https://www.mercadopago.com.br/developers/en/docs/subscriptions/additional-content/your-integrations/notifications/webhooks).

Polar's live `handle_webhook` validates the full Standard Webhooks envelope:
ID, timestamp and raw payload are signed together; versioned Base64 signatures
are bounded and timestamps have a five-minute window. The documented Polar
legacy literal-secret and standard `whsec_` decoded-secret schemes are both
accepted, matching the provider's SDK transition. Its old body-only
`verify_signature` method returns `UnsupportedOperation` for live secrets.
See [Polar's signing contract](https://polar.sh/docs/integrate/webhooks/delivery)
and the [Standard Webhooks specification](https://github.com/standard-webhooks/standard-webhooks/blob/main/spec/standard-webhooks.md).

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

Missing or malformed status no longer implies a paid/active subscription.
Lemon Squeezy accepts only its subscription lifecycle kinds and `subscriptions`
objects with positive numeric IDs, valid states and consistent test-mode fields.
An explicitly configured store must match. `on_trial` becomes `Trialing`;
cancelled/expired snapshots retain a required valid end time as `Canceled`.
Grace-period access remains application policy. Invoice/payment/refund events
need separate processing; they are not subscription snapshots. See the provider's
[subscription object](https://docs.lemonsqueezy.com/api/subscriptions/the-subscription-object)
and [event types](https://docs.lemonsqueezy.com/help/webhooks/event-types).
The legacy normalized event does not retain the provider mode or durable event
identity; validated parsing alone does not complete owner binding or an inbox.

Unsupported Razorpay and Coinbase event kinds fail closed; Coinbase event
names are matched exactly, not by substring. This does not establish every
provider payload schema or an application-specific entitlement/tenant policy.

Razorpay subscription normalization requires the subscription's own bounded ID,
customer ID and plan ID, plus an event/entity state match. Authentication alone
and standalone payment/order events cannot activate a subscription. Activated,
charged and resumed events require `active`; pending, halted, paused and
cancelled events require their corresponding provider state. Completed and
authenticated states remain unsupported by the v12 normalized contract. Email
is optional contact data. The application still owns customer/tenant binding,
event ordering, durable processing and reconciliation; `Active` is a lifecycle
state, not proof that a particular invoice was paid. See Razorpay's
[subscription states](https://razorpay.com/docs/payments/subscriptions/states/)
and [webhook payloads](https://razorpay.com/docs/webhooks/subscriptions/).

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
rullst-capital = "12.0.0"
```

The heavier NFS-e schema/signature boundary is opt-in:

```toml
rullst-capital = { version = "12.0.0", features = ["nfse"] }
```

Native invoice PDF is independently opt-in. One-call Mail delivery uses the
downstream `rullst-mail/capital-invoice` feature, or `rullst/capital-mail` when
using the umbrella crate:

```toml
rullst = { version = "12.0.0", features = ["capital-mail"] }
```

Durable relational quota accounting is separately opt-in:

```toml
rullst = { version = "12.0.0", features = ["capital-quota-sql"] }
# Or directly: rullst-capital = { version = "12.0.0", features = ["quota-sql"] }
```

Durable cross-process webhook replay claims are independently opt-in:

```toml
rullst = { version = "12.0.0", features = ["capital-webhook-sql"] }
# Or directly: rullst-capital = { version = "12.0.0", features = ["webhook-sql"] }
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
subscription owner's tier rather than a client payload. `QuotaGate` atomically
reserves before calling the application operation, skips exact idempotent
replays and releases a fresh reservation when the callback returns an error.

The always-available `InMemoryQuotaStore` is deterministic and process-local.
With `quota-sql`, `SqlQuotaStore` persists a unique event claim and conditionally
increments the shared counter on SQLite, PostgreSQL, MySQL or MariaDB. For a
relational create that must be atomic with accounting, open a transaction from
`store.pool()`, call `reserve_with_transaction`, execute the domain insert on
that same transaction and commit once. See the
[SaaS billing tutorial](https://rullst.github.io/Rullst/book/tutorials/19-saas-billing-capital.html#8-enforce-one-shared-workspace-quota-before-creation)
for the complete flow.

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
contains both the requested subscription and coupon. Lemon Squeezy discount
codes are checkout-only, so applying one to an existing live subscription
returns `UnsupportedOperation`; unreviewed live adapters do the same.

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
the explicit absolute operation. Stripe and Lemon Squeezy bind trial-update
responses, but authorization, concurrent-command serialization, billing-cycle
policy, signed-webhook reconciliation and real-account acceptance remain host
or release work.

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

`LemonSqueezyUsageRecord` instead requires the provider's numeric subscription
item ID and an explicit `Increment` or `Set` action. The action must match the
aggregation configured for that variant. Lemon Squeezy's request does not carry
the application's event key, so atomically claim `event_key()` in a durable
outbox before submission. Stripe's identifier is provider-forwarded but only
has a rolling deduplication guarantee. Empty or `mock_*` keys return a stable
`UsageStatus::Mock`, never a live acceptance.

### Payment-Bound Invoice Delivery

`Invoice::bind_succeeded_charge` accepts only final `Succeeded` evidence with
an exact recipient, minor-unit amount and currency match. The resulting
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

`rullst-capital` includes Axum and opt-in Actix Web middleware adapters over one canonical [`webhook` verifier](https://github.com/Rullst/Rullst/blob/main/rullst-capital/src/webhook.rs). Both bound the body, verify supported provider signatures, enforce timestamp freshness for Stripe, Paddle and Polar, reject duplicate Standard Webhooks envelope headers, restore the exact body, insert a normalized event, and reject replayed payloads through a bounded TTL store. Live Mercado Pago verification is unavailable through this body-only API. Empty webhook secrets are configuration errors. `mock_*` secrets are explicit local fixtures and are rejected by the production-safe entry points. The in-memory store now fails closed when full instead of discarding an unexpired replay proof.

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
another node's proof early.

That middleware path records the payload before calling the handler, so it is
a replay firewall rather than an exactly-once delivery protocol. A crash after
admission can still occur before a business mutation.

When a verified provider protocol supplies a stable event ID, prefer
`check_and_record_event_key` over payload-only replay detection. A relational
handler that uses the provider's low-level signature contract can call
`check_and_record_event_key_with_transaction` and write its domain mutation
through the same transaction before one commit. Do not pre-claim the same event
through SQL middleware on this atomic path. This is atomic only inside that
database: provider API calls, e-mail, queues, and other systems still require
an outbox, idempotent consumers, and reconciliation.

---

## 🧾 NFS-e Padrão Nacional — Homologation Preparation

The local pipeline now implements a bounded ordinary-service DPS 1.01 builder,
checksum-pinned validation against official production/restricted XSD sources,
PKCS#12 RSA-SHA256 XMLDSig with inclusive C14N 1.0, independent local
signature verification, deterministic GZip/Base64 issuance JSON, bounded
signed-authorization and structured-rejection parsing, and rustls mTLS client
construction. The signed request now carries its parsed `tpAmb`, so a caller
cannot reinterpret a homologation DPS as production (or the reverse).
Certificate bytes, passphrases, and derived PEM are redacted and zeroized where
owned by Rullst.
The production profile applies one exact, documented in-memory compatibility
normalization after hash verification: it removes `.NET` `^...$` anchors from
the known DPS-series pattern because XSD regex grammar treats them as literals.

The same `nfse` feature includes `FiscalCommandJournal`, a bounded local
single-active-writer journal for the caller-owned transport workflow. It
synchronously records one `prepared` command and one bound `authorized` or
`rejected` terminal result, suppresses exact replays, rejects command-key
conflicts, and recovers unresolved descriptors after restart. A named 256-bit
HMAC key authenticates the header and a chain of at most 4,096 frames/16 MiB;
an independently retained exact-tip checkpoint detects valid-prefix
truncation. The file contains only the opaque application command ID,
request/result digests, environment, state, and bounded timestamps—not XML,
access keys, certificate material, provider bodies, or processing messages.

This is preparation for homologation, not live issuance. `Homologation` and
`Production` still return `FiscalError::Unsupported` without network I/O until
full certificate/emitter and ICP-Brasil policy, authoritative request/outbox
storage, reconciliation, retained official protocol fixtures, real
restricted-environment evidence, independent review, and official homologation
are complete. The journal itself does not send or retry a request, provide a
multi-process lock, or prove cross-system exactly-once behavior. The host owns
a non-PII command namespace, key custody/rotation, trusted directory, exclusive
writer, actual request storage, external checkpoint, retention, and backup.

Enable the crate's `nfse` feature (or umbrella `rullst/capital-nfse`) for the
XSD, XMLDSig, protocol codec, and mTLS preparation APIs. The strict DPS builder
and unmistakable offline mock remain available through the base Capital crate.

The runnable [`nfse_v101_preview`](https://github.com/Rullst/Rullst/blob/v12.0.0/rullst-capital/examples/nfse_v101_preview.rs) example emits
the unsigned bounded DPS. When `RULLST_NFSE_XSD_DIR` points to an extracted
official production package whose files match the pinned hashes, it validates
the document before writing it:

```bash
RULLST_NFSE_XSD_DIR=/path/to/NFSe/Schemas/1.01 \
  cargo run -p rullst-capital --example nfse_v101_preview
```

Only `NfseEnvironment::Mock` is executable. Its response is typed as `FiscalResponseKind::OfflineMock`, uses `MOCK_NOT_AUTHORIZED`, and must never be accounted as an issued invoice:

```rust
use rullst_capital::fiscal::{
    issue_nfse_direct, FiscalCertificate, FiscalCustomer, FiscalEmitter,
    FiscalResponseKind, NfseEnvironment, TaxRegime,
};

// 1. Configure the emitting SaaS company
let emitter = FiscalEmitter {
    cnpj: "12.345.678/0001-90".to_string(),
    inscricao_municipal: "1234567".to_string(),
    legal_name: "Minha Empresa SaaS Ltda".to_string(),
    trade_name: Some("MeuSaaS".to_string()),
    ibge_code: "3550308".to_string(), // São Paulo
    tax_regime: TaxRegime::SimplesNacional,
};

// 2. Customer data
let customer = FiscalCustomer {
    doc_number: "123.456.789-00".to_string(),
    name: "João Silva".to_string(),
    email: "joao@cliente.com.br".to_string(),
    zip_code: Some("01310-100".to_string()),
    address: Some("Av Paulista, 1000".to_string()),
    ibge_code: Some("3550308".to_string()),
};

// 3. Convert paid invoice to national DPS format
let dps = invoice.to_dps("1.03.01", "3550308", 2.0); // 1.03.01 = SaaS & Hosting, 2.0% ISS

// 4. Mock mode does not load or use a real certificate.
let cert = FiscalCertificate::offline_mock();

// 5. Produce a deterministic offline fixture; no network request is made.
let response = issue_nfse_direct(
    &emitter,
    &customer,
    &dps,
    &cert,
    NfseEnvironment::Mock,
).await?;
assert_eq!(response.kind, FiscalResponseKind::OfflineMock);
assert!(!response.is_officially_authorized());
```

---

## 🔐 Security Invariants

- **Constant-Time Verification**: Supported HMAC/token signatures use cryptographic verification or `subtle::ConstantTimeEq`.
- **Fail-Closed Configuration**: Empty webhook secrets never authenticate a request; mock credentials require a deliberate `mock_*` value.
- **Freshness and Replay Protection**: Timestamped protocols have a configurable five-minute window, and middleware records provider-scoped payload hashes in a bounded 24-hour TTL store. `webhook-sql` adds bounded durable claims; stable semantic event IDs can share a transaction with the domain mutation.
- **Alipay Containment**: Live RSA2 checkout and webhook verification return `UnsupportedOperation`; only explicitly mock-prefixed credentials operate offline.
- **Fiscal Containment**: Local XSD/XMLDSig/mTLS preparation and the authenticated command journal are not an official NFS-e authorization; live transmission remains disabled until the documented external gates pass.
