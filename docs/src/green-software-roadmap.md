# Green Software Roadmap

Status: **planned (recorded 2026-10-01)** for the v13 feature line. Nothing on
this page is a shipped capability unless it links to existing documentation.

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
| 3 | **Efficient release defaults** | Generated projects get a tuned `[profile.release]` (LTO, single codegen unit, stripped symbols, abort on panic where safe), smaller container images (static or distroless runtime) and fast start-up suitable for scale-to-zero hosting. |
| 4 | **Efficient HTTP defaults** | Long-lived `Cache-Control: immutable` for fingerprinted static assets, ETag/conditional requests, pre-compressed Brotli/Zstd assets, and modern image formats in the asset pipeline. |
| 5 | **Lean pages** | Keep zero-bundle HTMX rendering the default, report page weight in `cargo rullst dash`, and warn on oversized assets. |
| 6 | **Database efficiency** | Development-time N+1 query detection, slow-query hints (already visible in the live dash), default pagination and index suggestions. |
| 7 | **Carbon-aware jobs** | Optional scheduling of deferrable queue jobs (reports, exports, re-indexing) for times or regions with lower grid carbon intensity, using a pluggable intensity source with an offline fallback. |
| 8 | **AI efficiency** | Token usage is already reported by `rullst-ai` and `cargo rullst ai`; add response caching, prompt-size budgets and guidance for choosing smaller or local models when they are sufficient. |
| 9 | **Resource view in the dash** | Show the app process's CPU and memory next to the request metrics so developers see the cost of a change while they work. |
| 10 | **Green hosting guidance** | Document how to choose lower-carbon regions and providers, and show region information during `cargo rullst deploy` when it is available. |

## Principles

- **Measure, then claim:** every number comes from a stated method and
  environment; estimates are labelled as estimates with their uncertainty.
- **Defaults over options:** efficiency should not require expert tuning.
- **No hidden network calls:** carbon-intensity data sources are opt-in, with
  an offline default.
- **Small surface:** measurement tooling lives in the CLI and development
  builds, never in production request paths.
