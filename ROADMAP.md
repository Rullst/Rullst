# Rullst Roadmap 🗺️

A planning document: intentions, not promises. `ROADMAP.md` is canonical
(`docs/src/roadmap.md` embeds it in the book); the
[capability ledger](docs/src/capability-ledger.md) holds the evidence and
limits of each capability, and [`CHANGELOG.md`](CHANGELOG.md) what shipped.

## In one minute

- **Where v13 is (20 November 2026):** `13.0.0-alpha.1` to `alpha.6` were
  published from 11 October to 20 November 2026; `13.0.0-alpha.6` is the 13.0
  feature and public API freeze, and `13.0.0-rc.1` is in preparation.
- **Next:** `13.0.0-rc.1` (fixes, documentation and the changes approved
  after the freeze), `13.0.0`, then
  [13.1](#next-131). v12 reached end of life on 16 October 2026 (12.3.0).
- **Rule:** additive work in 13.x minors, breaking changes avoided, no v14 by
  choice ([release rule](#release-rule)).
- **Next twelve months (November 2026 – November 2027):** 13.0.0, 13.1, later
  minors ordered by user demand, and the [verification roadmap](#verification-roadmap).

## Release rule

- **13.0.0-rc.1 takes fixes and documentation,** and the changes the
  maintainer approves one by one after the freeze (listed under
  [Planned after 13.0.0-alpha.6](#planned-after-1300-alpha6)); a changed
  `.github/public-api/` listing must be one of them, named in the changelog
  and the migration guide.
- **Additive improvements go to 13.1 and later minors** (new opt-in features,
  new APIs beside the existing ones; [support window](docs/src/compatibility-policy.md#supported-release-window)).
- **Breaking changes are avoided.** The maintainer wants a long, stable 13.x
  and no v14 by choice. A major version happens only if the ecosystem forces
  it, for example a breaking Axum, SQLx or Tokio release that changes types
  Rullst exposes ([public dependencies](docs/src/compatibility-policy.md#public-dependencies)).
  A replaced API is deprecated and keeps working.

## Now: 13.0.0

### Path to 13.0.0

1. **13.0.0-alpha.6** was published on 20 November 2026: the 13.0 feature
   and public API freeze ([release lines](#published-and-planned-release-lines),
   [history](#delivered-in-1300-alpha6)).
2. **13.0.0-rc.1** fixes the defects found in alpha.6 (by its admission, the
   applications upgraded to it and the first-hour and deployment jobs),
   corrects documentation and carries the changes the maintainer approved
   after the freeze; it adds no feature.
3. **13.0.0** follows once rc.1 leaves no open defect in the stability scope;
   the [maturity tiers](docs/src/maturity.md) and the 13.x support window
   apply from it.

### Planned after 13.0.0-alpha.6

Merged into `main` for rc.1:

- **The Experimental tier removed** (decided 19 November 2026: the opt-in
  `s3` adapter of `rullst-media` and `#[memoize]` have the tier of their
  package, and the documentation states what each part is tested against;
  [maturity tiers](docs/src/maturity.md#no-experimental-tier-since-19-november-2026))
  and **"Reporting problems"** sections in every crate's documentation, with a
  bug-report issue form ([#775](https://github.com/Rullst/Rullst/pull/775)).
- **"Why not Leptos or Dioxus?"** on the [comparison page](https://rullst.win/compare/),
  and links to [Scope and limits](docs/src/scope-and-limits.md)
  ([#774](https://github.com/Rullst/Rullst/pull/774)).
- **A robotics and IoT classroom guide** for teaching with `rullst-iot`
  ([#776](https://github.com/Rullst/Rullst/pull/776)).
- **No Experimental names in public APIs** (approved after the freeze;
  breaking): `rullst-iot` drops the `experimental-simulators` feature and its
  SHA-256 fixtures, and `rullst-labs` renames
  `ExecutionProfile::LinuxExperimental` to `PinnedRunner` and
  `ResultEvidence::Experimental` to `SignedReceipt`, keeping their stored form
  ([#778](https://github.com/Rullst/Rullst/pull/778)).
- **Fewer pool round trips** (approved after the freeze): a PostgreSQL or
  MySQL pool created by `Orm` pings a connection before handing it out only
  after 500 ms idle, instead of on every acquisition
  ([#780](https://github.com/Rullst/Rullst/pull/780)).

Still in progress: the fixes from the applications upgraded to alpha.6 and an
independent review of what the documentation claims.

## Next: 13.1

Additive, decided with the maintainer on 19 November 2026:

- **`cargo rullst bug-report`:** collects the Rullst version, enabled features
  and platform, and opens a prefilled issue without sending code.
- **Nightly provider sandbox tests** against Stripe test mode and a Bunny
  Stream test library, with maintainer-owned GitHub secrets: they cover the 29
  Capital [mutants](docs/src/mutation-testing.md#campaign-status) that only a
  live Stripe response reaches, and Media, which has no live test today.
- **Automatic SQLite backup (M38, reframed):** scheduled online backup to S3,
  R2 or Bunny Storage, and a restore command, instead of read replicas; today
  the [VM guide](docs/src/vm-deployment.md#8-back-up-the-database-volume)
  documents a manual `sqlite3 ".backup"`.
- **`make:chat`:** person-to-person chat on Core realtime (`BroadcastManager`,
  `PresenceTracker`, `RealtimeSse`/WebSockets) and the ORM: rooms, history,
  presence, moderation and reporting, rate limits and tenant isolation. It is
  distinct from `make:chat-session`, which is AI conversation memory.
- **Outgoing webhooks in Nexus:** delivery status of `rullst-messaging`
  `webhooks` and one-click retry of dead letters.
- **Async embedded IoT with Embassy (M25):** validated on an ESP32-C3
  ([#637](https://github.com/Rullst/Rullst/issues/637)).
- **Read-only natural-language-to-SQL in Studio (M36):** built on the Nexus
  AI assistant; a query runs only after the developer approves it.
- **Lenient date text forms,** if users ask: a space before the offset and a
  `UTC` suffix, which no longer decode since the ORM
  [moved off `chrono`](docs/src/migration-v13.md#moving-off-chrono).

### 13.x minor candidates and decided non-goals

Recorded on 30 October 2026, unscheduled and ordered by user demand: **htmx 4
as an opt-in for new applications** once npm's `latest` is 4.x and its SSE
extension is stable; **`cargo rullst footprint` on FreeBSD**
([#640](https://github.com/Rullst/Rullst/issues/640)), which needs a FreeBSD
machine; **Turso/libSQL beyond the Blank and API starters**, on demand and at
low cost (`Schema` emitting Turso migrations, `#[derive(Orm)]` generating the
Turso model).

Decided non-goals: **full ORM parity for Turso/libSQL**, since a second client
path beside SQLx for every ORM feature would double the maintenance and test
matrix; **reading the CSRF `_token` from any multipart position**, which would
buffer uploads before the check (forms send `_token` first, and clients can
send `X-CSRF-Token`).

## Later (13.x, no date)

Compatible ideas; none is scheduled, and each needs a maintainer, focused
tests and an acceptance environment first.

- **AI-assisted maintenance:** blueprint-specific `AGENTS.md` and a versioned
  discovery schema ([AI maintainability](docs/src/ai-maintainability-roadmap.md)).
- **Security and identity:** WebAuthn/FIDO2 conformance before a stable
  passkey claim; key rotation and Cloud KMS; distributed rate limits, durable
  audit sinks and SIEM connectors; counsel-reviewed compliance building
  blocks ([legal compliance](docs/src/legal-compliance-roadmap.md)).
- **Data and integrations:** remote brokers (RabbitMQ, Kafka, NATS, SQS/SNS,
  GCP Pub/Sub, Pulsar) after provider fault evidence; presigned uploads and
  image resizing; explain-and-approve index advice (M8).
- **IoT and edge:** MQTT, CoAP and LoRaWAN transports; firmware download,
  flashing and rollback; hardware-in-the-loop CI; audited HSM adapters.
- **Safe updates:** compatible updates work today; cold and warm timings and
  power-loss and disk-full evidence remain open
  ([assisted upgrades](docs/src/tutorials/36-assisted-framework-upgrades.md)).
- **API documentation:** per-interface reference pages and one REST learning
  path ([API navigation index](docs/src/api-reference.md)); also the
  [resource efficiency](docs/src/resource-efficiency-roadmap.md) programme.
- **Compatible paths instead of a v14:**
  - the legacy cron syntax of messaging schedules stays deprecated but working
    ([migration guide](docs/src/migration-v13.md#messaging-schedules-and-posix-cron));
  - Rullst-owned types are added beside third-party public types, which are
    then deprecated, to shrink the 25 public dependencies over time;
  - the security stack is consolidated inside `rullst-security`, keeping
    every public path.

### Ideas recorded for after 13.0

Ideas, not commitments (8 October 2026; robotics on 19 November 2026):

- **Offline IoT/health example:** sensor readings in SQLite on a dashboard
  without internet; the health variant treats readings as sensitive data and
  is not a medical device.
- **Multiplayer quiz:** Core realtime and presence with `rullst-labs` grading
  for server-authoritative scoring.
- **Reverse proxy instead of a gateway:** extend the generated Caddy output
  and add nginx, on Core's readiness/drain and trusted-proxy contracts.
- **Robotics education** from the
  [classroom guide](docs/src/iot-classroom.md)
  ([#776](https://github.com/Rullst/Rullst/pull/776)): a classroom weather
  station, a robot web control panel, a competition scoreboard and signed
  firmware updates.

## If a major version is ever needed

None is scheduled, and the maintainer prefers a long 13.x. If the ecosystem
forced one, it would also remove what 13.x deprecated:

- the legacy cron syntax of messaging schedules;
- the third-party public types that Rullst-owned types replaced.

## Long-term vision (no date)

Kept as future planning, outside the 13.x promise:

- **Aerospace, autonomous vehicles, robotics and defence (M31):** separate
  programmes outside the web framework; flight and defence software needs
  certification (such as DO-178C), hard real time and export control.
- **Post-quantum (M13):** rustls 0.23.46 with the aws-lc-rs provider, used for
  every Rullst TLS connection, already supports the hybrid `X25519MLKEM768`
  key exchange. Post-quantum signatures, such as for OTA manifests (Ed25519
  today), wait for mature standards and audited crates. The `rullst-iot`
  post-quantum placeholder (SHA-256 test fixtures with no ML-KEM, key exchange
  or quantum safety) was removed for 13.0.0-rc.1
  ([#778](https://github.com/Rullst/Rullst/pull/778)).
- **Homologated NFS-e issuance:** a separate product outside the framework.

## Not planned, and why

| Milestone | Why |
| :--- | :--- |
| M8, automatic production index changes | DDL without review is not worth the risk; explain-and-approve advice stays ([Later](#later-13x-no-date)). |
| M20, an event-streaming and ledger engine | It overlaps `rullst-messaging` (durable delivery, consumer groups, dead letters) and the HMAC audit chain. |
| M16, Wasm islands (removed in 13.0.0-alpha.5) | The default CSP has no `'wasm-unsafe-eval'`, so browsers could not run them under the default headers. |
| M18, LiveView-style components (removed in 13.0.0-alpha.5) | htmx with `RealtimeSse` or WebSockets covers server push ([migration](docs/src/migration-v13.md#upgrading-an-alpha4-application-to-alpha5)). |
| M30, gRPC scaffolding (`make:grpc` removed in 13.0.0-alpha.4) | Use `tonic` through the Axum escape hatch (`rullst::web::axum`). |
| M39, a Rullst gateway (abandoned 8 October 2026, never built) | Established reverse proxies cover it; `cargo rullst deploy` writes a Caddyfile. |

## Milestone index

**Delivered (with scope)** means it works, is tested and its limits are
documented; **In progress** means part of it ships and is tested, and its link
says what is still open. A status moves up only on a commit that passes every
gate, with a benchmark for performance and a threat model for security claims.

| Area | Delivered (with scope) | In progress | Next (13.1) | Long-term, not planned or removed |
| :--- | :--- | :--- | :--- | :--- |
| Developer experience | [M4](docs/src/cli_reference.md) `make:resource` and error console; [M28](docs/src/tutorials/27-zero-cost-di.md) compile-time DI; [M32](docs/src/capability-ledger.md#architecture-and-framework-contract) Axum/Tower escape hatches | [M1](docs/src/capability-ledger.md#cli-generated-applications-and-release-engineering) `make:*` generators; [M2](docs/src/build-times.md) build tuning and hot reload; [M3](docs/src/capability-ledger.md#architecture-and-framework-contract) granular features and ejection; [M5](docs/src/capability-ledger.md#cli-generated-applications-and-release-engineering) OpenAPI and TypeScript generation; [M29](docs/src/api-reference.md) Scalar `/docs`; [M34](docs/src/capability-ledger.md#cli-generated-applications-and-release-engineering) SDK generator | | |
| ORM and data | | [M6](rullst-orm/ROADMAP.md) ORM and Turso/libSQL | [M38](#next-131) SQLite backup (reframed) | [M8](#not-planned-and-why) automatic index changes: not planned; [M20](#not-planned-and-why) ledger engine: not planned |
| Auth and security | | [M9](docs/src/capability-ledger.md#security-authentication-studio-and-nexus) auth, OAuth/OIDC and passkeys; [M10](docs/src/capability-ledger.md#security-authentication-studio-and-nexus) validation and rate limiting; [M12](rullst-security/ROADMAP.md) defence in depth (continuous) | | [M13](#long-term-vision-no-date) post-quantum: long-term vision |
| SaaS and admin | | [M11](docs/src/capability-ledger.md#capital-and-billing-vision) Nexus and billing; [M33](docs/src/capability-ledger.md#capital-and-billing-vision) declarative entitlements | | |
| Runtime and realtime | | [M7](docs/src/capability-ledger.md#connect-real-time-queues-storage-and-data) edge runtime and distributed data; [M15](docs/src/capability-ledger.md#connect-real-time-queues-storage-and-data) queues and messaging; [M17](docs/src/capability-ledger.md#connect-real-time-queues-storage-and-data) realtime, storage and media | | [M30](#not-planned-and-why) gRPC scaffolding: removed |
| Frontend | | [M14](docs/src/scope-and-limits.md) HTMX-first SSR; [M21](docs/src/capability-ledger.md#cli-generated-applications-and-release-engineering) PWAs and mobile bridge | | [M16](#not-planned-and-why) Wasm islands: removed; [M18](#not-planned-and-why) LiveView: removed |
| IoT and robotics | | [M24](rullst-iot/ROADMAP.md) `no_std` frames and signed OTA | [M25](#next-131) Embassy | [M31](#long-term-vision-no-date) aerospace, vehicles, robotics and defence: long-term vision |
| AI and observability | [M19](docs/src/capability-ledger.md#security-authentication-studio-and-nexus) Radar, spans and `/metrics` | [M22](docs/src/capability-ledger.md#ai-and-mail) agentic DevOps advice; [M23](docs/src/capability-ledger.md#ai-and-mail) auto-healing diagnostics; [M35](docs/src/distributed-tracing.md) trace waterfall; [M37](docs/src/capability-ledger.md#ai-and-mail) error-console fix | [M36](#next-131) natural-language SQL | |
| Operations and deploy | [M27](docs/src/tutorials/25-kubernetes-deployment.md) Kubernetes manifests and probes | [M26](docs/src/vm-deployment.md) guided VM deploy | | [M39](#not-planned-and-why) Rullst gateway: not planned |
| Learning (Labs, Supervision, Privacy) | | [M40](docs/src/labs-runner-contract.md) Labs contracts; [M41](docs/src/privacy-age-assurance-roadmap.md) privacy and age assurance | | |

`rullst-supervision` has no milestone; see its [README](rullst-supervision/README.md).

### Audit of the detailed crate roadmaps

The `rullst-*/ROADMAP.md` design backlogs are interpreted in the
[detailed crate roadmaps table](docs/src/capability-ledger.md#detailed-crate-roadmaps);
see also [capability status](docs/src/capability-status.md) and the
[investment proposal](docs/src/v13-priorities.md).

## Verification roadmap

Assurance experiments, not current guarantees; [WORKFLOWS.md](WORKFLOWS.md)
describes what runs today, and TLS on aws-lc-rs is a decided policy. Keep the
OpenSSF Silver badge current, extend mutation testing
([campaign status](docs/src/mutation-testing.md#campaign-status)) and prepare
an independent review ([audit kit](docs/src/external-audit-kit.md)).

| Experiment | Status and plan |
| :--- | :--- |
| Differential databases, browsers and accessibility, `cargo-careful` | **In CI since 13.0.0-alpha.5** ([WORKFLOWS.md](WORKFLOWS.md#required-local-and-release-baseline)); the browser job is a required release job since alpha.6, not a pull-request check or WCAG certification. |
| Live provider sandbox conformance | **Next (13.1):** Stripe and Bunny Stream nightly; Resend and OAuth providers later. |
| Verus proofs | **13.x: hosted acceptance** of the manual-only `AgePolicy::permits` pilot, outside release admission ([scope](docs/src/verus-roadmap.md)). |
| Shuttle (in place of Loom) | **Optional, no date** (31 October 2026): only the in-memory broker and the rate limiter; Loom does not model Tokio. |
| Chaos testing with `fail-rs` | **Not planned** (31 October 2026): global failpoints are implicit runtime behaviour; injectable `Clock` traits and failing doubles instead. |
| `cargo-vet` | **Not planned with a sole maintainer** (31 October 2026): exemptions become a rubber stamp; `cargo-deny`, `cargo audit`, SBOMs and attested releases cover the supply chain. |
| Zero-allocation assertions, PGO/BOLT, AFL.rs, Cosign | **Deferred** until benchmarks, production profiles, healthy corpora for the 42 fuzz targets or separately distributed binaries exist. |
| Cross-platform CI acceleration | **Deferred.** Acceptable only with (1) distinct target roots for repository and generated-project builds; (2) cold and warm times, cache hits, test and doctest counts recorded on all three systems; (3) the all-feature, outbox and live-provider matrices preserved; (4) caches keyed by OS, compiler, lockfile and profile, bounded, never evidence, not writable by untrusted pull requests; (5) nextest only with equivalent semantics, separate doctests and `cargo test` as recovery; (6) repeated green cold and warm runs on every OS, a material gain, no new flakes or lost tests, and a rollback path. |
| Upstream OSS-Fuzz, independent security review | **Needs the maintainer:** `helper.py build_fuzzers` and `check_build` for every target before submission; repository automation cannot award review evidence to itself. |

## Published and planned release lines

The release process maintains this table and records each publication
after its tag.

| Version | Status | Honest scope |
| :--- | :---: | :--- |
| **v12.0.0** | `[x] Published stable` | Tag `v12.0.0` at `eb11f892` completed the protected release workflow and published all sixteen packages on September 15, 2026. |
| **v12.x** | `[x] End of life` | Ended on 16 October 2026 with 12.3.0 as the last release; the `v12` branch is frozen. New 12.x projects no longer resolve dependencies after crates.io yanked the `zip` releases its Studio needs; use v13. |
| **v12.1.0** | `[x] Published compatible minor` | All sixteen packages were published from `b62390b4` on 20 September 2026 UTC. Guided CLI/project updates, SaaS/Nexus fixes, account mail and the other bounded contracts are recorded with immutable source and registry evidence in the [publication record](docs/src/v12.md#1210-published-maintenance-release). |
| **v12.1.1** | `[x] Published patch` | All sixteen packages were published from `d27db26c` on 24 September 2026 UTC; see the [publication record](docs/src/v12.md#1211-published-maintenance-release). |
| **v12.1.2** | `[x] Published patch` | Tagged `v12.1.2` on 28 September 2026 UTC from the `v12` maintenance line; its publication evidence is kept in the [frozen v12 release record](https://github.com/Rullst/Rullst/blob/v12/docs/src/v12.md). |
| **v12.2.0** | `[x] Published compatible minor` | Tagged `v12.2.0` at `565e222a` on 3 October 2026 from the `v12` maintenance line; see the [frozen v12 release record](https://github.com/Rullst/Rullst/blob/v12/docs/src/v12.md). |
| **v12.3.0** | `[x] Latest published stable` | Tagged `v12.3.0` at `67b2d0e5` and published on 9 October 2026: hickory 0.26.3, fewer WAF/RASP false positives and deprecations of the Capital and Mail APIs that 13.0 removes. It is the last 12.x release: the `v12` line reached end of life on 16 October 2026 and no further patch will be published. |
| **v13.0.0-alpha.1** | `[x] Published prerelease` | Tag `v13.0.0-alpha.1` at `fc44bf76`; all twenty packages were published on 11 October 2026 for evaluation. See the [delivery plan](docs/src/v13-delivery-plan.md). |
| **v13.0.0-alpha.2** | `[x] Published prerelease` | All twenty packages were published on 17 October 2026 from tag `v13.0.0-alpha.2` at `0daf4941` ([release run 39041726912](https://github.com/Rullst/Rullst/actions/runs/39041726912)): the fixes from the alpha.1 dogfooding report and the breaking `html!` → `Markup`, `Schema` date/time and Nexus changes. A first tag at `669befe6` failed release verification before publishing anything and was replaced. |
| **v13.0.0-alpha.3** | `[x] Published prerelease` | All twenty packages were published on 24 October 2026 from tag `v13.0.0-alpha.3` at `be336833` ([release run 40129821475](https://github.com/Rullst/Rullst/actions/runs/40129821475)) after all 25 required workflows passed on that commit. See the [changelog](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md#1300-alpha3---2026-10-23). |
| **v13.0.0-alpha.4** | `[x] Published prerelease` | All twenty packages were published on 31 October 2026 from tag `v13.0.0-alpha.4` at `2bd3c2ad` ([release run 41233925160](https://github.com/Rullst/Rullst/actions/runs/41233925160)). See the [changelog](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md#1300-alpha4---2026-10-31). |
| **v13.0.0-alpha.5** | `[x] Published prerelease` | All twenty packages were published on 14 November 2026 from tag `v13.0.0-alpha.5` at `5feda4ac` ([release run 43631881119](https://github.com/Rullst/Rullst/actions/runs/43631881119)) after all 23 required workflows passed on that commit. A first tag at `034df65e` stopped in release verification before publishing anything (`cargo rullst audit` lacked the governed advisory exceptions, #690) and was replaced with the maintainer's approval. The fixes and additions from the applications upgraded to alpha.4; planned as the 13.0 feature and public API freeze, which moved to alpha.6 the same day. See [Delivered in 13.0.0-alpha.5](#delivered-in-1300-alpha5) and the [changelog](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md#1300-alpha5---2026-11-14). |
| **v13.0.0-alpha.6** | `[x] Published prerelease` | All twenty packages were published on 20 November 2026 from tag `v13.0.0-alpha.6` at `58438b64` ([release run 44753380780](https://github.com/Rullst/Rullst/actions/runs/44753380780)) after all 23 required workflows passed on that commit. Admission of the first candidate, `ef28c0e0`, found two Miri scopes and the packaged distribution's email-login consumer failing; [#773](https://github.com/Rullst/Rullst/pull/773) fixed them before the tag. The 13.0 feature and public API freeze: every pending fix and improvement, the Rust 1.100 toolchain and current dependency majors, the removal of every API still deprecated (in 12.x or during the 13.0 prereleases), smaller dependency graphs and the `#[non_exhaustive]` public API audit. See [Delivered in 13.0.0-alpha.6](#delivered-in-1300-alpha6) and the [changelog](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md#1300-alpha6---2026-11-20). |
| **v13.0.0-rc.1** | `[ ] Planned` | Fixes, documentation and the changes the maintainer approved after the alpha.6 feature and API freeze. See [Planned after 13.0.0-alpha.6](#planned-after-1300-alpha6). |
| **v13.0.0** | `[ ] Planned stable` | Follows once rc.1 leaves no open defect in the stability scope; the [maturity tiers](docs/src/maturity.md) apply from this release. |
| **v13.x** | `[ ] After 13.0.0` | Compatible minors after 13.0.0: 13.1 is planned under [Next: 13.1](#next-131) and later ideas under [Later](#later-13x-no-date); breaking changes are avoided ([release rule](#release-rule)). |

## History

Each release's changes are in [`CHANGELOG.md`](CHANGELOG.md), its evidence in
the [delivery plan](docs/src/v13-delivery-plan.md) and v12 in the
[v12 record](docs/src/v12.md).

### Delivered in 13.0.0-alpha.6

The 13.0 freeze: Rust 1.100, current dependency majors, no deprecated API,
smaller dependency graphs and the `#[non_exhaustive]` audit
([changelog](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md#1300-alpha6---2026-11-20)).

### Delivered in 13.0.0-alpha.5

What the applications on alpha.4 asked for, and the removal of islands, Live
components and three ORM adapters
([changelog](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md#1300-alpha5---2026-11-14)).

---

<div align="center">
  <p><i>"All glory and honor to God יהוה in the name of Yeshua the Messiah (Jesus Christ)."</i></p>
</div>
