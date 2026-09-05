# Rullst Capital: billing and fiscal boundaries

> **Vision preserved:** capabilities removed from the usable-today contract are
> still evaluated item by item in the [capability ledger](capability-ledger.md#capital-billing-and-fiscal-vision),
> including whether each one is worth implementing and why.

`rullst-capital` provides billing abstractions, revenue metrics, payment-provider
adapters, payout helpers, and verified webhook plumbing. Provider capabilities
are not uniform: consult the adapter API and its tests before depending on a
particular checkout, refund, payout, or webhook operation.

`RevenueDashboardManager` is a bounded process-local presentation source.
Applications call `update_metrics` with values reconciled from their durable
billing database and call `record_event` after a verified webhook path. Recording
an event deliberately does not invent a plan price, fee, currency or subscriber
count from its event name.

## Payment providers

Initialize only the provider required by the application and treat credentials
as deployment secrets. Empty credentials are configuration errors for live
operations. Credentials deliberately prefixed with `mock_` select deterministic
offline behavior where that adapter documents support for it.

Every reviewed live method uses the same pooled outbound client with a
five-second connect timeout, twenty-second whole-request timeout, disabled
redirects and ambient proxy discovery, and one-MiB JSON limit. Returned
checkout locations must be bounded absolute HTTPS URLs without credentials or
fragments. `CapitalError::Provider` exposes redacted static provider/operation
labels, failure kind, optional HTTP status and bounded numeric `Retry-After`.
Its permanent/transient/rate-limited class is evidence for application policy,
not permission to repeat a mutation. Only retry after proving that the exact
operation forwards a persisted idempotency key and retaining reconciliation.

### Immediate charges without raw payment data

`Billable::charge_with` accepts only a provider-tokenized customer and payment
method. Amounts are integer minor units and every attempt needs an
application-owned idempotency key. The reviewed live adapter is Stripe Payment
Intents; other billing adapters fail with `UnsupportedOperation` instead of
pretending to charge.

```rust,no_run
use rullst_capital::{Billable as _, CapitalError, StripeProvider};

async fn collect_order<Account>(
    account: &Account,
    stripe: &StripeProvider,
) -> Result<(), CapitalError>
where
    Account: rullst_capital::Billable + Sync,
{
    let receipt = account
        .charge_with(
            stripe,
            4_990, // BRL 49.90 in cents
            "BRL",
            "cus_provider_owned",
            "pm_provider_tokenized",
            "order_42-attempt_1",
        )
        .await?;
    if receipt.is_succeeded() {
        // Reconcile durable order state; do not grant access from this alone.
    }
    Ok(())
}
```

The application must establish the customer's authority over both provider
IDs, retain the key with its order, reconcile signed webhooks and handle
mandate/SCA rules. Empty/`mock_*` credentials produce a deterministic receipt
with the distinct non-success `ChargeStatus::Mock`; it must never grant access
or be booked as revenue. This API has no raw-card field and never guesses
currency.

## Provider-specific metered usage

Use the static-dispatch `MeteredBillingProvider` boundary for new code.
`StripeMeterEvent` requires the authoritative `cus_*` customer, configured
meter event name, positive quantity, timestamp and a unique visible identifier;
the adapter sends the default `stripe_customer_id`/`value` payload and binds all
of those fields in the accepted response. `LemonSqueezyUsageRecord` requires
the provider's numeric subscription-item ID plus the explicit `Increment` or
`Set` action and binds item/quantity/action from the JSON:API response.

Both responses are capped at one MiB and offline credentials return a
deterministic `UsageStatus::Mock`. Stripe provides only rolling identifier
deduplication. The reviewed Lemon request has no provider event-key field, so
the application must atomically claim `event_key()` in a durable outbox before
sending. It must also configure the matching aggregation, reconcile invoices
and derive quotas/entitlements from authoritative state. The old uniform
`BillingProvider::report_usage` remains source-compatible for mocks but fails
closed in live Stripe/Lemon configurations instead of guessing required fields.

## Coupons and relative trial extensions

`CouponCode` validates/redacts the provider coupon identifier, while a
statically dispatched `SubscriptionHandle` applies it. Stripe uses the current
expanded `discounts[0][coupon]` update and binds the response to the requested
subscription/coupon. Lemon Squeezy codes are checkout-only; Lemon and adapters
without reviewed subscription-discount contracts return `UnsupportedOperation`
for live credentials instead of silently succeeding.

`handle.extend_trial(15)` resolves a bounded 15-day expiration from the current
UTC clock. Retryable workers should persist the command creation time and call
`extend_trial_days_at(15, command_created_at)`; this emits the same absolute
expiration on every attempt. `set_trial_end` is available for explicit
reconciliation. Stripe and Lemon Squeezy have bounded protocol/response-binding
tests for trial updates. Authorization, command serialization, billing-cycle
policy, webhook reconciliation and live account acceptance remain host/release
responsibilities. See the [billing tutorial](tutorials/19-saas-billing-capital.md#7-use-a-bounded-subscription-handle-and-grace-period).

## Shared subscriptions and strict resource quotas

One Team/Workspace model can own `Billable` and its tier policy while every
authorized member uses the same bounded `BillingSubject`. Derive that subject
from trusted `TenantContext`, never from a client-selected owner ID.
`Billable::quota_request` derives the limit from `tier_limit`; `QuotaGate`
reserves before invoking a create operation, suppresses an exact retry and
compensates an ordinary callback error.

`InMemoryQuotaStore` provides deterministic process-local behavior. Enable
`quota-sql` directly or `capital-quota-sql` on the umbrella crate for a durable
unique-claim and conditional-counter implementation on SQLite, PostgreSQL,
MySQL and MariaDB. Use `reserve_with_transaction` and perform the domain insert
in that same transaction whenever quota and creation must commit atomically.
The [complete tutorial](tutorials/19-saas-billing-capital.md#8-enforce-one-shared-workspace-quota-before-creation)
shows this path and its replay semantics.

The application still owns membership, authoritative tier/webhook state,
migrations, reconciliation of abandoned standalone reservations and adapters
for Turso or non-relational stores. Rullst does not intercept writes performed
outside the explicit quota boundary.

## Payment-bound invoice PDF and delivery

Enable the umbrella `capital-mail` feature (or Capital's `invoice-pdf` and
Mail's `capital-invoice` features separately). `Invoice::bind_succeeded_charge`
rejects non-final/mock receipts and any recipient, minor-unit total or currency
mismatch. `PaidInvoiceDelivery::prepare` then creates escaped HTML, a bounded
native PDF attachment and a message that has already passed Mail pre-flight.
It can use the configured facade, a tenant route or an explicit static driver.

The returned stable `delivery_key` is for a unique durable outbox record owned
by the application. Rullst does not infer a webhook, claim that key atomically,
guarantee attachment support at every provider or promise exactly-once
delivery. See the [SaaS billing tutorial](tutorials/19-saas-billing-capital.md#4-render-and-deliver-the-invoice-only-after-final-success).

Webhook endpoints must use the Capital verification middleware. Its Axum and
opt-in Actix adapters call the same canonical verifier. For supported protocols
it performs cryptographic verification, freshness checks, a two-megabyte body
limit, and bounded replay protection before the application receives a
normalized event. A webhook route may receive a narrowly scoped CSRF exemption
only when this verifier remains mandatory on that exact route. See the
[payment guide](payment-gateways-guide.md#actix-web-adapter) for Actix setup.

The default store is process-local. The opt-in `webhook-sql` feature persists
bounded provider-scoped payload digests or stable event IDs across SQLite,
PostgreSQL, MySQL, and MariaDB processes. SQL-backed middleware is a replay
firewall that claims before dispatch, not exactly-once delivery. For an atomic
relational business transition, use the verified provider event ID with
`check_and_record_event_key_with_transaction` in the domain transaction and do
not pre-claim it through SQL middleware. Cross-system effects still require an
outbox, idempotent consumers, and reconciliation.

```rust,no_run
use axum::{routing::post, Router};
use rullst_capital::verify_webhook;

async fn billing_webhook() {
    // Read the verified event inserted by the middleware in real handlers.
}

let router: Router = Router::new()
    .route("/webhooks/billing", post(billing_webhook))
    .layer(axum::middleware::from_fn(verify_webhook));
```

## NFS-e Nacional: bounded homologation preparation

The fiscal module can construct a strict ordinary-service DPS 1.01 subset,
validate it against checksum-pinned official schema sources with the one
documented production regex normalization, sign its `infDPS/@Id` with a
matching RSA key/certificate from PKCS#12, independently verify the local
XMLDSig, and construct the bounded rustls mTLS client. These local properties
do not constitute an authorization from the Brazilian National NFS-e service.

- `NfseEnvironment::Mock` returns `FiscalResponseKind::OfflineMock`, status
  `MOCK_NOT_AUTHORIZED`, and `is_officially_authorized() == false`.
- `NfseEnvironment::Homologation` and `NfseEnvironment::Production` fail closed
  with `FiscalError::Unsupported`.
- `sign_dps_xml` rejects malformed, duplicate-ID, already-signed, non-RSA, and
  mismatched key/certificate inputs instead of returning partial signature XML.
- `NfseIssueRequest` verifies the embedded DPS signature, produces the exact
  deterministic `dpsXmlGZipB64` JSON object and parses bounded signed success or
  structured rejection material without performing network I/O.
- transmission and the external homologation gates remain deliberately
  disconnected from the network path.

```rust,no_run
use rullst_capital::fiscal::{
    issue_nfse_direct, FiscalCertificate, FiscalCustomer, FiscalEmitter,
    FiscalResponseKind, NfseDps, NfseEnvironment,
};

async fn offline_preview(
    emitter: &FiscalEmitter,
    customer: &FiscalCustomer,
    dps: &NfseDps,
) -> Result<(), rullst_capital::fiscal::FiscalError> {
    let unused_certificate = FiscalCertificate::offline_mock();
    let response = issue_nfse_direct(
        emitter,
        customer,
        dps,
        &unused_certificate,
        NfseEnvironment::Mock,
    )
    .await?;

    assert_eq!(response.kind, FiscalResponseKind::OfflineMock);
    assert!(!response.is_officially_authorized());
    Ok(())
}
```

The offline protocol boundary is intentionally separate from transport:

```rust,no_run
use rullst_capital::fiscal::NfseIssueRequest;

# fn prepare(signed_dps: &str) -> Result<Vec<u8>, rullst_capital::fiscal::FiscalError> {
let request = NfseIssueRequest::try_from_signed_dps(signed_dps)?;
let exact_json_body = request.to_json()?;
# Ok(exact_json_body)
# }
```

An application may retain this material for a reviewed homologation fixture,
but the Rullst client does not send it. `parse_response` accepts only the
documented 201/400/403/500 issuance outcomes, applies four-MiB/cardinality/text
limits, binds the selected environment to the signed `infDPS/tpAmb`, and never
turns a rejection or unsigned/tampered XML into authorization.

The same `nfse` feature exposes `FiscalCommandJournal`: a bounded,
single-active-writer HMAC-chained file that synchronizes a prepared command
before a caller-owned transport, records one bound terminal response, suppresses
exact replay, and recovers minimized pending descriptors after restart. It
stores no XML, access key, processing message, provider body, or certificate.
The host must retain the actual request/outbox and the journal checkpoint
separately, protect and rotate the 32-byte key, enforce an exclusive writer,
and own reconciliation, retry, retention, and backup.

Live issuance remains disabled until full certificate/emitter and ICP-Brasil
policy, deployment of the local journal and authoritative request/outbox,
retained official fixtures, real A1 restricted-environment tests, independent
review, and official end-to-end homologation are complete. Follow the
[NFS-e homologation-preparation tutorial](tutorials/40-nfse-homologation-preparation.md)
and never account an offline fixture as an issued invoice.

## Operational checklist

- Derive tenant identity from an authenticated context, not an arbitrary client
  header.
- Keep webhook secrets non-empty, rotate them, and retain replay state according
  to the provider contract.
- Reconcile provider events idempotently in the application database.
- Treat Studio revenue panels as observability views, not an accounting ledger.
- Verify every adapter capability in a sandbox before enabling live traffic.
