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
- the official request/response envelope and external homologation gates remain
  deliberately disconnected from the network path.

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

Live issuance remains disabled until the official JSON envelope, strict bounded
response/rejection parser, full certificate/emitter and ICP-Brasil policy,
durable idempotency/audit, real A1 restricted-environment tests, independent
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
