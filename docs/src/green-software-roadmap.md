# Green Software Roadmap

Status: **planned (recorded 2026-10-01)** for the v13 feature line. Nothing on
this page is a shipped capability unless it links to existing documentation.
Parts of items 3 and 4 exist in the v13 development source (`13.0.0-alpha.1`);
see [measured release defaults](#measured-release-defaults).

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
| 1 | **`cargo rullst footprint` report** | Load-tests the running app with a fixed scenario and reports CPU time, peak and idle memory, requests per second, energy per request where the hardware exposes it (Linux powercap/RAPL), binary and container image size, and an estimated carbon intensity following the Software Carbon Intensity (SCI) specification (ISO/IEC 21031:2024). The report states the method, hardware, region grid intensity used and its uncertainty. |
| 2 | **Reproducible public benchmark** | The same reference application implemented idiomatically in Rullst and in other frameworks, with published code, scenario, hardware and dates, so comparisons are verifiable instead of claimed. |
| 3 | **Efficient release defaults** | v13 source: generated projects get a tuned `[profile.release]` (thin LTO, one codegen unit, stripped symbols; `panic` stays `unwind` so one panicking handler cannot stop the server) and a distroless runtime image ([CLI reference](cli_reference.md#cargo-rullst-new-name)). Remaining: measured image sizes and fast start-up suitable for scale-to-zero hosting. |
| 4 | **Efficient HTTP defaults** | v13 source: the standard `/static` mount sends `Cache-Control: public, max-age=31536000, immutable` for content-hashed file names and `no-cache` with `ETag`/`Last-Modified` revalidation (`304`) for other files ([static assets](tutorials/10-static-assets-and-compression.md#step-3-cache-headers)). Pre-compressed assets already exist: `cargo rullst build` writes Brotli/Zstandard siblings that the Core static handler serves. Remaining: fingerprinted names written by the asset pipeline itself and modern image formats. |
| 5 | **Lean pages** | Keep zero-bundle HTMX rendering the default, report page weight in `cargo rullst dash`, and warn on oversized assets. |
| 6 | **Database efficiency** | Development-time N+1 query detection, slow-query hints (already visible in the live dash), default pagination and index suggestions. |
| 7 | **Carbon-aware jobs** | Optional scheduling of deferrable queue jobs (reports, exports, re-indexing) for times or regions with lower grid carbon intensity, using a pluggable intensity source with an offline fallback. |
| 8 | **AI efficiency** | Token usage is already reported by `rullst-ai` and `cargo rullst ai`; add response caching, prompt-size budgets and guidance for choosing smaller or local models when they are sufficient. |
| 9 | **Resource view in the dash** | Show the app process's CPU and memory next to the request metrics so developers see the cost of a change while they work. |
| 10 | **Green hosting guidance** | Document how to choose lower-carbon regions and providers, and show region information during `cargo rullst deploy` when it is available. |

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

## Principles

- **Measure, then claim:** every number comes from a stated method and
  environment; estimates are labelled as estimates with their uncertainty.
- **Defaults over options:** efficiency should not require expert tuning.
- **No hidden network calls:** carbon-intensity data sources are opt-in, with
  an offline default.
- **Small surface:** measurement tooling lives in the CLI and development
  builds, never in production request paths.
