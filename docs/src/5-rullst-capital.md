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

### Immediate charges without raw payment data

`Billable::charge_with` accepts only a provider-tokenized customer and payment
method. Amounts are integer minor units and every attempt needs an
application-owned idempotency key. The reviewed live adapter is Stripe Payment
Intents; other billing adapters fail with `UnsupportedOperation` instead of
pretending to charge.

```rust,no_run
use rullst_capital::{Billable as _, CapitalError, StripeProvider};

async fn collect_order(
    account: &impl rullst_capital::Billable,
    stripe: &StripeProvider,
) -> Result<(), CapitalError> {
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

Webhook endpoints must use the Capital verification middleware. Its Axum and
opt-in Actix adapters call the same canonical verifier. For supported protocols
it performs cryptographic verification, freshness checks, a two-megabyte body
limit, and bounded replay protection before the application receives a
normalized event. A webhook route may receive a narrowly scoped CSRF exemption
only when this verifier remains mandatory on that exact route. See the
[payment guide](payment-gateways-guide.md#actix-web-adapter) for Actix setup.

```rust,no_run
use axum::{routing::post, Router};
use rullst_capital::verify_webhook;

async fn billing_webhook() {
    // Read the verified event inserted by the middleware in real handlers.
}

let router = Router::new()
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
limits and never turns a rejection or unsigned/tampered XML into authorization.

Live issuance remains disabled until full certificate/emitter and ICP-Brasil
policy, durable idempotency/audit, retained official fixtures, real A1
restricted-environment tests, independent review, and official end-to-end
homologation are complete. Follow the
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
