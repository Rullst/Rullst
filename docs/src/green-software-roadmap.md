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
| 4 | **Efficient HTTP defaults** | Long-lived `Cache-Control: immutable` for fingerprinted static assets, ETag/conditional requests, and modern image formats in the asset pipeline. Pre-compressed assets already exist: `cargo rullst build` writes Brotli/Zstandard siblings that the Core static handler serves. |
| 5 | **Lean pages** | Keep zero-bundle HTMX rendering the default, report page weight in `cargo rullst dash`, and warn on oversized assets. |
| 6 | **Database efficiency** | Development-time N+1 query warnings are in the v13 dash ([N+1 query warning](cli_reference.md#n1-query-warning)); slow-query hints are already visible there. Default pagination and index suggestions remain planned. |
| 7 | **Carbon-aware jobs** | Started in v13: see [carbon-aware deferrable jobs](#carbon-aware-deferrable-jobs-v13). Region selection remains planned. |
| 8 | **AI efficiency** | Token usage is already reported by `rullst-ai` and `cargo rullst ai`; add response caching, prompt-size budgets and guidance for choosing smaller or local models when they are sufficient. |
| 9 | **Resource view in the dash** | Show the app process's CPU and memory next to the request metrics so developers see the cost of a change while they work. |
| 10 | **Green hosting guidance** | Document how to choose lower-carbon regions and providers, and show region information during `cargo rullst deploy` when it is available. |

## Carbon-aware deferrable jobs (v13)

Opt-in and unpublished. A queue job or scheduled task can be marked
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
