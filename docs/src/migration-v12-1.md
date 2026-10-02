# Migrating from 12.0 to 12.1

Version [12.1.1 is published](v12.md#1211-published-maintenance-release).
Updating dependencies or the CLI does not rewrite generated application files, migrate
databases or redeploy the Azure examples. Keep the existing application and
lockfile, generate a separate comparison project, and review the changes below.

<a id="preparing-for-the-1211-patch"></a>

## Upgrading to the 12.1.1 patch

The [12.1.1 maintenance change set](v12-1-1-review.md) preserves the 12.1.0 API,
database schema and Rust 1.96.0 MSRV. Applications using public scaffold keys
must replace them with unique secure secrets; changing a session-encryption key
requires session renewal. HTTP drain now accounts for unfinished response
bodies, and SQLite message leases use the time after acquiring the write lock.
Existing generated LMS files are not rewritten by a dependency update.

## Account mail and machine requests

The [account-mail guide](account-mail-v12-1.md) describes the opt-in durable
PostgreSQL/SQLite recovery registry and worker. Existing reset templates benefit
from the mandatory-pipeline fix, but existing encrypted-cookie sessions and
password tables do not gain revocation automatically. Review imports before
switching the authoritative account store. Keep provider tracking disabled.

`Server::with_machine_endpoints` accepts exact method/path registrations with
mandatory bearer, signed-webhook or transport mTLS verification. Use it for
trusted machine POST routes instead of disabling browser CSRF globally. It
rejects browser-cookie/origin inputs; the application still owns domain
permissions and ingress rate limits.

Core with `orm` and disabled defaults now requires an explicit `strict-*`
backend or `drivers-all`. Studio and Nexus retain their default convenience
features; disable defaults to obtain an exclusive backend graph. Studio's
SQLite queue is controlled by `queue-sqlite`, independent of PostgreSQL/MySQL.
The facade forwards these choices, including optional Studio/Nexus consumers.

## Billing migration

Generated real-mode Stripe billing now additionally requires
`BILLING_LIVE_ACKNOWLEDGEMENT=I_UNDERSTAND_REAL_CHARGES`. This is a launch control,
not a substitute for merchant activation and refund/dispute procedures. Test
credentials do not need that acknowledgement.

For a one-time purchase, use the opt-in `StripePaymentCheckoutRequest` and
`create_one_time_checkout` API, with `StripeOneTimePrice` from a server-owned
allowlist. It validates the current active provider price before creation and
uses `mode=payment`, one card-funded item and a fixed amount/currency. It does
not enable adaptive pricing, discounts or automatic tax. The existing generated
subscription flow remains recurring; do not substitute a one-time price there.

Persist owner/account/mode/attempt/session bindings. Process
`verify_one_time_event` notifications through a durable inbox, then re-read
`read_one_time_receipt` before an atomic entitlement transition. Refund and
dispute hints require reconciliation too. Any partial refund/dispute prevents a
`Paid` receipt; restoration after a resolved dispute needs an explicit reviewed
application policy. Never grant access from a return URL or mock receipt.
This additional one-time contract has protocol tests; the earlier subscription
sandbox evidence does not validate it or the production certificate offer.


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

Stripe's customer-ID portal requires an active default portal configuration in
the selected account and test/live mode, plus permission to create portal
sessions. Configure allowed subscription changes and cancellation explicitly and
test the portal handoff separately. Successful Checkout acceptance does not
validate portal configuration; generated `BILLING.md` records this prerequisite.

The new generated durable real-provider integration is Stripe-specific. Paddle
and Polar expose typed adapter operations; their application persistence and
event orchestration remain explicit. Consult the
[complete provider matrix](https://github.com/Rullst/Rullst/blob/v12.1.1/rullst-capital/README.md#-supported-providers)
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
`v12` commit. Only then may the protected release pipeline publish the sixteen
crates in `.github/release-order.json` and attach verified native CLI binaries.
See [release recovery](release-recovery.md) for partial publication handling.

## Next 12.x minor: Core, Security and Connect review fixes

These unreleased fixes, ported from the v13 review, keep the 12.x API and MSRV.
Existing applications may notice the following behaviour changes:

| Area | Change |
|---|---|
| Server probes | `Server`'s rate limiter and Traffic Shield no longer count or shed exact `GET`/`HEAD /health` and `/ready` requests. |
| Scheduler | A scheduler attached with `Server::schedule` logs each task failure on the `rullst::scheduler` target; only a failed scheduler loop makes `Server::run` return `ServerError::Scheduler`. `Scheduler::task` keeps its `cron`-crate semantics (weekdays 1=Sunday to 7=Saturday, restricted day fields intersect), which are now documented. |
| Validation | HTMX requests (`HX-Request: true`) receive `ValidatedForm`/`ValidatedJson` error fragments with `200 OK` and `X-Rullst-Validation-Status: 400\|422`; other clients keep `400`/`422` JSON. Update HTMX handlers or tests that matched the 4xx status. |
| CSRF | `HEAD` is handled like `GET`: it receives the request `CsrfToken` and, without a CSRF cookie, the same `Set-Cookie`. An unrelated non-ASCII cookie no longer hides the `rullst_csrf` cookie. |
| Body inspection | The Core WAF and PII layers and Security's RASP, schema guard, DLP and AI firewall classify JSON, XML and form media types case-insensitively, including `+json`/`+xml` suffixes and any `application/x-www-form-urlencoded` prefix, so such bodies are now inspected. |
| Queue | The SQLite and Redis drivers fail a job, instead of requeuing it, when its fifth lease stalls. SQLite adds a `stalled_recoveries` column to an existing `rullst_jobs` table on start; `retry_failed_job` resets it. `stalled_after` must exceed the longest `job_timeout` of every worker sharing a queue. |
| Feature flags | `DbFeatureDriver` caches missing flags and failed lookups for its TTL, serves the last value read after a failed refresh, bounds one lookup to two seconds and caches at most 4,096 flag names. |
| Rate limiting | Security's `rate_limit_middleware` keys IPv6 peers per /64 (IPv4-mapped IPv6 as IPv4), so addresses in one /64 share a budget. |
| Honeypot | A trap hit that a page initiated (`Sec-Fetch-Site` `same-origin`/`same-site`/`cross-site`, or `Origin`/`Referer` without fetch metadata) is refused but no longer bans the peer. |
| Log redaction and DLP | `redact_secrets` also redacts compound key names (`DB_PASSWORD`, `access_token`, `client_secret`, `SECRET_KEY`) and whole unquoted `Authorization`/`Cookie` values. DLP also masks EC, DSA, encrypted PKCS#8 and OpenPGP private-key blocks. |
| Connect | `XProvider` authenticates token requests with HTTP Basic. `OidcProvider` uses HTTP Basic when discovery lists `client_secret_basic` without `client_secret_post`. Refresh requests send `Accept: application/json`. `AutoRefreshingSession` keeps a rotated refresh token when a same-user refresh response is rejected. `OidcProvider` accepts profiles without `name`; `ConnectUser::name` may then be empty. |

The ORM fixes of the same minor have their own
[upgrade checklist](crates/orm.md#upgrading-from-121): nested transactions,
`SecretString` serialization, `paginate()`, query-cache and Redis hash keys,
new typed errors, Nexus field hiding and generated Redis effects.

## Next 12.x minor: Mail, Capital, Messaging, AI, IoT, Nexus and Studio review fixes

These unreleased fixes, ported from the v13 review, keep the 12.x API and MSRV.
Existing applications may notice the following behaviour changes:

| Area | Change |
|---|---|
| Mail content and recipients | The pipeline rejects a subject over 2 KiB or an HTML or text body over 2 MiB with `ValidationError`. A recipient may be a bare address, `<address>` or `Name <address>`; the prepared message, suppression lookup and disposable-domain check use the bare address, so transports no longer receive the display name. Lists, groups, comments, quoted local parts and domain literals are rejected. |
| Mail links | The homograph check compares scripts per host label, so single-script IDNs such as `пример.com` and non-Latin query text are accepted. Links are checked after decoding character references and as a browser resolves them (tabs/newlines removed, `\` as `/`, `https:host`, percent-encoded and A-label hosts), so these spellings of a mixed-script host are now rejected. |
| Mail attachments | `AttachmentInspectionGuard` also uses the filename extension and content signature: executable and script-host extensions are rejected under both policies, and the strict policy rejects HTML extensions or markup, script URIs, unknown extensions and declared types it does not inspect (except `application/octet-stream`). SVG/XHTML are recognized by root element and namespace. The SES bearer proxy now sends attachments and inline CID assets (`Simple.Attachments`) within the SES message bounds instead of dropping them. |
| Mail queue and drivers | Queued jobs keep the 12.1 format (attachment bytes as a JSON array of numbers), so 12.1 and 12.2 workers and producers can be upgraded in any order; workers also accept the base64 form that 13.0 producers write. That array still costs about 32 bytes of memory per attachment byte when a job is enqueued and again in the worker, so keep queued attachments small. `FailoverDriver::send_for_tenant` forwards the tenant to every driver it tries. The ACS driver posts to `{endpoint}/emails:send`, and `AzureManagedIdentity` accepts the `http://localhost:<port>` endpoint that Container Apps inject. `PaidInvoiceDelivery` messages still have no sender: set your verified sender on a copy of `message()` and send it with `Mail::send`. |
| Capital Wise | `WiseProvider::parse_webhook_payload` is unauthenticated and now works only with an explicit `mock_*` token (`ConfigurationError` for an empty token, `UnsupportedOperation` for a live one). The live transfer-status read requires a positive decimal transfer ID and a matching response `id`; `bounced_back`/`charged_back` return `UnsupportedOperation` and unknown or missing states a provider error, never `Processing`. |
| Capital webhooks | Paddle's legacy `handle_webhook` accepts only `subscription.*` lifecycle events with `sub_`/`ctm_`/`pri_` identities and Paddle subscription statuses; other signed events return `PayloadParseError`. Razorpay `subscription.completed` maps to `Canceled` instead of being rejected; plan checkout still requests a fixed 12 cycles. On MySQL/MariaDB the SQL replay claim is a plain `INSERT`, so a duplicate hidden by a caller transaction's snapshot returns `WebhookReplay`. |
| Capital MySQL quotas | New MySQL/MariaDB quota tables use `ascii_bin` key columns, so keys that differ only by case stay distinct. `SqlQuotaStore` never alters an existing table: a table created by 12.1 or earlier keeps working as in 12.1, so tenant IDs, features and event keys that differ only by case still share one counter or claim until it is migrated, and each store logs one `tracing` warning (target `rullst_capital::quota`) naming the migration. Running the `ALTER TABLE` migration in the Capital README is recommended; review keys that differ only by case first, because it cannot split rows already merged. |
| Messaging | `OrmOutboxRelay` publishes with the idempotency key `<hex SHA-256 of the stream>/<event_key>`, so streams sharing a topic no longer collide. A claim published but not acknowledged before the upgrade is republished once under the new key; drain in-flight claims first or rely on consumer idempotency. |
| AI | Check-digit-valid CPF and CNPJ numbers are masked before dispatch. Anthropic and Gemini receive every system message, joined in order, instead of only the last. `StatefulChat::send` returns `Generation(BlockedByFirewall)` and stores nothing when the model response would be blocked on replay. New MySQL/MariaDB chat-memory tables use `ascii_bin` keys and every statement compares keys byte-exactly; on a legacy table, IDs differing only by case fail closed until the README migration runs. |
| IoT | Committing an OTA update no longer changes `OtaManager::current_partition`. Constructors still assume `PartitionA`, so platform code must set `current_partition` to the bank its bootloader started before verifying an update. |
| Nexus | Basic Auth counts only presented credentials that fail, per IPv4 address or IPv6 /64, and sets the `rullst_nexus_known_client` cookie after a success. Panel assets are served same-origin from `/nexus/assets/` without inline scripts, styles or handlers, so the default production CSP applies unchanged. Password fields are masked and an empty submission keeps the stored value. The edit form submits only changed fields; an emptied number, relation, date, date-time, enum or JSON field is stored as NULL. |
| Studio | Responses use `Referrer-Policy: same-origin`, and `Origin: null` is accepted only with `Sec-Fetch-Site: same-origin`. A table whose key has a column outside the identifier boundary is read-only, and each row write commits only when exactly one row changed. Under the default `sqlx::Any` build, PostgreSQL table, search, row-action and ER queries work. Without an existing pool, Studio reads only the process `DATABASE_URL` or the parsed `[database].url` and never creates `db.sqlite`. |
