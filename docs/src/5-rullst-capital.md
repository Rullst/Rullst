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

Webhook endpoints must use the Capital verification middleware. For supported
protocols it performs cryptographic verification, freshness checks, and bounded
replay protection before the application receives a normalized event. A webhook
route may receive a narrowly scoped CSRF exemption only when this verifier remains
mandatory on that exact route.

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

## NFS-e Nacional: contained offline preview

The current fiscal module can construct an escaped DPS XML fixture. It does not
claim a valid XMLDSig signature or an authorization from the Brazilian national
NFS-e service.

- `NfseEnvironment::Mock` returns `FiscalResponseKind::OfflineMock`, status
  `MOCK_NOT_AUTHORIZED`, and `is_officially_authorized() == false`.
- `NfseEnvironment::Homologation` and `NfseEnvironment::Production` fail closed
  with `FiscalError::Unsupported`.
- `sign_dps_xml` also fails closed; it never fabricates a `<Signature>` element.

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
    let unused_certificate = FiscalCertificate::from_base64("", "");
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

Live issuance remains roadmap work until PKCS#12 private-key handling, XML
C14N/XMLDSig, XSD validation, mTLS, strict response parsing, rejection handling,
and official end-to-end homologation are independently verified. Do not account
an offline fixture as an issued invoice.

## Operational checklist

- Derive tenant identity from an authenticated context, not an arbitrary client
  header.
- Keep webhook secrets non-empty, rotate them, and retain replay state according
  to the provider contract.
- Reconcile provider events idempotently in the application database.
- Treat Studio revenue panels as observability views, not an accounting ledger.
- Verify every adapter capability in a sandbox before enabling live traffic.
