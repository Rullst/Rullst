# Migrating from 12.0 to 12.1

12.1 is a maintenance candidate until its admitted tag is published. Updating
dependencies or the CLI does not rewrite generated application files, migrate
databases or redeploy the Azure examples. Keep the existing application and
lockfile, generate a separate comparison project, and review the changes below.

## Billing migration

| Existing integration | Required change |
|---|---|
| Generated SaaS/`make:billing` on 12.0 | Adopt the new billing modules and migrations in a reviewed application change. Configure the provider account, mode, recurring price allowlist and HTTPS URLs from generated `BILLING.md`. |
| Stripe email-based owner lookup | Persist opaque local owner/customer/account/mode bindings. Use typed customer and checkout requests with durable immutable attempt keys; store session IDs before HTTP 303. |
| Stripe normalized events alone | Adopt signed Checkout/subscription envelopes, current-state reads and atomic inbox/domain commits under database revision fencing. Never grant access from email or a return-page visit. |
| Stripe portal by email | Use the typed customer-ID-bound portal operation after authorizing the persisted binding. Other legacy uniform live portal calls remain unsupported. |
| Paddle legacy checkout | Migrate to `PaddleCustomerRequest`, `PaddleCheckoutRequest` and `create_transaction_checkout`. Configure the default approved Paddle.js payment page, select sandbox explicitly and persist all attempt/customer/transaction bindings. |
| Polar price-based checkout | Migrate to `PolarCheckoutRequest` and `create_product_checkout` with a product UUID and opaque external customer identity. Supply client IP only from a reviewed trusted-proxy boundary. |
| Lemon Squeezy checkout | Supply the merchant's positive numeric store ID through `with_store_id` and a variant belonging to that store. |
| Wise email-based transfer | Do not invoke it with real credentials. Recipient, quote, transfer and funding need separate reviewed contracts; the legacy call fails before network dispatch. |

The new generated durable real-provider integration is Stripe-specific. Paddle
and Polar expose typed adapter operations; their application persistence and
event orchestration remain explicit. Consult the
[complete provider matrix](https://github.com/Rullst/Rullst/blob/main/rullst-capital/README.md#-supported-providers)
before enabling a method. Empty/`mock_*` credentials are deterministic local
fixtures; mixed configuration must not silently produce a real-payment success.

Do not replay a Paddle or Polar creation merely because it timed out. Their
typed correlation metadata does not establish provider idempotency. Recover
known objects and reconcile uncertain outcomes before another mutation. Stripe
also needs durable attempt retention and reconciliation after its idempotency
window. Generated `BILLING.md` describes recovery and replacement subscriptions.

Existing customer rows cannot be adopted just by matching email. Back up the
application database, review schema/unique constraints and migrate only bindings
established from authorized provider evidence. Do not replace paid production
tables with empty generated models. Test rollback and restart before rollout.

## Application and CLI changes

- Replace `Redirect::temporary`/307 for hosted checkout with a 303 handoff.
  Add the exact provider checkout origin to application CSP `form-action`;
  Stripe's default SaaS policy permits `https://checkout.stripe.com`.
- Commit application `Cargo.lock` and use locked container builds. Remove the
  generated unsupported MSVC `/DEBUG:FASTLINK` flag from existing projects.
- Review strict ORM feature edges: backend-specific builds require disabled
  default features throughout the resolved dependency graph, not just one crate.
- Nexus and AI share provider resolution, including explicitly configured Groq.
  Azure Basic authentication still requires the explicit trusted-TLS capability;
  raw forwarding headers do not establish trusted termination.
- Core's default WAF no longer blocks ordinary HTTP libraries by user agent.
  Explicit application blocklists remain unchanged and may need local review.
- Remove application workarounds for Studio embedded assets/cache navigation and
  the Nexus mobile drawer only after testing the updated native renderers.
- Native CLI installation/update requires the admitted release artifacts and
  their verification evidence. Candidate CI binaries are not release assets.
  Application updates require a reviewed diff; installing the CLI alone does
  not update an application's dependencies or migrations.

## Acceptance and release evidence

Run the provider's sandbox journey against the candidate, including signed
events, owner isolation, retry/replay, cancellation and current-state recovery.
Keep fixture tests, hosted CI and provider-account acceptance distinct. A
Chromium navigation to hosted Checkout proves handoff only; it does not prove
payment, webhook delivery or subscription reconciliation. A one-time payment
application does not validate the framework's recurring subscription flow.

The release additionally requires the full workspace all-feature tests, strict
Clippy, formatting, package/consumer checks, coverage, SemVer/security checks and
every workflow listed in `.github/release-required-workflows.json` at the exact
main commit. Only then may the protected release pipeline publish the sixteen
crates in `.github/release-order.json` and attach verified native CLI binaries.
See [release recovery](release-recovery.md) for partial publication handling.
