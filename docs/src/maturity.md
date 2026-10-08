# Maturity tiers

**Decision recorded on 8 October 2026 for the v13 line, before
`13.0.0-alpha.1`.** Every Rullst package, and every CLI generator whose result
depends on an outside toolchain or platform, has one of three maturity tiers.
A tier tells you how stable a package is and how much of it has been validated.
It does not tell you whether a feature exists. For that, see the
[capability status](capability-status.md) and the
[capability ledger](capability-ledger.md).

| Tier | Who needs it | Promise in one line |
| :--- | :--- | :--- |
| **Core** | Every application | Supported: stable within 13.x, with the widest platform validation. |
| **Extension** | Applications that opt in | Supported: the same 13.x stability promise, for the documented scope of each crate. |
| **Experimental** | Applications that opt in and accept change | May change between 13.x releases and is not validated in every scenario. |

> **Prerelease note.** Until stable `13.0.0` is published, every v13 package is
> a prerelease and may change in any alpha, beta or release candidate. The
> promises below begin with `13.0.0`. [`SECURITY.md`](../../SECURITY.md) lists
> which published versions receive security fixes.

## What "supported" promises

Core and Extension packages are **supported**. In practice, that means:

- **Stability within 13.x.** Documented public Rust APIs, Cargo feature names,
  CLI commands and flags, configuration keys and serialized contracts follow the
  [compatibility policy](compatibility-policy.md). A 13.x minor or patch release
  does not intentionally break them. A removal is deprecated first and happens
  in a later major release. The policy's security exception still applies: an
  unsound or unsafe API can be disabled sooner, with an advisory.
- **Validation.** The documented scope runs in the repository's required checks:
  tests, strict Clippy, feature boundaries, the zero-panic and unsafe policies,
  dependency audits and package checks. The workspace runs on Linux for every
  pull request and on the full Linux, macOS and Windows matrix nightly; Core,
  the `rullst` facade and the macros also compile for WebAssembly. External
  services (payment, mail, AI and identity providers), real devices and
  production topologies count as validated only where the crate's own
  documentation says so.
- **Support.** Bug reports are triaged and fixes ship in a 13.x release. A known
  regression in a Core package blocks the release that would contain it. An
  Extension regression is fixed or documented as a known issue before release.
  Security reports follow [`SECURITY.md`](../../SECURITY.md). Rullst has a sole
  maintainer (see [`GOVERNANCE.md`](../../GOVERNANCE.md)) and offers no paid
  support or response-time commitment beyond `SECURITY.md`.

**Core vs Extension.** The difference is reach, not quality. Core packages are
in every application's dependency graph, so they get the widest platform
validation and the highest priority. Extensions are optional; you add them when
your application needs them. Within an Extension, the documentation of an
individual adapter or provider can give it a different label, and that label
wins. For example, `rullst-capital` supports Stripe and marks InfinitePay as
experimental.

## What "experimental" promises

Experimental packages are optional and opt-in. **Experimental does not mean
broken.** They pass the same gates as supported packages: tests, strict Clippy,
the zero-panic and unsafe policies, dependency audits and, where applicable,
fuzzing. What the label tells you:

- **The API can still change.** Public APIs, Cargo features, storage schemas and
  generated output may change between 13.x releases, including in ways that
  stop your code from compiling. Each breaking change is listed in the changelog
  and in the [v13 migration guide](migration-v13.md).
- **Some real-world scenarios are not validated yet.** Each experimental package
  lists its gaps in the table below and on its own page. Examples: physical
  hardware, live provider accounts, real devices and multi-host deployments.
  Tests cover the documented contract; they do not certify those environments.
- **Support is best effort.** Bug reports and contributions are welcome, and
  security reports are handled under `SECURITY.md` like any other package. A
  security fix may change an experimental API. A non-security regression does
  not block a release.

To use an experimental package today, pin an exact version in `Cargo.toml` and
read the changelog before you upgrade. Check that the documented scope covers
what your application needs.

## Packages

| Package | Tier | Scope notes |
| :--- | :--- | :--- |
| `rullst-core` | Core | HTTP runtime, routing, lifecycle, queues/realtime and the default security baseline. |
| `rullst-macros` | Core | `html!`, model and runtime macros. |
| `rullst-orm` | Core | Relational models, migrations and transactions. Optional store adapters document their own validation boundaries. |
| `rullst-orm-macros` | Core | Typed ORM code generation. |
| `rullst-security` | Core | Defense-in-depth middleware, guards and audit helpers. |
| `rullst` (facade) | Core | Its own API is Core. A feature that re-exports another crate has that crate's tier: `iot` and every `privacy` feature are experimental. |
| `cargo-rullst` | Core | Some generators have their own label; see [CLI generators](#cli-generators). |
| `rullst-auth` | Extension | Passwords, sessions, passkeys and authorization helpers. |
| `rullst-connect` | Extension | OAuth2/OIDC identity integrations. |
| `rullst-mail` | Extension | Transactional email and delivery controls; see the [crate page](crates/mail.md) for the supported drivers. |
| `rullst-capital` | Extension | Payments and billing: Stripe (supported) and InfinitePay (experimental). |
| `rullst-ai` | Extension | Guarded LLM clients and retrieval; provider coverage is in the [AI provider matrix](ai-provider-capabilities.md). |
| `rullst-nexus` | Extension | Admin CMS for registered models. |
| `rullst-studio` | Extension | Local developer control room. |
| `rullst-messaging` | Extension | Broker-neutral messaging, durable local state and the [Redis Streams profile](redis-messaging.md). |
| `rullst-iot` | Experimental | Not validated on physical hardware and has no network transport. See the [module status](crates/iot.md#module-status). |
| `rullst-privacy` | Experimental | Unpublished v13 package. No live age providers or verified guardianship; see the [privacy roadmap](privacy-age-assurance-roadmap.md). |
| `rullst-supervision` | Experimental | The host owns capture, models and reviewer workflow. Read [Responsible use](supervision.md#responsible-use) first. |
| `rullst-media` | Experimental | Interoperability with real Bunny Stream accounts, transcoding and CDN delivery is unvalidated; see the [managed-video candidate](managed-video.md). |
| `rullst-labs` | Experimental | `publish = false`, so it is not released to crates.io. Bring your own runner; see the [runner contract](labs-runner-contract.md). |

## CLI generators

`cargo-rullst` is a Core package, and commands without a label here have the
CLI's Core tier, within the boundaries described for each command in the
[CLI reference](cli_reference.md). A command that the reference marks as a v13
preview or candidate is unreleased. A generator that adds an Extension or
Experimental crate to your application cannot promise more than that crate. For
example, `make:privacy` and `make:age-gate` generate consumers of the
experimental `rullst-privacy`.

| Generator | Commands | Tier | Why |
| :--- | :--- | :--- | :--- |
| Omni desktop and Android | `make:omni --platform desktop`, `make:omni --platform android`, `omni desktop`, `omni android` (including `--release`) | Supported | The maintainer tests these targets directly. CI also generates fresh desktop shells and checks them on Linux, macOS and Windows, and compiles an unsigned Android debug APK. Signing keys, store review and every device or OS version stay outside the promise. |
| Omni iOS | `make:omni --platform ios`, `omni ios` | Experimental | Not part of the maintainer-tested scope. CI only compiles a fresh shell for a macOS iOS simulator target. |
| Foundry | `foundry:init`, `foundry:deploy` | Experimental | Assumes a prepared systemd/Caddy VPS with root or passwordless `sudo`. It replaces the global Caddyfile and has no automatic rollback. |
| Nix | `nixify`, `new --nix` | Experimental | Writes `flake.nix` and `.envrc` as starting points. Reproducibility depends on your pinned inputs. |
| Buildah | `generate:buildah`, `new --buildah` | Experimental | Rootless image builds depend on the host's Buildah and container setup. |
| gRPC | `make:grpc` | Experimental | Writes a Tonic service and `.proto` file. It does not edit `Cargo.toml`, add a `build.rs` or start a server. |
| IoT | `make:iot` | Experimental | Generates a telemetry module on top of the experimental `rullst-iot`. |

For an experimental generator, the generated files, their layout, and the
command's flags may change between 13.x releases. Review the diff when you
regenerate.

## How a tier changes

- **Promotion** (Experimental to Extension) can happen in any 13.x minor
  release. It requires recorded evidence for the gaps listed above, an API
  expected to stay stable, complete documentation and a maintainer who commits
  to supporting it. The changelog and this page record each promotion.
- **A supported package keeps its promise for the rest of 13.x.** Moving it to
  Experimental, or retiring it, is a major-release change. It is announced
  through deprecation, as the compatibility policy requires.
- An experimental package may be redesigned, extracted or retired in a later
  release. Any such change appears in the changelog and migration guide.

The [v13 maintenance scope](v13-maintenance-scope.md) explains the investment
decisions behind these tiers. For the two Core security crates, see
[which security layer to use, and when](security-layers.md).
