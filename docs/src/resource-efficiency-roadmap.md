# Resource Efficiency Roadmap

Status: **planned (recorded 2026-10-01)** for the v13 feature line. Nothing on
this page is a shipped capability unless it links to existing documentation.
Item 1 and parts of items 3, 4, 5, 6 and 7 exist in the v13 development source
(`13.0.0-rc.2`); see [measured release defaults](#measured-release-defaults).
The compiled crate count in the report (item 1), SQLite-only starters
without a TLS stack (item 3) and compressed pages after per-response CSRF
masking (item 4) were added in 13.0.0-alpha.6, the 13.0 feature freeze; the
rest comes after 13.0. The page was the "Green Software
Roadmap" until 13.0.0-alpha.5; it follows the methods of the green software
discipline, such as the Green Software Foundation's SCI, without describing
Rullst or the applications built with it as green.

## Goal

Rullst applications are compiled Rust services that render HTML on the server
with little client-side JavaScript. Independent studies of programming-language
energy efficiency rank Rust among the most efficient languages, and a small,
long-running native process usually needs less CPU and memory than an
interpreted stack for the same work. That is a real advantage, but it must be
**measured per application**, not assumed.

The goal is to make the efficient path the default, and to let each team
**measure and report** the footprint of its own Rullst application with a
recognised, reproducible methodology.

## Honest boundary: reports, not "greener than" certificates

Rullst will not issue certificates claiming that an application is "greener
than" one built with another framework. Such comparative or generic
environmental claims are regulated:

- In the European Union, Directive (EU) 2024/825 restricts generic
  environmental claims and requires claims to be substantiated.
- In the United States, the FTC Green Guides govern environmental marketing.
- In Brazil, consumer law and advertising self-regulation prohibit misleading
  environmental claims.

An unsubstantiated "green" badge could expose Rullst users to the very legal
risk the [legal & regulatory compliance roadmap](legal-compliance-roadmap.md)
aims to reduce. Instead, Rullst will produce **dated, reproducible measurement
reports** that state the method, the environment and the limits, which teams
can choose to publish.

## Planned work

| # | Item | Notes |
| :--- | :--- | :--- |
| 1 | **`cargo rullst footprint` report** | **13.0.0-alpha.5 development source, a Core command of the CLI:** [`cargo rullst footprint`](footprint.md) (`green` in 13.0.0-alpha.4) reports only measured figures, each with its method: release binary size, a named local container image's size, the app's idle resident memory (Linux, macOS and Windows), the `Cargo.lock` dependency count, with `--bench` the app's CPU time and response bytes per request under a bounded loopback load, and with `--page-weight` a page's bytes and requests on a first and a repeat visit and the third-party origins it references. It measures no energy; a Software Carbon Intensity figure (SCI, ISO/IEC 21031:2024) appears only as a labelled estimate from the measured CPU time and bytes and the power, network and grid figures you supply. It is a measurement and an estimate, not a certification or an environmental claim. 13.0.0-alpha.6 development source: the compiled dependency count beside the lockfile count (`cargo tree --locked --offline --edges normal,build` in the project directory, for the host platform and the default features; see [what is measured](footprint.md#what-is-measured)), and the start-up time of the release binary, the median of up to five cold starts, which a scale-to-zero host adds to the first request after an idle period ([start-up time](footprint.md#start-up-time)). Still planned after 13.0: a fixed multi-route reference scenario, repeated runs with a reported spread, and stated uncertainty for the estimate's inputs. |
| 2 | **Reproducible public benchmark** | The same reference application implemented idiomatically in Rullst and in other frameworks, with published code, scenario, hardware and dates, so comparisons are verifiable instead of claimed. |
| 3 | **Efficient release defaults** | v13 source: generated projects get a tuned `[profile.release]` (thin LTO, one codegen unit, stripped symbols; `panic` stays `unwind` so one panicking handler cannot stop the server) and a distroless runtime image ([CLI reference](cli_reference.md#cargo-rullst-new-name)). 13.0.0-alpha.6 development source: Studio and Nexus read the security telemetry store from Core instead of depending on `rullst-security`, so each generated starter's `Cargo.lock` lists 49 fewer packages (SaaS 398 to 349, Blank and API 374 to 325; registry packages, as `cargo rullst footprint` counts them). Passkeys check ES256 keys and signatures with aws-lc-rs, behind `rullst-auth`'s `passkeys` feature (`auth-passkeys` in `rullst`), instead of `p256`: an application without passkeys compiles neither, and the SaaS starter lists 18 fewer packages (349 to 331; Blank and API, which have no `rullst-auth`, are unchanged). Core also evaluates cron expressions itself instead of with the `cron` crate, 9 fewer packages in the Blank and API starters (325 to 316 in `Cargo.lock`, 251 to 242 compiled), and encrypts `storage-multipart` recovery records with RustCrypto `aes-gcm` in the same format, so `ring` leaves Core (with all features Core compiles 282 packages instead of 293). SQLite-only starters build no TLS stack ([measurements](#sqlite-starters-without-a-tls-stack)). Remaining: measured image sizes and fast start-up suitable for scale-to-zero hosting. |
| 4 | **Efficient HTTP defaults** | v13 source: the standard `/static` mount sends `Cache-Control: public, max-age=31536000, immutable` for content-hashed file names and `no-cache` with `ETag`/`Last-Modified` revalidation (`304`) for other files ([static assets](tutorials/10-static-assets-and-compression.md#step-3-cache-headers)). Pre-compressed assets already exist: `cargo rullst build` writes Brotli/Zstandard siblings that the Core static handler serves, and since 13.0.0-alpha.5 the generated Dockerfile writes them in the image. Since 13.0.0-alpha.6 the CSRF token is masked per response (BREACH) and the `Caddyfile` that `cargo rullst deploy` writes compresses pages (`encode zstd gzip`); until then pages were sent uncompressed ([compressed pages and BREACH](security-layers.md#compressed-pages-and-breach)). Remaining: fingerprinted names written by the asset pipeline itself and modern image formats. |
| 5 | **Lean pages** | Keep zero-bundle HTMX rendering the default. 13.0.0-alpha.5: `cargo rullst footprint --page-weight` reports a page's first-visit and repeat-visit bytes and requests ([page weight](footprint.md#page-weight)). Remaining: page weight in `cargo rullst dash` and warnings on oversized assets. |
| 6 | **Database efficiency** | Development-time N+1 query warnings are in the v13 dash ([N+1 query warning](cli_reference.md#n1-query-warning)); slow-query hints are already visible there. Default pagination and index suggestions remain planned. |
| 7 | **Carbon-aware jobs** | Started in v13: see [carbon-aware deferrable jobs](#carbon-aware-deferrable-jobs-v13). Region selection remains planned. |
| 8 | **AI efficiency** | Token usage is already reported by `rullst-ai`; add response caching, prompt-size budgets and guidance for choosing smaller or local models when they are sufficient. |
| 9 | **Resource view in the dash** | Show the app process's CPU and memory next to the request metrics so developers see the cost of a change while they work. |
| 10 | **Lower-carbon hosting guidance** | Document how to choose lower-carbon regions and providers, and show region information during `cargo rullst deploy` when it is available. |

## Measured release defaults

Measured on 2026-10-08 with the `13.0.0-alpha.1` development source. The
numbers describe this one starter on this one machine; they are not a
prediction for other applications or hardware.

- **Application:** `cargo rullst new green_probe --default --skip-initial-migration`
  (Blank starter, SQLite; `rullst` features `orm`, `strict-sqlite`, `studio`),
  dependency versions from the repository `Cargo.lock`, built `--offline`.
- **Toolchain:** Rust 1.98.1 (`RUSTUP_TOOLCHAIN=1.98.1`),
  `x86_64-unknown-linux-gnu`, rustc's default linker (the generated
  `.cargo/config.toml` selected neither mold nor lld).
- **Machine:** AMD Ryzen 5 7520U (8 logical CPUs), 3.4 GiB RAM, NVMe disk,
  Ubuntu with GCC 15.2, `CARGO_BUILD_JOBS=2`, a shared developer workstation.
- **Command:** `/usr/bin/time -v cargo build --release --offline --target-dir <new empty directory>`;
  size from `ls -l <target-dir>/release/green_probe`.
- **Samples:** two cold builds per profile, in alternating order
  (default then generated, then generated then default).

| `[profile.release]` | Binary size (bytes) | Wall-clock build (run 1 / run 2) | User CPU time (run 1 / run 2) | Largest process RSS (run 1 / run 2) |
| :--- | ---: | :--- | :--- | :--- |
| Cargo's default (section absent) | 13,966,600 | 6:17 / 6:17 | 648 s / 648 s | 723 MiB / 726 MiB |
| Generated: `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"` | 7,776,880 (−44.3 %) | 5:20 / 5:18 | 517 s / 516 s | 923 MiB / 970 MiB |

Each profile produced the same size in both runs. For comparison,
`strip --strip-all` applied to the default binary alone gives 10,031,552 bytes
(−28.2 %); thin LTO and one codegen unit account for the rest.

Limits of this measurement:

- With only two parallel jobs, a single codegen unit costs little
  parallelism; the lower CPU time is consistent with skipping the per-crate
  local ThinLTO across 16 units, but that cause was not isolated. With more
  parallel jobs the wall-clock result may reverse; that was not measured.
- `lto = true` (fat LTO) was not measured: it needs more memory than this
  machine could spare alongside other work.
- Runtime behaviour (requests per second, CPU time per request, memory) and
  energy were not measured.
- Container image size was not measured: Docker is not available on this
  machine. Measure the generated distroless image with `docker image ls` in CI
  or on a machine with Docker before publishing a number.

## SQLite starters without a TLS stack

13.0.0-alpha.6 development source. A SQLite database needs no TLS, but until
then every starter compiled rustls and aws-lc-rs: `rullst-orm` enabled SQLx's
TLS for every driver, Core's `orm` built rustls to install its provider, and
Studio built `rullst-capital` (and its HTTPS client) for the revenue page. Now
SQLx's TLS comes with the PostgreSQL and MySQL drivers, Core's `tls` feature
with every feature that makes TLS connections, and Studio's revenue dashboard
with the `capital` feature (see
[cryptography providers](security-architecture.md#cryptography-providers)).

- **Projects:** each starter generated by the CLI of the commit measured, its
  framework crates resolved from the checkout (as the
  [starter footprint budgets](https://github.com/Rullst/Rullst/blob/main/MAINTAINING.md#starter-footprint-budgets)
  job does) and its lock file resolved from the repository `Cargo.lock`.
- **Before:** `main` at `075f5a276` (after the security store moved to Core,
  passkeys without `p256`, Core without `ring` and `cron`, and the revocable
  sessions). **After:** the same commit with this change.
- **Counts:** `Cargo.lock` packages with a registry or git source, and those
  that `cargo tree --edges normal,build` lists for `x86_64-unknown-linux-gnu`
  (the `cargo rullst footprint` methods: third-party packages only, so the
  compiled figure leaves out the Rullst path crates that some counts above
  include), measured on 2026-11-15.

| Starter | `Cargo.lock` packages | Compiled packages | rustls / aws-lc-rs compiled |
| :--- | ---: | ---: | :--- |
| Blank or API, SQLite | 316 → 257 | 234 → 212 | yes → no |
| Blank, no database | 316 → 257 | 228 → 202 | yes → no |
| Blank, PostgreSQL | 316 → 280 | 238 → 228 | yes → yes (SQLx only) |
| Blank, MySQL | 316 → 280 | 231 → 220 | yes → yes (SQLx only) |
| Blank, Turso | 317 → 316 | 234 → 232 | yes → yes (HTTPS) |
| SaaS, SQLite | 322 → 321 | 242 → 241 | yes → yes (Stripe, mail HTTPS) |
| SaaS, PostgreSQL | 322 → 322 | 246 → 246 | yes → yes |

Release binaries and builds, measured earlier on `main` at `3e7adcd5c` (before
the passkey, `ring` and `cron` changes) with and without this change:

| Starter | Release binary (bytes) | Release build in the CI job |
| :--- | ---: | ---: |
| Blank, SQLite | 10,554,840 → 7,655,824 (−27.5 %) | 2 min 52 s → 1 min 44 s |
| API, SQLite | 10,494,936 → 7,600,752 (−27.6 %) | 19.9 s → 18.8 s |
| SaaS, SQLite | 15,121,560 → 15,120,432 | 1 min 38 s → 2 min 16 s |

- **Binaries:** the "Starter footprint budgets" job on a GitHub-hosted
  `ubuntu-latest` runner (4 logical CPUs, Rust 1.100.0), each starter's
  generated `[profile.release]`; run 43666669225 (before) and run
  43680823606 (after), one run each. Idle memory changed little (Blank
  9,891,840 → 9,457,664 bytes, API 9,805,824 → 9,379,840, SaaS 12,070,912 →
  12,099,584).
- **CI build times** are not clean builds: the job builds Blank, SaaS and API
  in that order into one target directory with sccache, so Blank's time is
  the first build with whatever the cache held, and the SaaS starter now
  compiles the TLS stack that the Blank build no longer leaves for it. All
  three took 4 min 50 s before and 4 min 19 s after.
- **Cold build on a workstation:** the Blank starter with SQLite,
  `cargo build --release --offline` into a new empty target directory with
  `CARGO_BUILD_JOBS=2` and no compiler cache, on the machine of
  [measured release defaults](#measured-release-defaults) with Rust 1.100.0,
  shared with other work, one run each: 3:46 → 2:50 wall clock, 367 s → 298 s
  user CPU time, largest process 924 → 941 MiB, target directory 630 → 541
  MiB, binary 10,274,864 → 7,494,928 bytes (rustc's default linker: the
  generated `.cargo/config.toml` selected neither mold nor lld).

- **First development build on a hosted runner:** measured on 2026-11-16
  with `main` at `fc8c05016` on GitHub's `ubuntu-latest` runner (Intel Xeon
  Platinum 8573C, 4 logical CPUs, Rust 1.100.0), `cargo build --offline` with
  the generated development profile into a new empty target directory,
  `CARGO_BUILD_JOBS=2` and no compiler cache, two runs each: Blank with
  SQLite 53.3 s and 54.0 s (266 units, no `aws-lc-sys`), SaaS with SQLite
  80.9 s and 81.0 s (307 units, of which the `aws-lc-sys` 0.47 build script
  took 22.4 s). See
  [build times](build-times.md#first-build-with-the-1300-alpha6-source).

The 22 packages a SQLite starter no longer compiles are rustls,
rustls-webpki, rustls-pki-types, webpki-roots, rustls-native-certs,
rustls-platform-verifier, openssl-probe, untrusted, tokio-rustls,
hyper-rustls, aws-lc-rs, aws-lc-sys and its build tools (cmake, jobserver,
dunce, fs_extra), and reqwest with the client-only packages it brought (ipnet,
want, try-lock, hex and a second tower-http version).

## Carbon-aware deferrable jobs (v13)

Opt-in; first published in the 13.0.0-alpha.1 prerelease. A queue job or scheduled task can be marked
deferrable with a deadline and daily time windows; see
[deferrable jobs and time windows](crates/core.md#deferrable-jobs-and-time-windows).
The mechanism:

- **Time windows.** The job becomes claimable at the start of the next allowed
  window (immediately inside an open one), never after its deadline; when no
  window opens in time, at the deadline.
- **Optional intensity source.** An application-provided
  `CarbonIntensitySource` returns forecast slots with their unit. A
  `CarbonAwarePlanner` places the job at the lowest-value slot inside its
  windows before the deadline. Core ships no network source; if the source
  fails or times out, the window rule applies and one warning is logged per
  outage.
- **Persistence.** Placement uses the queue's existing scheduled-job
  timestamp, so no schema changes and 12.x rows keep working.
- **Observability.** The chosen time, the reason (`window`, `intensity` or
  `deadline`) and the source name are returned in `DeferredJob` and recorded on
  a `rullst.queue.deferral` tracing span.

The planner only shifts when a job runs, using the values the source
reports. Whether that changes an application's emissions depends on the grid,
the workload and the data; Rullst does not measure or claim it.

## Principles

- **Measure, then claim:** every number comes from a stated method and
  environment; estimates are labelled as estimates with their uncertainty.
- **Defaults over options:** efficiency should not require expert tuning.
- **No hidden network calls:** carbon-intensity data sources are opt-in, with
  an offline default.
- **Small surface:** measurement tooling lives in the CLI and development
  builds, never in production request paths.
