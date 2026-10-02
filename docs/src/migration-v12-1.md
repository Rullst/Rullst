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

### CLI and generated projects

The CLI fixes of the same minor change what some commands write or accept.
Applications generated by 12.1 keep building unchanged; regenerate or copy a
fixed starter file only after reviewing its diff.

| Area | Change |
|---|---|
| Model scaffolds | `make:model --migration` and `make:resource` skip the create-table migration when the model file or a `*_create_<table>.rs`/`*_create_<table>_table.rs` migration already exists, because its rollback would drop the live table; add schema changes with `make:migration`. |
| Auto migrations | `make:migration:auto` declares columns from the field's Rust type (`integer`, `big_integer`, `float`, `boolean`, `string`) and `NOT NULL` for non-`Option` fields instead of `TEXT` for every field. A required column added to an existing table gets a typed `NOT NULL DEFAULT`; unsupported types, and required date, encrypted or JSON columns added to a populated table, are refused before anything is written. |
| Database-first models | `generate:models`/`make:models-from-db` refuse to replace an existing `<table>.rs` and append missing `pub mod` declarations to an existing `mod.rs` instead of rewriting it; move colliding files aside or use another `--output`. |
| Authentication scaffold | `cargo rullst auth` fails before writing anything when `src/models/user.rs`, the auth controller, middleware or pages already exist, or when a `*_create_users.rs`/`*_create_users_table.rs` migration already creates the users table (the blank database starter and the SaaS/LMS blueprints ship one). Its second users migration previously failed `db:migrate` and its rollback dropped `users`; add the account columns with `make:migration` instead. It also rejects Turso-primary projects, enables the `orm` and `auth` umbrella features and registers the `controllers`, `middlewares`, `models` and `pages` modules in `src/lib.rs` (or `src/main.rs`). |
| Second factor scaffold | `make:mfa` no longer verifies a code against a secret sent by the client. It generates a `user_mfa_factors` migration and handlers bound to the signed-in `Extension<i32>` user id that store the secret server-side and accept each TOTP step once; it enables the `orm` and `security` features, registers the module, rejects Turso-primary projects and refuses to overwrite `src/controllers/mfa.rs`. Replace controllers generated by 12.1, whose `mfa_verify` accepted any self-chosen secret. |
| Kubernetes manifests | `make:k8s` fails before writing when any `k8s/` manifest exists and does not write through a symlinked `k8s/` directory or file; move customized manifests aside to regenerate the templates. |
| Packaging files | `dockerize`, `nixify` and `generate:buildah` refuse to replace an existing `Dockerfile`, `flake.nix`, `.envrc` or `build_buildah.sh`, and `dockerize` refuses a symlinked `.dockerignore`; move a customized file aside to regenerate its template. |
| Deploy exit status | `deploy --platform fly` and `--platform railway` exit non-zero when the installed `flyctl deploy` or `railway up` fails, instead of printing a hint and exiting 0; a missing provider CLI is still advisory. Update scripts that relied on the old exit status. |
| Project update verification | `update project verify` resolves the candidate with `cargo update --workspace` instead of `cargo generate-lockfile`, so unrelated `Cargo.lock` pins are kept. A verification recorded by the 12.1 CLI no longer passes `review`/`apply` ("verification command or status changed"); run `verify` again. |
| Project update toolchain | Cargo/rustc commands that `update project prepare`, `verify`, `review` and `apply` run in a project copy pin `RUSTUP_TOOLCHAIN` to the caller's toolchain (the inherited value, otherwise rustup's default) and ignore the project's `rust-toolchain(.toml)`, whose `path` toolchain could run project binaries before consent. Set `RUSTUP_TOOLCHAIN` to verify with a project-pinned channel. |
| Upgrade downgrades | `cargo rullst upgrade` fails before writing when its target (the installed CLI version or `--to`) is older than a managed requirement's lower bound or a Rullst package locked in `Cargo.lock`, instead of pinning the project to the older release; install a CLI that is not older than the project. |
| IDOR audit | `audit --idor` (and the `--network` listener check) skips only each top-level `#[cfg(test)]` item instead of everything after the first one, so routes declared after an early `#[cfg(test)] mod tests;` are now scanned and may need a `// rullst-access:` classification. |
| Foundry service account | `foundry:deploy` provisions a dedicated `rullst-<app>` system account (it now requires `useradd`) and runs the application under a sandboxed unit (`User=`, `NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome`, `PrivateTmp`) instead of root. Existing `/opt/rullst/<app>/data` is re-owned to that account on the next deploy, so stored data stays readable; the application can write only there. Before redeploying a 12.1 deployment, move state it writes elsewhere (for example an absolute SQLite path or an upload directory outside `data/`) under `/opt/rullst/<app>/data` and update `[env]`. |
| Foundry port | `foundry:deploy` writes `PORT` (the `[app] port`, default 3000, that Caddy proxies to and the health check probes) into the service environment unless `[env]` sets it, uses `[env] PORT` when `[app] port` is omitted, and rejects an `[env] PORT` that differs from `[app] port`, which previously deployed into a 502. |
| Starter seeds | New Blog, ERP, Portfolio and LMS projects seed rows without SQLite-only `datetime('now')`, which failed `db:migrate` on PostgreSQL, MySQL and MariaDB, and on PostgreSQL their migrations advance each explicitly seeded table's `SERIAL` sequence. A PostgreSQL project generated by 12.1 whose first inserts fail with duplicate primary keys can run `SELECT setval(pg_get_serial_sequence('<table>', 'id'), (SELECT MAX(id) FROM <table>))` once per seeded table. |
| MySQL indexes | New SaaS projects, the LMS `auth`/`auth,learning` users and learning-access migrations, `cargo rullst auth` and SQLx `make:billing` declare their indexed email, status, event-key and subscription-id columns as bounded `VARCHAR` instead of `TEXT`, which MySQL/MariaDB cannot index without a prefix length; the learning audit index orders by `id` instead of `created_at`. The complete LMS academy migrations still index `TEXT` columns on MySQL/MariaDB. |
| ERP dashboard | New ERP projects serve the dashboard at `/` (customer names, order totals and revenue) behind the same Nexus administrator policy as the inventory mutations: loopback-only in debug builds and Basic Auth with the `NEXUS_ADMIN_*` credentials behind verified TLS in release builds. ERP projects generated by 12.1 serve it publicly in release builds; move `get("/" => controllers::erp_controller::index)` into `admin_routes` in `src/main.rs` (or `src/lib.rs`). |
| LMS lesson progress | The detached `auth,learning` LMS profiles render a fresh progress key per lesson-player view and scope it to the requested percentage, so a later 50%/100% save no longer returns 409 after the first save. The complete LMS profile already did this; a detached project generated by 12.1 can copy `new_progress_key`/`progress_event_key` from a new one into `src/controllers/learning_controller.rs`. |
| Password work | Generated auth controllers (`cargo rullst auth`, SaaS and LMS) run Argon2id through a four-permit semaphore instead of the unbounded `*_password_async` helpers, so a burst of logins cannot hold hundreds of 19 MiB hashes at once; a submission that waits two seconds without capacity receives 503. Tune `MAX_CONCURRENT_PASSWORD_WORK` in `src/controllers/auth_controller.rs`. |
| Blog and Portfolio styling | New Blog and Portfolio projects bind their inline styles to the request's CSP nonce, replace `style` attributes with classes, use system fonts instead of Google Fonts and default the avatar to `/static/rullst.png`, so pages keep their styling under the production security headers. The generated `pages::*::render`/`index_page`/`detail_page` functions take a `csp_nonce: &str` argument. |
| Host linker config | New projects' `.gitignore` and `.dockerignore` exclude the generated `.cargo/config.toml`, which selects the generating host's `mold`/`lld`; container builds and CI clones no longer inherit a linker they lack. A 12.1 project can add `/.cargo/config.toml` to `.gitignore` and `.cargo/config.toml` to `.dockerignore` if that file only holds the generated linker settings. |
