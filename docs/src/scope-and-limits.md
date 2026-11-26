# Scope and Limits

> 🇧🇷 [Leia esta página em português](pt-BR/scope-and-limits.md).

This page lists what Rullst does not do by design, and what its own tests
cannot establish because it depends on external services, hardware or
upstream releases. It holds for every 13.x release until a release changes
it, and that release's changelog says so. How the project is verified, and
what that verification does not prove, is on
[How Rullst is verified](how-rullst-is-verified.md). Limitations that belong to one
release (upgrading from the previous prerelease, figures measured on that
release's source, work scheduled for the next one) are in that release's
"Known limitations" in the [changelog](../../CHANGELOG.md).

## Project

- Rullst has a sole maintainer, with a successor arrangement in
  [`GOVERNANCE.md`](../../GOVERNANCE.md).
- The v12 line reached end of life on 16 October 2026; 12.3.0 is its last
  release. crates.io has yanked the `zip` releases that 12.x's Studio needs, so
  a new 12.x project no longer resolves its dependencies, and the documented
  v5 to 12.x upgrade path may not resolve either; use a 13.x release.

## Dependencies and advisories

- `RUSTSEC-2026-0459` (bundled SQLite 3.53 in `libsqlite3-sys` 0.37.0, through
  `sqlx-sqlite` 0.9.0 and every Rullst SQLite feature; the fix is
  `libsqlite3-sys` 0.39, which no `sqlx-sqlite` release accepts yet) and
  `RUSTSEC-2026-0440` (`astral-tokio-tar`, unmaintained, reached only through
  the integration tests' `testcontainers` 0.27.3 dev-dependency; 0.28 still
  depends on it) keep their time-limited exceptions until 2027-01-28 in
  [Security Advisory Exceptions](security-advisory-exceptions.md). Do not open
  SQLite files that a less trusted party can write. The CI workflow that
  `cargo rullst new` writes ignores `RUSTSEC-2026-0459` in its `cargo audit`
  step until the same date.

- For all of 13.x, the 25 public dependencies listed in the
  [compatibility policy](compatibility-policy.md#public-dependencies) stay on
  their version lines. An incompatible update of one ships only in 14.0, as a
  security exception, or behind a new opt-in feature next to the 13.x line.
  Pre-1.0 ones among them, such as `axum` 0.8, `sqlx` 0.9, `tower` 0.5,
  `tower-http` 0.7, `tower-sessions` 0.15 (Connect's `axum-session`),
  `validator` 0.21, `pgvector` 0.4 and `secrecy` 0.10, may fall behind
  upstream during 13.x.

## Upgrades

- `cargo rullst upgrade` changes the Rullst versions, applies the compiler's
  suggestions (`cargo fix`) and checks that the project still builds; it
  rewrites no other application code. Its rules locate what they can find by
  name in the application's source, manifests, scripts and project files. A
  change that shows only at run time (a value built at run time or read from
  configuration, a response type set by middleware, input that arrives in
  requests) is not located, and the command cannot prove that a database
  upgrade, an authorization rule, a provider integration or a deployment
  still behaves correctly
  ([what the command can guarantee](tutorials/36-assisted-framework-upgrades.md#what-the-command-can-guarantee)).
  The compiler, the migration guide's checklist for each release and the
  application's own tests remain the reference.

## Databases and the ORM

- Turso/libSQL support is deliberately bounded: it is a primary database only
  in the Blank and API starters (any starter can add it as a side store with
  `--turso`), through `TursoOrm` and the `TursoModel` contract that
  `#[derive(Orm)]` with `#[orm(backend = "turso")]`, or `#[derive(TursoModel)]`,
  generates; such a model must declare `#[orm(full_row_save)]`. It does not
  work with `Schema`, `artisan!`, `Server::with_db`, Nexus, Studio's database
  pages or `make:billing`, and its transactions are statement batches
  (`TursoStore::transaction`). Full ORM parity is not planned (see the ROADMAP's
  [decided non-goals](../../ROADMAP.md#13x-minor-candidates-and-decided-non-goals)).
  The client follows a same-origin `base_url` from a multi-instance server
  (such as Turso Cloud); a `base_url` at another origin is refused with
  `PolyglotError::CrossOriginEndpoint` and the insert is not committed.
  An empty or `mock_` URL makes `TursoStore::connect` and
  `TursoOrm::init_from_env` use a local SQLite file (`TURSO_OFFLINE_PATH`, by
  default `turso-development.db`) in every environment, production included,
  because `rullst-orm` does not read the runtime environment: an application
  that calls them directly must refuse an empty URL in production itself.
  Since 13.0.0-rc.1 the generated starter does: in staging and production it
  stops at start-up when `TURSO_DATABASE_URL` is empty or unset, and only an
  explicit `mock_` value selects the local file.
- `save()` writes only the changed columns for a model with a `RowSnapshot`
  field. A model with `#[orm(full_row_save)]`, and every Turso model, writes
  every column, so two requests that change different columns of one row can
  still lose one of the changes.
- Native date/time `Schema` columns need a strict driver feature
  (`strict-postgres`, `strict-mysql`, `strict-sqlite`). On the SQLx `Any` pool
  `timestamps()` keeps `TEXT` columns.
- The ORM's date and time types (`rullst::orm::Timestamp`, `DateTime`, `Date`
  and `Time`) wrap `jiff` 1. A `Timestamp` holds instants from
  `-9999-01-02T01:59:59Z` to `9999-12-30T22:00:00.999999999Z`, so an instant
  stored outside that range, such as the end-of-time sentinel
  `9999-12-31 23:59:59` in a `timestamptz` or MySQL `DATETIME` column, is a
  decode error that names the value. Store NULL in an `Option<Timestamp>` for
  "no end", or use a civil `DateTime` column, which keeps the full range.
  Leap seconds are not represented: a stored `:60` reads as `:59`.
- In WAL mode (`sqlite_journal_mode = "wal"`) the database file alone is no
  longer a backup (use `sqlite3 ".backup"`), every process must run on one
  host, and WAL must not be used on a network filesystem (NFS, SMB, Azure
  Files: set `"delete"` there); `cargo rullst doctor` warns about a WAL
  database on such a mount.
- Startup migrations are opt-in (`RULLST_AUTO_MIGRATE=true`).
- The durable audit trails of `rullst-ai` (`DurableToolAuditTrail`,
  `DurableRagAuditTrail`) are single-process local writers, not authenticated
  or distributed audit services: their SHA-256 frames detect corruption but
  do not authenticate events. A multi-instance application implements
  `ToolAuditSink` over a durable destination of its own
  ([guarded local AI tools](ai-tool-security.md#durable-local-evidence)).

## Requests and pages

- Generated deployments compress pages because the CSRF token is masked per
  response. Masking covers that token only: an application should not put
  other secrets in a compressed page next to text taken from the request.
- Multipart: the CSRF `_token` field must come before the file fields, within
  the first 64 KiB (or send the `X-CSRF-Token` header). Uploaded files are not
  scanned for malware or stripped of metadata, and the WAF inspects the text
  fields only, not file contents or names ([form uploads](form-uploads.md)).
- `/docs` (Scalar) loads one pinned Scalar release, 1.67.0, with its integrity
  hash, and its policy allows the style elements that release inserts by
  their hashes, so a later Scalar version needs a Rullst release with new
  hashes. The page uses system fonts and has no "Ask AI" agent: both are
  turned off.

## Packages with a bounded scope

The packages promoted to the Extension tier keep their documented scope
([maturity tiers](maturity.md)).

- Labs never executes learner code: the application owns the runner and its
  isolation ([runner contract](labs-runner-contract.md)). A Docker container
  per job is not enough isolation for hostile code; the contract lists the
  minimum (a separate host or VM, no network, resource limits, syscall and
  filesystem restrictions).
- Supervision ships no media model: camera-presence and audio-activity
  adapters are the application's, and the browser sends statistics, not
  frames or audio. A parental `CoursePolicy` is one absolute UTC window of at
  most 30 days; recurring allowed hours are application code.
- Privacy has no live age provider or verified guardianship.
- IoT was validated on a micro:bit V2, and its `std` build in a gateway fed by
  Arduino Uno boards. MQTT and CoAP transports, TLS/DTLS and brokers, other
  boards, a persistent rollback-counter store, flashing, bootloader handoff
  and power loss are not validated.
- Media's Bunny Stream adapter is used with a real library by a production
  application, but transcoding and CDN edge cases are not covered by Rullst's
  own tests, which run offline. Bunny's just-in-time status 7 stays
  `Processing` (only status 8 is `Playable`), and a `Playable` video plays
  only in Bunny's own player. The S3/R2 adapter is tested offline only; no
  automated test runs against a live S3, R2 or MinIO service.
- The InfinitePay adapter covers one-off payment links and source-confirmed
  notifications, validated with one real Pix payment in the 13.0.0-alpha.5
  acceptance run. Subscriptions, the portal, usage, coupons and trials return
  `UnsupportedOperation`, and the merchant must enable external checkout
  ("Checkout externo") in the InfinitePay app first.

## External accounts and providers

- No automated test runs against a live provider account: CI exercises the
  payment, video, OAuth, mail and AI providers against local mocks, fixtures
  and test servers, without credentials.
- Google and GitHub sign-in are used by the maintainer's production
  applications, with no automated test against a live account; Microsoft,
  Apple and the other OAuth providers have not been used with real accounts. Microsoft sign-in needs the optional claims
  `email` and `xms_edov` to link or create accounts. `make:oauth` needs the
  account files of `cargo rullst auth` or the SaaS starter, or the functions
  of the documented authentication contract (`src/auth_contract.rs`).
- PWA: the generated icons are placeholders. The Google Play (Bubblewrap) and
  Microsoft Store (PWABuilder) flows are documented, not validated with real
  store submissions; no App Store package is generated.
- `cargo rullst deploy`: CI builds the generated Dockerfile and starts the
  generated `compose.yaml` and `Caddyfile` for the Blank and SaaS starters with
  Docker and rootless Podman on a GitHub Ubuntu runner, checking HTTPS through
  Caddy with its local certificate authority, compressed pages with the masked
  CSRF token, Server-Sent Events through the compression, the container
  hardening, migrations, the SQLite volume in WAL mode across a restart and
  the backup and restore commands. Real domains, DNS and Let's Encrypt,
  provider accounts and firewalls, Arm machines, reboots and PostgreSQL/MySQL
  deployments are not tested in CI (the maintainer's production applications,
  Academy, capoeira and vene.win, run on real domains on one Oracle Ampere A1
  Arm machine behind a shared Caddy with its Let's Encrypt certificates), and on Ubuntu 26.04 `podman compose restart`
  can leave Caddy stopped (use `podman compose down` and `up -d`). The
  provider guides and the Advanced deployment page were written from the
  providers' and tools' documentation, not followed end to end on real
  accounts.

## CLI and Studio

- `cargo rullst auth` on a Blank or API starter needs exactly one strict
  database profile (a project with `drivers-all` is refused) and adds
  migrations that extend the starter's `users` table and create the
  `rullst_auth_sessions` table. Tests run it on SQLite with existing rows; on
  PostgreSQL 15, MySQL 8.4 and MariaDB 11.4 CI runs it against a server on
  fresh Blank and API starters that hold two rows and checks with SQL the
  added columns with their types and defaults, the unique
  `users_email_unique` index, the placeholder emails and the new tables, then
  a sign-up over HTTP that registers a session and `db:rollback`; other
  server versions and a real deployment's data are not tested. Back up the
  database and try it on a copy first.
- `cargo rullst footprint`'s carbon figure is an SCI estimate from the
  `--cpu-watts`, `--grid-intensity` and `--network-kwh-per-gb` figures you
  give, not a measurement; an image size is measured only through a local
  Docker or Podman socket; and the page weight fetches same-origin resources
  only, listing third-party origins without fetching them, without cookies
  or a sign-in. A published `https://` site gets the page weight only, and an
  application found with `--url` has no start-up time.
- Production monitor: the figures are per process and reset on restart; the
  starters do not enable it. The standalone Studio's desktop notifications
  appear only while a Studio page is open in a browser (a background tab is
  enough, which the browser may then check about once a minute), and a page
  opened later does not show earlier ones. A proxy that needs a password can
  only come from the environment (`HTTPS_PROXY`, `HTTP_PROXY` or
  `ALL_PROXY`), and SOCKS proxies are not supported: a SOCKS value is refused
  and blocks every connection. Tokens are protected against a leaked
  configuration file, not against malware running as the developer
  ([production monitoring](production-monitoring.md#through-a-proxy)).

## Locales and documentation

- i18n formatting has CLDR data for the documented 27 locales; a language
  without data uses English (`en`) data. Plural rules cover 18 languages, and
  a plural message in another language fails the build. Currencies other than
  the 15 with a constant (BRL, USD, EUR, GBP, JPY, ARS, MXN, CLP, COP, PEN,
  UYU, CAD, AUD, CHF and CNY) are written with their ISO code. A new locale
  file needs the documented `build.rs` line ([translations and locales](i18n.md)).
- CI runs the commands of the first-hour pages only (the README quickstart,
  getting started, tutorials 01, 02, 03, 05 and 11 and the REST API
  quickstart), without PostgreSQL or MySQL servers or external accounts.
  Other pages' commands are not run, and their Rust blocks marked
  `rust,ignore` are not compiled. Only these pages, this one and
  [How Rullst is verified](how-rullst-is-verified.md) are
  translated into Brazilian Portuguese; CI checks that their code blocks
  equal the English ones byte for byte, but only prints a notice when the
  English prose changed after the commit a translation names, so a
  translation can lag behind (the English page prevails).

## Release evidence

- Release fuzz evidence: a change to a book page that the facade's doctests
  include reruns the facade's 12 fuzz targets, because the build-graph rule
  counts included files as inputs ([fuzz evidence reuse](fuzz-evidence.md)).
