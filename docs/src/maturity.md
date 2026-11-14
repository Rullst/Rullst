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
> a prerelease and may change in any prerelease. `13.0.0-alpha.5` is the 13.0
> feature freeze: `13.0.0-rc.1` carries only fixes and documentation. The
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
wins. For example, `rullst-media` supports its Bunny Stream adapter and marks
the opt-in `s3` adapter as experimental.

## What "experimental" promises

Since the [promotions in 13.0.0-alpha.5](#promotions-in-1300-alpha5), no
package and no CLI generator is Experimental. The label still applies to the
parts of a supported package that its documentation marks experimental: the
opt-in `s3` adapter of `rullst-media`, the `experimental-simulators` fixtures
of `rullst-iot` and `#[memoize]` in `rullst-macros`. A package added later can
also start in this tier.

Experimental packages and parts are optional and opt-in. **Experimental does
not mean broken.** They pass the same gates as supported packages: tests, strict Clippy,
the zero-panic and unsafe policies, dependency audits and, where applicable,
fuzzing. What the label tells you:

- **The API can still change.** Public APIs, Cargo features, storage schemas and
  generated output may change between 13.x releases, including in ways that
  stop your code from compiling. Each breaking change is listed in the changelog
  and in the [v13 migration guide](migration-v13.md).
- **Some real-world scenarios are not validated yet.** Each experimental
  package or part lists its gaps on its own page. Examples: physical
  hardware, live provider accounts, real devices and multi-host deployments.
  Tests cover the documented contract; they do not certify those environments.
- **Support is best effort.** Bug reports and contributions are welcome, and
  security reports are handled under `SECURITY.md` like any other package. A
  security fix may change an experimental API. A non-security regression does
  not block a release.

To use an experimental package or part, pin an exact version in `Cargo.toml`
and read the changelog before you upgrade. Check that the documented scope
covers what your application needs.

## Packages

| Package | Tier | Scope notes |
| :--- | :--- | :--- |
| `rullst-core` | Core | HTTP runtime, routing, lifecycle, queues/realtime and the default security baseline. |
| `rullst-macros` | Core | `html!`, model and runtime macros. |
| `rullst-orm` | Core | Relational models, migrations and transactions. Optional store adapters document their own validation boundaries. |
| `rullst-orm-macros` | Core | Typed ORM code generation. |
| `rullst-security` | Core | Defense-in-depth middleware, guards and audit helpers. |
| `rullst` (facade) | Core | Its own API is Core. A feature that re-exports another crate has that crate's tier: `iot` and every `privacy` feature are Extension, and so is `iot-ingest`, which builds on `iot`. |
| `cargo-rullst` | Core | Some generators have their own label; see [CLI generators](#cli-generators). |
| `rullst-auth` | Extension | Passwords, sessions, passkeys and authorization helpers. |
| `rullst-connect` | Extension | OAuth2/OIDC identity integrations. |
| `rullst-mail` | Extension | Transactional email and delivery controls; see the [crate page](crates/mail.md) for the supported drivers. |
| `rullst-capital` | Extension | Payments and billing with two supported adapters: Stripe, and InfinitePay for one-off payment links and source-confirmed notifications on its public Checkout API (its subscription, portal and other recurring operations return `UnsupportedOperation`). |
| `rullst-ai` | Extension | Guarded LLM clients and retrieval; provider coverage is in the [AI provider matrix](ai-provider-capabilities.md). |
| `rullst-nexus` | Extension | Admin CMS for registered models. |
| `rullst-studio` | Extension | Local developer control room. |
| `rullst-messaging` | Extension | Broker-neutral messaging, durable local state and the [Redis Streams profile](redis-messaging.md). |
| `rullst-iot` | Extension | `no_std` telemetry and frame helpers, MQTT/CoAP encoders and signed OTA verification, validated on a BBC micro:bit V2 and two Arduino Uno boards (see [hardware validation](crates/iot.md#hardware-validation-bbc-microbit-v2)). Network transports, secure elements (HSM), flashing and bootloaders are the application's. See the [module status](crates/iot.md#module-status). |
| `rullst-privacy` | Extension | Age-assurance policies and optional consent records, with SQLite or PostgreSQL stores. Live age providers and verified guardianship are out of scope; see the [privacy roadmap](privacy-age-assurance-roadmap.md). |
| `rullst-supervision` | Extension | Exam and parental supervision contracts. The host owns capture, models and reviewer workflow; no media model is included. Read [Responsible use](supervision.md#responsible-use) first. |
| `rullst-media` | Extension | A production application uses the Bunny adapter with a real Bunny Stream library (create, browser upload, status, update, deletion and private playback); transcoding and CDN edge cases are not covered by Rullst's own tests. The opt-in `s3` adapter (S3, R2, MinIO) stays experimental: it is validated against the offline mock only. See [managed video](managed-video.md). |
| `rullst-labs` | Extension | Contracts only; it never executes learner code. Bring your own runner and isolate it as the [runner contract](labs-runner-contract.md) requires: a container per job alone is not enough for hostile code. |

## CLI generators

`cargo-rullst` is a Core package, and commands without a label here have the
CLI's Core tier, within the boundaries described for each command in the
[CLI reference](cli_reference.md). A command that the reference marks as a v13
preview or candidate is not yet in a stable release. A generator that adds an Extension or
Experimental crate to your application cannot promise more than that crate. For
example, `make:privacy` and `make:age-gate` generate consumers of
`rullst-privacy`, an Extension.

| Generator | Commands | Tier | Why |
| :--- | :--- | :--- | :--- |
| IoT | `make:iot`, `iot:keygen`, `iot:sign` | Extension | Generates a telemetry module, and signs firmware updates, on top of `rullst-iot`, an Extension. |
| Footprint report | `footprint` | Core | Measured size, memory, dependency, benchmark CPU and byte figures, page weight and a labelled SCI estimate; Rullst's own CI budgets the three starters with it, and its files are in the pull-request mutation gate. See [resource figures for an app](footprint.md). |

No generator has been Experimental since 13.0.0-alpha.5. For a generator
labelled experimental, the generated files, their layout and the command's
flags may change between 13.x releases; review the diff when you regenerate.

13.0.0-alpha.4 removed the experimental Foundry, Nix, Buildah and gRPC
generators and the Fly.io, Railway and Render targets of `deploy`; see
[Advanced deployment](advanced-deployment.md) for those setups. It also removed
the terminal AI assistant (`cargo rullst ai`; use your own assistant with the
project's `AGENTS.md` and the site's
[`llms.txt`](https://rullst.github.io/Rullst/llms.txt)) and renamed
`footprint` to `green`; 13.0.0-alpha.5 renamed it `footprint` again, without
an alias.

`cargo rullst new` offers three starters with the CLI's Core tier: Blank, API
and SaaS. 13.0.0-alpha.4 moved the LMS, Blog, Portfolio and ERP starters out of
the CLI; the LMS and Blog applications in `examples/` replace them
(13.0.0-alpha.5 removed the ERP and Portfolio examples). These
[examples](examples.md) are applications to copy from and carry no
tier.

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

### Promotions in 13.0.0-alpha.5

On 12 November 2026 the maintainer promoted the packages and parts below out
of the Experimental tier and committed to supporting them for the rest of
13.x. The evidence recorded for each:

| Promoted | Tier now | Evidence |
| :--- | :--- | :--- |
| `rullst-iot`, the facade's `iot` and `iot-ingest` features, and `make:iot`, `iot:keygen` and `iot:sign` | Extension | Real hardware in the 13.0.0-alpha.5 acceptance work. On 2026-11-10 a BBC micro:bit V2 ran the encoder, signed-OTA (also streamed in 4 KiB chunks) and rollback-counter checks and verified an update signed by `cargo rullst iot:sign` ([hardware validation](crates/iot.md#hardware-validation-bbc-microbit-v2)). On 2026-11-11 two Arduino Uno boards sent telemetry through `rullst::iot_ingest` to a device registry kept by the application, and every post with a revoked device's token was refused ([two-board run](crates/iot.md#two-boards-through-rullstiot_ingest)). Network transports, secure elements, flashing and bootloaders are application-owned: that is the crate's scope, not a validation gap. |
| `rullst-media` | Extension | A production application uses the Bunny adapter with a real Bunny Stream library: create, browser upload, status, update, deletion and private playback. The `conformance` kit checks application-owned providers against the same contract. The opt-in `s3` adapter stays experimental. |
| `rullst-labs` | Extension | Contracts only; it never executes learner code. In the 13.0.0-alpha.5 acceptance run, a runner that starts one Docker container per job graded Python submissions end to end through these contracts: correct answers 3/3, wrong answers 0/3, a syntax error reported with the compiler's message, network access blocked and the wall-time limit enforced. A container per job is not enough isolation for hostile code; see the [runner contract](labs-runner-contract.md#isolation-the-host-owns). |
| `rullst-privacy` and the facade's `privacy` features | Extension | The 13.0.0-alpha.5 acceptance application used the `make:age-gate` and `make:privacy` consumers with PostgreSQL consent stores: age declarations, signed attestations (with replay, forged-signature, binding and clock-rollback refusals), consent, withdrawal, a new notice version and the own-account export. Live age providers and verified guardianship stay out of scope. |
| `rullst-supervision` | Extension | Exercised on a real phone in the acceptance application: camera presence (including a covered camera), page hidden and visible, the microphone path that sends loudness statistics only (speech and no speech recorded) and parental time windows. No media model is included; read [Responsible use](supervision.md#responsible-use) first. |
| The InfinitePay adapter of `rullst-capital` | supported | One-off payment links and source-confirmed notifications: one real Pix payment confirmed, forged webhooks and a wrong amount refused, and replays idempotent. Subscriptions, the customer portal, usage, coupons and trials return `UnsupportedOperation`, which is the adapter's scope. |
| `cargo rullst footprint` | Core (the CLI's tier) | Rullst's own CI budgets the three starters with it. |

The [v13 maintenance scope](v13-maintenance-scope.md) explains the investment
decisions behind these tiers. For the two Core security crates, see
[which security layer to use, and when](security-layers.md).
