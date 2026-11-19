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

Since 13.0.0-alpha.6 every public enum, and every public struct whose fields
are all public, is either `#[non_exhaustive]`, so that a minor release can add
a variant or a field, or documented as "Exhaustive on purpose" with the
reason: an authorization or decision enum whose next variant must break
callers until they handle it, a closed set, a versioned wire format or a
newtype. Adding a variant or a field to an exhaustive type is a major change.
The listing script fails on a listed type that is neither. Unit structs (such
as `CspSecurityLayer` or `HtmlSanitizer`) stay outside this rule, because
their unit expression is how applications build them: adding a field to a
public unit struct is a v14 change.

Files that Rullst writes and reads back on a developer's machine follow the
same contract. Since 13.0.0-alpha.6 the standalone Studio's project list
(`projects.json`) is forward compatible within 13.x: a 13.x Studio reads the
list that any later 13.x Studio wrote, keeps the fields it does not know and
writes them back unchanged. A later 13.x release only adds fields (within the
documented bounds); it does not change the type of an existing field, the
values an older Studio accepts in it or the meaning of a value, and adds a
new field for that instead. The proxy setting is the exception, because it is
checked when used: an older Studio keeps a proxy mode or URL it cannot apply
and connects to nothing until another is chosen. The file's `version` counts
incompatible changes: only a major release may raise it, and a Studio refuses
a higher version, without changing the file, with a message that names the
Studio that reads it. Studios before 13.0.0-alpha.6 refuse unknown fields, so
the rule holds from 13.0.0-alpha.6 on
([details](production-monitoring.md#studio-versions-that-share-the-project-list)).

The `cargo-rullst` package is the exception. Its library is the CLI's
implementation, shared by its binaries and tests, and is not a public API: the
stable contract is the CLI's commands, flags, exit codes and versioned `--json`
schemas. It has no listing, and the SemVer workflow reports it as excluded
instead of comparing it.

### Maturity tiers (v13)

From `13.0.0`, each package has a [maturity tier](maturity.md), Core or
Extension. Both are supported and follow the contract above for the whole 13.x
line, and so does every part of a package, its opt-in adapters and Cargo
features included. There is no Experimental exception: the tier that allowed
breaking changes within 13.x was dropped on 19 November 2026
([maturity tiers](maturity.md#no-experimental-tier-since-19-november-2026)).
Security handling is the same for every tier.

## Minimum Supported Rust Version

The v12 release line and v13 both declare **Rust 1.96.0** in every publishable
manifest. The MSRV is the oldest compiler Rullst supports; the recommended
toolchain is the latest stable Rust (`rustup update stable`). The repository
itself is built with the compiler pinned in `rust-toolchain.toml`.

- Patch releases do not raise the MSRV.
- In 13.x, a minor release raises the MSRV only when a dependency, a security
  fix or a concrete need (a language or standard library feature the release
  uses) requires it, and only to a Rust release published at least six months
  before that minor release. Rust 1.100.0 was published on 12 November 2026,
  for example, so no 13.x minor release published before 12 May 2027 can
  require it.
- A minor release that raises it does so with an explicit changelog entry,
  updated manifests and documentation, and a green MSRV CI job for the release
  commit.
- A major release may select a new baseline, which must be announced in its
  migration guide.

The MSRV promise covers the supported feature boundaries exercised by CI: the
required `Check MSRV (Rust 1.96.0)` job compiles the whole workspace with every
feature on Rust 1.96.0, and the informational
[MSRV Tests](https://github.com/Rullst/Rullst/blob/main/.github/workflows/msrv-tests.yml)
workflow runs the unit and integration tests of Core, the ORM on SQLite,
Security, Auth, the macro crates and the `rullst` facade on it every night. A
nightly-only analysis tool or an experimental target does not change the crate
MSRV and must be labelled separately.

## Deprecation and removal

A stable public API scheduled for removal is marked with `#[deprecated]` and
kept for at least one released minor version before removal in the next major
release. Documentation must name the replacement when one exists.

13.0 is the major release that removes the 12.x deprecations. Every API
still marked `#[deprecated]`, whether it was deprecated in 12.x or during the
13.0 prereleases (`rullst_orm::schema_diff`, deprecated in 13.0.0-alpha.5),
was removed in 13.0.0-alpha.6, the 13.0 feature and public API freeze, so
13.x does not carry it. Each removal has a `cargo rullst upgrade` rule and a
row in the [migration guide](migration-v13.md) that names the replacement.
The period above applies to the deprecations made from 13.0.0 on.

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
stays on the version line below. Before the 13.0 freeze, planned for
13.0.0-alpha.6, dependencies move to their current major so that 13.x starts
from current lines: 13.0.0-alpha.6 moves ed25519-dalek to 3, jsonwebtoken to
12 and jsonschema to 0.61, which the public API does not expose. The same
release takes three pre-1.0 crates that break every few months out of the
public API, so that 13.x releases can update them as private dependencies:
`opentelemetry_sdk` and `tracing-opentelemetry`
(`DistributedTelemetry::layer` returns an `impl tracing_subscriber::Layer`)
and `utoipa` (`Studio::with_openapi` takes any `serde::Serialize` document).
It also replaces `chrono` 0.4, which
[RUSTSEC-2026-0543](security-advisory-exceptions.md) marks unmaintained, with
`jiff` 1 ([moving off `chrono`](migration-v13.md#moving-off-chrono)).

| Crate | 13.x line | Where the API exposes it |
| :--- | :--- | :--- |
| `actix-web` | `4` | Capital and Connect Actix integrations (`capital-actix`, `actix` features) |
| `async-trait` | `0.1` | `rullst::async_trait` |
| `aws-sdk-sesv2` | `1` | Mail: `rullst::mail::aws_ses_sdk` and the native `AwsSesDriver` constructors |
| `axum` | `0.8` | `rullst::web::axum`, `Router`, handlers and extractors of Core, Auth, Security, Capital, Connect, Nexus and Studio |
| `http` | `1` | `rullst::http` and the header, method and status types of the web crates |
| `jiff` | `1` | `rullst::orm::jiff` and the ORM date/time types (`rullst::orm::Timestamp`, `DateTime`, `Date`, `Time`), which dereference to and convert from jiff's; Core, Mail and Capital date and time values |
| `pgvector` | `0.4` | `rullst::orm::Vector` (`orm-pgvector`) |
| `redis` | `1` | ORM: `Orm::redis_client` and `Orm::redis_manager` (`redis` feature), deliberately: they hand the application the ORM's own Redis client and connection manager |
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
| `tower-sessions` | `0.15` | Connect's opt-in `axum-session` feature: `begin_oauth_session`, `begin_oidc_session` and the `AuthSession`/`AuthSessionForm` extractors, which read the `Session` that the application's `SessionManagerLayer` inserts (`rullst_connect::challenge` needs no session layer) |
| `tracing` | `0.1` | Core telemetry (`TraceParent`) |
| `tracing-core` | `0.1` | Core `RedactPersonalDataLayer` |
| `tracing-subscriber` | `0.3` | Core `RedactPersonalDataLayer` implements its `Layer`; `DistributedTelemetry::layer` (`telemetry`) returns an `impl Layer` |
| `url` | `2` | AI `ValidatedEgressUrl::into_url` and Connect |
| `validator` | `0.21` | `rullst::Validate` (the trait and its derive) and the bound of `ValidatedForm` and `ValidatedJson`; since 13.0.0-alpha.6 `ValidationError` carries Rullst's `FieldErrors` instead of `validator::ValidationErrors` |
| `zeroize` | `1` | `Zeroizing` values returned by Auth, Labs and Security |

Since 13.0.0-alpha.5 no public error variant or `From` conversion names a
third-party error type: the AI, Connect, Core, IoT and ORM errors carry
Rullst types or plain text, with the library error kept in their `source()`
chain. Security's collections are private behind methods (`SecurityStore`,
`DeceptionRoutes`), and Connect's JWKS cache returns its own
`JsonWebKeySet`. `reqwest`, `base64`, `dashmap` and `jsonwebtoken` are
therefore no longer public dependencies; `redis` stays one on purpose, and
`serde_json` only through `Value` arguments and results.

Since 13.0.0-alpha.6 `DistributedTelemetry::layer` returns an opaque
`impl tracing_subscriber::Layer<S> + Send + Sync + 'static`, Studio's
`with_openapi` and `api_playground::router` take any `serde::Serialize`
document (the `OpenApi` of any utoipa version, or a `serde_json::Value`),
and `ValidationError::ValidationError` carries `FieldErrors`, whose
`messages()` and `codes()` return Rullst types. `opentelemetry_sdk`,
`tracing-opentelemetry` and `utoipa` are therefore private dependencies.
`validator` stays public with a smaller surface: `#[derive(Validate)]`
expands to `::validator::…` paths in the application, so the trait the
extractors require must be the application's `validator` 0.21; replacing
it would take a Rullst derive macro. `tower-sessions` also stays public,
for a reason a trait cannot remove: the extractors read the `Session` that
the application's own `SessionManagerLayer` puts in the request, which is
found only when both use the same `tower-sessions` line. The `challenge`
module is Connect's flow without it.

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
- **Newer major side by side.** A 13.x minor release may also support a
  newer, incompatible major of a public dependency behind a new opt-in Cargo
  feature, next to the 13.x line, when both can be compiled into one
  application (for example a second `axum-session` feature for a newer
  `tower-sessions`, with its own extractors). The default features stay on
  the 13.x line in the table above, and the opt-in feature is documented
  with the line it follows; the newer line becomes the default only in a
  major release.
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
- The maintainer intends to keep 13.x the active line for 12 months after
  13.0.0. Meanwhile breaking ideas go to the
  [major-version candidates](../../ROADMAP.md#if-a-major-version-is-ever-needed) list, and a 13.x release
  may mark a future change with `#[deprecated]`.
- v14 development starts only when breaking changes justify it, and is not
  planned before those 12 months end.
- After 14.0.0 is published, the maintainer intends to give 13.x security
  fixes for 12 more months.

These periods are an intention, not a guarantee, and they may change for
reasons nobody can foresee today. Any change will be announced in advance in
the CHANGELOG, the README and on GitHub.

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
