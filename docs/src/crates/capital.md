# Rullst Capital

> **Vision preserved:** live NFS-e, Alipay RSA2, broader gateway coverage, and
> durable idempotency were not deleted. Their status and engineering recommendation
> are recorded in the [capability ledger](../capability-ledger.md#capital-billing-and-fiscal-vision).

`rullst-capital` provides billing abstractions, revenue metrics, payment-provider
adapters, payout helpers, and verified webhook plumbing. Adapter capabilities are
provider-specific; a shared trait does not imply that every checkout, refund,
payout, or webhook operation is available everywhere.

## Invariants

- Live webhook endpoints reject empty and mock secrets.
- Supported signed webhook protocols use cryptographic verification, freshness
  checks, and bounded replay protection before exposing a normalized event.
- Deterministic `mock_*` credentials provide offline fixtures where documented;
  they never turn a public endpoint into an authenticated live webhook.
- Tenant identity must come from authenticated membership, not an arbitrary
  client-supplied header.
- Unsupported Alipay RSA2 and fiscal live paths return typed errors.

## NFS-e boundary

The fiscal module can construct an escaped DPS XML fixture in
`NfseEnvironment::Mock`. Its response is `FiscalResponseKind::OfflineMock`, uses
`MOCK_NOT_AUTHORIZED`, and returns false from `is_officially_authorized()`.

Homologation and production fail closed until PKCS#12 key handling, XML
C14N/XMLDSig, XSD validation, mTLS, strict authority-response parsing, rejection
handling, and official end-to-end homologation are independently verified.
`sign_dps_xml` does not fabricate a signature.

See [Rullst Capital: billing and fiscal boundaries](../5-rullst-capital.md) for
the integration contract and operational checklist.

## Application responsibilities

- Test every selected adapter in its official sandbox before live traffic.
- Keep and rotate provider/webhook secrets outside source control.
- Reconcile webhook events idempotently and within the authenticated tenant.
- Treat revenue dashboards as observability views, not accounting ledgers.
- Never record an offline DPS fixture as an issued fiscal document.
