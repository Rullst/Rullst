# Compatibility, MSRV, deprecation, and support policy

This policy applies to the packages in the [release inventory](../../.github/release-order.json):
16 packages in the v12 line and 20 in v13, which adds `rullst-privacy`,
`rullst-supervision`, `rullst-media` and `rullst-labs`. They are released as one
synchronized release train even when a user depends on only one crate. The
current supported-version table in [`SECURITY.md`](../../SECURITY.md) remains
authoritative for releases that are actually available; a version in the
workspace is not a supported release merely because its manifest exists.

## Source branches

The permanent branches are `main` (v13 development and its 13.0.0 prereleases),
`v12` (frozen; 12.x reached end of life on 16 October 2026) and `gh-pages`
(generated site/benchmark data).
Use short-lived feature, fix and Dependabot branches with a pull request to the
appropriate protected base. Remove those temporary branches after their changes
are integrated; a superseded PR must identify the replacement preserving its
changes. The former `v13` integration branch is retired. Historical releases
remain available through immutable tags and crates.io archives.

Select a published version for application dependencies; a branch name is not
a release. Stable and development branches have independent commit histories:
ahead/behind counts are not a measure of security, freshness or missing fixes.
Do not reset or merge whole release trains merely to make those counters zero.

### Carrying fixes between maintained lines

Every stable maintenance fix must be assessed for `main`. Before closing its
maintenance item, record the linked forward-port PR, the commit where the fix
already exists, or a specific reason it does not apply. Carry the regression
and relevant documentation with the fix, adapt it to the destination API and
run that branch's required checks before integration. Conversely, evaluate
confirmed v13 fixes for the existing v12 surface without importing new v13
features or breaking the stable API/MSRV.

Versions, release records, feature inventories and branch-specific documentation
stay appropriate to each release train. Dependency updates require the same
assessment, but resolve and validate each branch's own graph; do not copy a
whole lockfile across diverged workspaces. Prefer a linked, focused port over a
blanket merge. A briefly pending port must stay explicitly tracked rather than
being hidden by commit counts. This policy does not establish an indefinite LTS
commitment or alter the support window below.

## Semantic Versioning contract

Rullst follows Cargo Semantic Versioning for stable releases:

- **Patch (`x.y.Z`)** releases contain compatible bug, security, documentation,
  and dependency fixes. They do not intentionally remove public APIs, public
  Cargo features, CLI commands, or accepted configuration fields.
- **Minor (`x.Y.z`)** releases may add compatible APIs, opt-in features,
  diagnostics, and deprecations. Changes to defaults or generated projects must
  include migration notes and compatibility tests.
- **Major (`X.y.z`)** releases may contain breaking changes. Each known break
  must be listed in the changelog and migration guide.
- **Prereleases** such as `13.0.0-alpha.1` are public evaluation artifacts. Their
  APIs may still change in a later alpha, beta or RC, and users must opt in
  explicitly. Published prerelease archives are immutable, just like stable
  releases; fixes require a new version.

The compatibility surface includes documented public Rust APIs, public Cargo
feature names, CLI command and flag identifiers, supported configuration keys,
serialized public data contracts, and generator output relied on by the
packaged-distribution tests. Undocumented internals, exact dashboard HTML/CSS,
test fixtures, deterministic mocks, and third-party provider behavior are not
stable interfaces. Items hidden from the API documentation with `#[doc(hidden)]`
are internals too, even where Rust lets code name them (for example helpers
that generated code calls).

The public API of each library crate, as docs.rs documents it, is recorded in
[`.github/public-api/`](../../.github/public-api/); a pull request that changes
it updates that listing, so the change is visible in review.

The `cargo-rullst` package is the exception. Its library is the CLI's
implementation, shared by its binaries and tests, and is not a public API: the
stable contract is the CLI's commands, flags, exit codes and versioned `--json`
schemas. It has no listing, and the SemVer workflow reports it as excluded
instead of comparing it.

### Maturity tiers (v13)

From `13.0.0`, each package has a [maturity tier](maturity.md). Core and
Extension packages are supported and follow the contract above for the whole
13.x line. Experimental packages, experimental CLI generators and facade
features that re-export an experimental crate are the exception: their public
APIs, features, schemas and generated output may change between 13.x releases.
Every such break is still listed in the changelog and migration guide. Security
handling is the same for every tier.

## Minimum Supported Rust Version

The v12 release line and v13 both declare **Rust 1.96.0** in every publishable
manifest.

- Patch releases do not raise the MSRV.
- A minor release may raise it only with an explicit changelog entry, updated
  manifests and documentation, and a green MSRV CI job for the release commit.
- A major release may select a new baseline, which must be announced in its
  migration guide.

The MSRV promise covers the supported feature boundaries exercised by CI. A
nightly-only analysis tool or an experimental target does not change the crate
MSRV and must be labelled separately.

## Deprecation and removal

A stable public API scheduled for removal is marked with `#[deprecated]` and
kept for at least one released minor version before removal in the next major
release. Documentation must name the replacement when one exists.

An API that is unsound, enables a security bypass, or cannot be made safe may be
disabled or removed sooner. Such an exception requires a security advisory or
changelog entry describing impact, affected versions, and the supported
migration path. Compatibility never overrides safety or fail-closed behavior.

Removing a public Cargo feature is a major change. Adding an opt-in feature is
normally minor-compatible. Changing default features is treated as an
operationally significant minor change and requires package and generated-app
regression tests.

## Public dependencies

Some third-party crates are part of Rullst's public API: their types appear in
public signatures, trait implementations or re-exports, such as
`rullst::web::axum` and `rullst::db::sqlx`. For the whole 13.x line, each one
stays on the version line below.

| Crate | 13.x line | Where the API exposes it |
| :--- | :--- | :--- |
| `actix-web` | `4` | Capital and Connect Actix integrations (`capital-actix`, `actix` features) |
| `async-trait` | `0.1` | `rullst::async_trait` |
| `aws-sdk-sesv2` | `1` | Mail: `rullst::mail::aws_ses_sdk` and the native `AwsSesDriver` constructors |
| `axum` | `0.8` | `rullst::web::axum`, `Router`, handlers and extractors of Core, Auth, Security, Capital, Connect, Nexus and Studio |
| `base64` | `0.23` | Connect: `From<DecodeError>` for `ConnectError` |
| `chrono` | `0.4` | Core date formatting, `rullst::orm::chrono`, Mail and Capital |
| `dashmap` | `6` | Security: `SecurityStore` fields |
| `http` | `1` | `rullst::http` and the header, method and status types of the web crates |
| `jsonwebtoken` | `11` | Connect: `From<jsonwebtoken::errors::Error>` for `ConnectError` |
| `mongodb` | `3` | ORM `MongoDbStore` (`orm-mongodb`) |
| `opentelemetry_sdk` | `0.33` | Core `DistributedTelemetry::layer` (`telemetry`) |
| `pgvector` | `0.4` | `rullst::orm::Vector` (`orm-pgvector`) |
| `redis` | `1` | ORM: `From<RedisError>` for `Error` |
| `reqwest` | `0.13` | AI and Connect error variants and conversions |
| `secrecy` | `0.10` | Connect tokens (`SecretString`, `prelude::ExposeSecret`) |
| `serde` | `1` | `Serialize`/`Deserialize` implementations in every crate |
| `serde_json` | `1` | `serde_json::Value` arguments and results (Core queue, AI, ORM, Connect, Security, IoT) |
| `sqlx` | `0.9` | `rullst::db::sqlx`, `rullst::orm::sqlx`, `FromRow`, `RullstPool` and ORM signatures |
| `sqlx-core` | `0.9` | The same SQLx types, through the ORM's direct dependency |
| `tokio` | `1` | `rullst::runtime`, `rullst::async_runtime::tokio`, channel receivers |
| `tower` | `0.5` | `rullst::web::tower` |
| `tower-http` | `0.7` | `rullst::web::tower_http` |
| `tower-layer` | `0.3` | `Layer` bounds of `Router` and middleware |
| `tower-service` | `0.3` | `Service` bounds and implementations of middleware |
| `tower-sessions` | `0.15` | Connect: `begin_oauth_session` and the session extractors |
| `tracing` | `0.1` | Core telemetry (`TraceParent`) |
| `tracing-core` | `0.1` | Core `RedactPersonalDataLayer` |
| `tracing-opentelemetry` | `0.34` | Core `DistributedTelemetry::layer` (`telemetry`) |
| `tracing-subscriber` | `0.3` | Core `RedactPersonalDataLayer` implements its `Layer` |
| `url` | `2` | AI `ValidatedEgressUrl::into_url` and Connect |
| `utoipa` | `6` | Studio: `Studio::with_openapi` |
| `validator` | `0.21` | `rullst::Validate`, `ValidatedForm` and `ValidatedJson` |
| `zeroize` | `1` | `Zeroizing` values returned by Auth, Labs and Security |

Types that one of these crates takes from its own dependencies (for example
`axum-core`, `serde_core`, the SQLx drivers, `tokio-macros` or `actix-http`)
follow the line of the crate listed.

- A semver-compatible update (a patch, or a compatible minor of a crate at 1.0
  or later) may ship in any 13.x release.
- A semver-incompatible update, for example axum 0.8 to 0.9, ships only in a
  Rullst major release (14.0).
- **Security exception.** If a vulnerability is fixed only in an incompatible
  upstream release and the fix cannot be backported, a 13.x minor release may
  take that update. The CHANGELOG marks it as a security exception, with
  migration notes.
- Private dependencies, which do not appear in the public API, may change in
  any release.
- Over time Rullst prefers its own types for the commonly used surface.
  `rullst::web::axum` remains the documented escape hatch to the underlying
  framework and follows the rules above.

[`.github/public-dependencies.json`](../../.github/public-dependencies.json)
records the same lines. CI fails when a release library's requirement or locked
version leaves its line; a security exception changes that file and this table
in the same pull request.

## Supported release window

Until 13.0.0 is published, only the most recent 13.0.0 prerelease is supported
and fixes ship in a new prerelease. From 13.0.0:

- **Patch releases** (13.0.x, later the patches of the newest 13.x minor) carry
  bug and security fixes, at any time.
- **Minor releases** (13.1, 13.2, …) carry backward-compatible features,
  performance improvements, new opt-in features and compatible dependency
  updates.
- **14.0** carries only breaking changes.
- 13.x stays the active line for at least 12 months after 13.0.0. Meanwhile
  breaking ideas go to the [v14 candidates](../../ROADMAP.md#v14-candidates)
  list, and a 13.x release may mark a future change with `#[deprecated]`.
- v14 development starts only when breaking changes justify it, and never
  before that minimum.
- After 14.0.0 is published, 13.x receives security fixes for 12 more months.

There is no LTS or multi-minor backport program: fixes ship in the newest
minor. The exact versions receiving security triage are listed in
`SECURITY.md`; older versions are end-of-life unless that table explicitly says
otherwise.

Release candidates receive fixes through a subsequent RC number rather than by
overwriting a published artifact. Security reports follow the coordinated
disclosure and response targets in `SECURITY.md` and the
[advisory-exception policy](security-advisory-exceptions.md).

## Release evidence

Before publishing a stable or prerelease version, the release commit must prove
the applicable compatibility contract with:

1. synchronized package versions and exact (`=`) internal requirements, so
   Cargo cannot pair packages from different releases;
2. the MSRV and multi-OS CI jobs;
3. full tests, strict Clippy, formatting, docs, and feature-boundary checks;
4. packaged crates and generated applications tested without monorepo paths;
5. a changelog and migration notes for every intentional behavior change.

Passing these gates is evidence for the exact commit and declared matrix, not a
guarantee about untested platforms, external providers, or downstream
application code.
