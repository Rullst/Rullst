# Resource Figures for an App (`cargo rullst footprint`)

**Status:** command of the main CLI, `cargo-rullst`, with the CLI's Core
[maturity tier](maturity.md#cli-generators) since 13.0.0-alpha.5 (Experimental
before; it was `cargo rullst green` in 13.0.0-alpha.4).
Part of the [resource efficiency roadmap](resource-efficiency-roadmap.md).

`cargo rullst footprint` reports resource figures it **measured** for your
application, says how it measured each one, and says which ones it could not
measure: binary and image size, idle memory, start-up time, dependencies, CPU
time and bytes per request under a bounded load, and the weight of a page on a
first and a repeat visit. It can check those figures against budgets in CI and set them
beside the bytes the application served in production, read from its
monitor. It measures no energy. A carbon figure appears only as a
Software Carbon Intensity (SCI) **estimate**, computed from figures that you
supply, and it is labelled `estimate` in the text and the JSON together with
every input. The report describes one run on one machine.

**The report is a measurement and an estimate, not a certification or an
environmental claim.** It does not rate your application or compare it with
other software. If your application publishes these numbers (on a website,
in marketing or in a sustainability report), you are responsible for how you
word them: state the method, the machine, the date and the inputs, and do not
present the estimate as a measured emission or the report as an
environmental label. Generic environmental claims to consumers are regulated,
for example by Directive (EU) 2024/825 from 27 September 2026.

## Install

`footprint` is part of the CLI; nothing else to install:

```bash
cargo install cargo-rullst --version =13.0.0-rc.2 --locked
```

Its flags are listed in [the CLI reference](cli_reference.md#cargo-rullst-footprint).

```bash
# Build the release binary, time a few cold starts, measure the last one, stop it
cargo rullst footprint

# Also measure a local container image and run a 10-second benchmark
cargo rullst footprint --image my-app:latest --bench --duration 10s

# Measure an app that is already running on loopback, and weigh its home page
cargo rullst footprint --url http://127.0.0.1:3000 --page-weight --bench

# Weigh the home page of the published site, as visitors download it
cargo rullst footprint --url https://app.example --page-weight

# Add an SCI estimate from your own power, network and grid figures
cargo rullst footprint --bench --cpu-watts 15 --network-kwh-per-gb 0.059 --grid-intensity 120 --json

# Fail when a figure exceeds its limit in Rullst.toml [footprint.budgets]
cargo rullst footprint --env staging --bench --page-weight --check-budget

# Show the bytes production served per route (token from the environment)
RULLST_MONITOR_TOKEN=... cargo rullst footprint --page-weight --bench --monitor-url https://app.example
```

## What is measured

| Figure | Method | Notes |
| :--- | :--- | :--- |
| Binary size | file size of the release binary built by `cargo build --release`; with `--url`, of the running executable (`/proc/<pid>/exe` on Linux, `proc_pidpath` on macOS, `QueryFullProcessImageNameW` on Windows) | Size on disk, not memory use. |
| Image size | `docker image inspect --format {{.Size}}` (then `podman`) of the image you name with `--image` | Uncompressed size reported by the local engine. Only on a local socket (`DOCKER_HOST`/`CONTAINER_HOST` unset or `unix://`/`npipe://`); otherwise, or without `--image`, `NOT MEASURED` with the reason. |
| Idle RSS | resident memory after start-up, before any page-weight or benchmark request: `VmRSS` from `/proc/<pid>/status` (Linux), the resident size from `proc_pidinfo(PROC_PIDTASKINFO)` (macOS), the working set from `GetProcessMemoryInfo` (Windows) | The method names the source on each platform. |
| Start-up time (13.0.0-alpha.6) | median of up to five cold starts of the release binary footprint starts, each from spawning the process to the first HTTP response (any status) to `GET --path`, probed every 5 ms | Only for the binary footprint starts: an app found with `--url` was already running and a published site has no local process, so both read `NOT MEASURED`. See [Start-up time](#start-up-time). |
| Dependencies | packages with a registry or git `source` in the nearest `Cargo.lock` | Every third-party crate Cargo resolved, for all targets and features; path and workspace packages are not counted. |
| Compiled dependencies (13.0.0-alpha.6) | the packages of the Dependencies count that `cargo tree --locked --offline --edges normal,build --prefix none --format {p}` lists in the project directory, for its package, each `name version` once | What Cargo compiles for the release build footprint measures: the host platform, the default features and the same feature resolver as `cargo build`, with proc macros and build-script dependencies, without dev-dependencies, other platforms' dependencies or optional dependencies no enabled feature needs, all of which `Cargo.lock` also lists. Run footprint from the project directory (the one with `Cargo.toml`); `cargo tree` reads only the local index and lock file, so a project that was never built, or whose `Cargo.lock` is out of date, reads `NOT MEASURED` with the reason. |
| CPU time (with `--bench`) | user + system CPU time of the app process over the benchmark: `utime + stime` from `/proc/<pid>/stat` (Linux), `proc_pidinfo` (macOS, converted from Mach time units), `GetProcessTimes` (Windows) | All threads of the process, not child processes. |
| CPU per request (with `--bench`) | CPU time ÷ completed requests, in milliseconds | Responses of any status count. |
| Bytes per request (with `--bench`) | response body bytes ÷ completed requests; every request sends `Accept-Encoding: zstd, br, gzip` and bodies are counted as served, never decompressed | Bodies only: HTTP headers, TLS and TCP overhead are not counted. The report lists the `Content-Encoding` values the app sent. |
| Page weight (with `--page-weight`) | see [Page weight](#page-weight) | First and repeat visit, requests and third-party origins. |

A value that could not be measured reads `NOT MEASURED` and keeps the reason.
Nothing else is reported: there is no throughput score, energy reading or
rating.

Idle memory and CPU time are measured on Linux, macOS and Windows, for the app
footprint starts and for an app found with `--url`. footprint finds the
process listening on the `--url` port by matching the socket in
`/proc/net/tcp{,6}` with `/proc/<pid>/fd` (Linux), with
`lsof -t -iTCP:<port> -sTCP:LISTEN` (macOS) or with `GetExtendedTcpTable`
(Windows); it only reads your own processes. On other platforms those figures
are `NOT MEASURED` with the reason.

In Git Bash (MSYS) on Windows, an argument that starts with `/` is converted
to a Windows path, so `--path /` reaches footprint as `C:/Program Files/Git/`
and is refused. Turn the conversion off for the command:

```bash
MSYS_NO_PATHCONV=1 cargo rullst footprint --path /
```

## What happens during a run

1. **Target.** Without `--url`, footprint runs `cargo build --release` (Cargo
   reuses an up-to-date build), starts the binary from the project root with
   `RULLST_ENV=production`, `HOST=127.0.0.1` and `PORT=<free port>`, and waits
   up to 60 seconds for the first HTTP response to `GET --path`. It starts the
   binary up to five times, each on a new free port and stopped before the
   next, to time its [start-up](#start-up-time), and measures the last start.
   It stops the app with SIGTERM (then a kill after 5 seconds) when the run
   ends.
   Production refuses to start without production settings, such as the SaaS
   starter's real billing credentials; set them, or pass `--env staging`,
   which accepts the starters' mock credentials and is recorded in the report.
   With `--url`, it measures an app that is already running. An `http://`
   URL must be on a loopback address (`127.0.0.0/8`, `::1` or `localhost`,
   which is pinned to `127.0.0.1`); an `https://` URL is a published site,
   whose page alone is weighed ([A published site](#a-published-site));
   anything else is refused before a request is sent.
2. **Idle memory**, read right after the app answers.
3. **Page weight (only with `--page-weight`)**, described below.
4. **Benchmark (only with `--bench`).** `--concurrency` connections (default
   4) each send `GET --path` (default `/`), wait for the whole response body,
   then send the next request, until `--duration` (default 10 s, 1 s to
   10 min) ends. Proxies are ignored and redirects are not followed, so no
   request leaves loopback. The load generator runs on the same machine; its
   own CPU use is not part of the app's CPU time.
5. **Report.** Terminal tables (plain text under `NO_COLOR`, `CI` or when
   stdout is not a terminal) or, with `--json`, the versioned
   `rullst.footprint.v1` document.

When the release build fails, or the app exits or does not answer within 60
seconds on any of its starts (a database it cannot reach, missing production settings), or nothing
answers on `--url`, footprint still prints the report, JSON included, with the
figures that need no running app: the dependency count, the image size and,
once a binary was built, its size. The others are `NOT MEASURED` with the
reason, the report's `failure` says what failed, and the error follows on
stderr with a fix chosen from the app's own error (its last stderr lines) and
the first lines of a failed build's log.

The command makes no network calls beyond loopback, the local container
engine, the published site's `https://` origin when `--url` names one, and,
only with `--monitor-url`, the application's monitor, and never fetches
carbon intensity data. The release build itself may download crates as
any `cargo build` does.

## Start-up time

Since 13.0.0-alpha.6, the report gives how long the release binary takes to
start: the time from spawning the process to the first HTTP response, of any
status, to `GET --path`. On a host that scales to zero, such as Azure
Container Apps, Cloud Run or Knative, the first request after an idle period
waits for this start, so a slow start is latency that visitors see.

- **Cold starts.** footprint starts the binary up to five times, each a new
  process on a new free loopback port, stopped before the next one starts,
  and reports the median, the fastest and the slowest start and every start's
  time. It stops starting once the timed starts add up to 20 seconds, so a
  slow app is started fewer times (at least once); the report says how many
  starts were timed (`runs`). The last start is the app that the other
  figures measure.
- **Resolution.** Readiness is probed every 5 ms, so a time is at most about
  5 ms (plus one loopback request) later than the moment the app could first
  answer. Times are reported to a tenth of a millisecond.
- **What it includes.** Everything the process does before it answers:
  loading the binary, reading configuration, opening the database pool,
  running what `main` runs before `Server` binds, and the first request. Use
  `--path` to time the path your host probes, such as `/ready` or `/health`
  (the page weight and the benchmark use the same path).
- **What it does not include.** On this machine, the binary and the files it
  reads are in the operating system's cache after the first start, and
  nothing is downloaded. A container host also pulls the image, creates the
  container and starts its runtime, which footprint does not measure, so
  compare a host's cold starts only with that host's own figures.
- **When it is not measured.** An app found with `--url` was already running,
  and a published `https://` site has no process here, so both read
  `NOT MEASURED` with the reason; so does a run whose build failed or whose
  app did not start.

The [Rullst/Benchmarks](https://github.com/Rullst/Benchmarks) suite reports a
different figure, `startup_s`: the time a framework's container takes to
answer `GET /text` with `200 OK` once `docker run -d` has returned, probed
every 0.25 s and rounded to hundredths of a second, so starts shorter than
about a quarter of a second cannot be told apart there. footprint times your
own release binary as a local process, without a container, from the spawn
and at a 5 ms probe interval. Do not compare the two figures with each other.

## Page weight

`--page-weight` loads the `--path` page as a browser with an empty cache
would, then again as a browser that kept what the first visit cached:

- **What is loaded.** The page, then the same-origin resources it references:
  stylesheets (`<link rel=stylesheet>`, with their `url()` and `@import`, up to
  three levels), scripts (`<script src>`, `modulepreload`), `preload`ed fonts,
  images and other files, `<img src>` (or the first `srcset` candidate),
  `<video poster>`, the icon and the web app manifest, `url()` in `<style>`
  elements and `style` attributes, and same-origin `<iframe>` documents
  (without their own resources). A redirect is followed when it stays on the
  origin, and each hop counts as a request. Each URL is loaded once. In a
  stylesheet's `src` list (`@font-face`), only the first `url()` is loaded.
- **What is not.** Media sources (`<video>`, `<audio>`, `<source>`),
  `prefetch` hints and links are not resources of the page; scripts' own
  requests (`fetch`, dynamic `import()`, htmx requests) are not run. Every
  `url()` of a stylesheet counts, whether or not an element on the page uses
  it, and `loading="lazy"` images count as loaded. At most 200 responses are
  loaded, each at most 32 MiB (a larger one is an error, left out of the
  totals), at most 256 MiB in all, and at most five redirects in a row, all
  within 180 seconds for both visits; what a bound leaves out is reported as
  `unfetched`.
- **Third parties.** URLs on another origin are listed by origin and counted,
  never fetched: the app's page weight does not include them, and a
  `preconnect` or `dns-prefetch` hint adds only its origin.
- **Bytes.** Response bodies as served, with `Accept-Encoding: zstd, br,
  gzip`, without headers. To find a compressed page's references footprint
  reads it once more without compression; that request is not counted.
- **Repeat visit.** For each response of the first visit, as a browser's
  private cache would: no request while `Cache-Control: max-age` (or, without
  it, `Expires` against `Date`) keeps the response fresh and `no-cache` does
  not ask for revalidation; a conditional request (`If-None-Match`,
  `If-Modified-Since`) when the response has an `ETag` or `Last-Modified`, which
  costs a request and, on `304 Not Modified`, no body; a full request again
  otherwise, and always for `no-store`. Heuristic freshness, which browsers
  may give a response with only `Last-Modified`, is not assumed, so the repeat
  visit never counts more caching than the app asked for.

The report gives first-visit and repeat-visit bytes and requests, how many
repeat responses came from the cache and how many were `304`, the third-party
origins and every loaded resource with its status, `Content-Encoding`, bytes
and what the repeat visit did. Rullst's `/static` mount already sends
`immutable` caching for content-hashed names and `ETag` revalidation for the
others ([static assets](tutorials/10-static-assets-and-compression.md#step-3-cache-headers)).

The figures are what the app serves over loopback: a CDN, reverse proxy or
another compression setting in production changes what users receive (the
app sends pages uncompressed, and since 13.0.0-alpha.6 the `Caddyfile` that
`cargo rullst deploy` writes compresses them with Zstandard or gzip; static
files are pre-compressed), and a
page that differs per user (a session, a nonce) can weigh differently for
each visitor.

### A published site

Since 13.0.0-alpha.6, `--url` also takes the `https://` origin of a published
site, to weigh what a visitor downloads with the server's (or CDN's)
compression:

```bash
cargo rullst footprint --url https://app.example --page-weight --path /pricing
```

- **Page weight only.** It needs `--page-weight` and refuses `--bench`
  (which would load the site and needs the app's CPU time) and `--image`.
  The binary size, idle memory, start-up time, dependency counts and CPU time
  are
  `NOT MEASURED` with the reason `not measured for a published https:// site`,
  the target's `mode` is `remote_url`, and the report's notes say that only
  the page weight was measured.
- **The same counting.** The requests send `Accept-Encoding: zstd, br,
  gzip`, nothing is decompressed, and bytes are bodies as the site sent them,
  without headers, under the bounds above. Only the `https://` origin is
  fetched: a redirect to another host (from the bare domain to `www.`, for
  example) is listed as a third party and not followed, so pass the final
  origin. The origin takes no credentials, path, query or fragment (put the
  path in `--path`).
- **What it is.** One visit from this machine, with no cookies or login: a
  visitor elsewhere, a CDN cache in another state or a signed-in page can
  receive different bytes. The client follows the system's proxy settings,
  connects within 5 seconds, waits at most 10 seconds per response and names
  itself `cargo-rullst-footprint/<version>` in `User-Agent`. A site that
  cannot be reached ends the run with status 1, as an unreachable `--url`
  does.

## What is estimated: SCI

footprint follows the Software Carbon Intensity specification
(ISO/IEC 21031:2024) of the Green Software Foundation:

```text
SCI = ((E × I) + M) / R
```

| Term | Source | Kind |
| :--- | :--- | :--- |
| E | measured CPU time × `--cpu-watts`, plus, with `--network-kwh-per-gb`, the measured response bytes ÷ 10⁹ × that figure, in kWh | estimate: footprint measures no energy |
| I | `--grid-intensity` in gCO2e/kWh | yours, never fetched |
| M | `--embodied` in gCO2e, the share of hardware emissions you allocate to this run | yours; *not included* when absent |
| R | requests completed in the benchmark; the functional unit is one HTTP request | measured |

`--cpu-watts`, `--network-kwh-per-gb` and `--grid-intensity` require `--bench`
(the network figure and the grid intensity also need `--cpu-watts`), so the
estimate is either computed from all of its inputs or not at all. Without
them the SCI row reads `NOT ESTIMATED` and lists the missing inputs. The
`--cpu-watts` figure is your assumption of power per busy CPU second (from the
processor's documentation or your own measurement); the estimate ignores
memory, disk and idle power.

### The network term

`--network-kwh-per-gb` adds the energy of moving the benchmark's response
bytes across networks to E, as `E_network = bytes ÷ 10⁹ × kWh/GB`; the report
shows `cpu_energy_kwh`, `network_energy_kwh` and their sum `energy_kwh`. It is
an allocation, not a measurement, and the result depends entirely on the
figure you choose:

- The figure should cover networks only (access and core); the server's
  energy is already the CPU term, and user devices are outside it. The
  [Sustainable Web Design model](https://sustainablewebdesign.org/estimating-digital-emissions/)
  (version 4) uses **0.059 kWh/GB** of operational energy for networks
  (310 TWh of network energy over 5.29 ZB of data transferred, from IEA 2022
  figures), the value used in the examples above. Aslan, Mayers, Koomey and
  France (*Journal of Industrial Ecology*, 2018) estimated 0.06 kWh/GB for
  fixed-line transmission in 2015 and found the figure halving about every
  two years; a value from an older year overstates a current one.
- Network energy is mostly fixed: equipment draws power whether or not your
  bytes pass. Mytton, Lundén and Malmodin
  (["Network energy use not directly proportional to data volume"](https://doi.org/10.1111/jiec.13512),
  *Journal of Industrial Ecology*, 2024) show that kWh per GB averages do not
  describe what one more gigabyte costs. Use the term to compare responses of
  the same app, not to claim a saving.
- Record the figure, its source and year next to the report, like the grid
  intensity. footprint never looks either up, so the same inputs always give
  the same result.

### Choosing a grid intensity

Use a value from a source you can cite for the place and time the software
runs, such as your electricity supplier's disclosure, your grid operator or
energy agency's published average, or a dataset your organisation already
uses. Record the value, the source, the year or hour it describes and whether
it is an average or a marginal figure next to the report.

## Budgets in CI

Limits on the measured figures live in `Rullst.toml`, next to the rest of the
project's configuration (the application ignores the table):

```toml
[footprint.budgets]
binary_size_bytes = 25_000_000
idle_rss_bytes = 64_000_000
dependency_count = 450
compiled_dependency_count = 380
startup_ms = 500
cpu_ms_per_request = 2.0
bytes_per_request = 60_000
page_first_visit_bytes = 500_000
page_repeat_visit_bytes = 40_000
page_requests = 25
page_third_party_origins = 0
```

Every key is optional; `image_size_bytes` is the twelfth. Values are numbers of
0 or more, in bytes, a count or milliseconds as the name says; an unknown key,
a negative value or a string stops the run before anything is built, so a
misspelt limit cannot pass unnoticed. `--check-budget` reads this table;
`--budget FILE` reads the same `[footprint.budgets]` table from another TOML
file instead, for example a stricter budget for the main branch.

After the report, each limit is **passed** (measured at or under the limit),
**exceeded** (above it) or **skipped**: its figure was not measured (no
`--bench` for `cpu_ms_per_request`, no `--page-weight` for the page limits,
no `--image`, an app found with `--url` for `startup_ms`, which checks the
median start-up time, or a platform that cannot read the process), so it is
never counted as passed. The command exits with status 1 when a limit is exceeded,
after printing the report, and names each one with the measured and allowed
values on stderr. The JSON report's `budgets` section lists every limit with
its `allowed`, `measured`, `status` and, when skipped, `reason`.

A CI job that builds the release binary, starts it with the starters' mock
credentials and checks the budget:

```yaml
  footprint:
    name: Footprint budget
    runs-on: ubuntu-latest
    timeout-minutes: 45
    steps:
      - name: Check out the repository
        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - name: Set up Rust with a dependency cache
        uses: actions-rust-lang/setup-rust-toolchain@a188e68d0fd2bd36ad51dccb2b07ba6e75105f30 # v2.0.1
      - name: Install the Rullst CLI
        run: cargo install cargo-rullst --version =13.0.0-rc.2 --locked
      - name: Measure and check the budget
        run: cargo rullst footprint --env staging --bench --duration 5s --page-weight --check-budget --json > footprint.json
      - name: Keep the report
        if: always()
        uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2
        with:
          name: footprint
          path: footprint.json
```

Compare figures only on the same runner type: hosted runners vary, so leave
some headroom in time-based limits such as `cpu_ms_per_request` and
`startup_ms`.

Rullst's own CI budgets the starters this way: the `Starter footprint
budgets` job of the repository's `ci.yml` builds the Blank, SaaS and API
starters in release mode and runs footprint with `--env staging --page-weight
--bench --budget` against one file per starter in `.github/starter-footprint/`.
Their limits are the figures measured on a GitHub-hosted Linux runner plus a
margin (15 % for the binary, 20 % for idle memory, 5 % for dependencies,
10 % but at least 1 KiB for bytes, none for page requests and third-party
origins), and `cpu_ms_per_request` and the start-up time are reported there
but not limited, because they vary from run to run on shared runners. A starter that grows
past a limit fails the job until the growth is removed or the limit is
raised on purpose.

The generated CI workflow of [the testing kit](testing-your-app.md) does not
include this job, because it needs the CLI installed and an application that
starts in CI; add it when your application does.

## Production bytes from the monitor

With `--monitor-url`, footprint also reads the application's monitor
(`GET /_rullst/monitor/v1`, mounted by
[`Server::with_monitor`](production-monitoring.md)) and reports, per route,
the requests and the response bytes it served in production: the bodies as
sent, after compression, since the process started and in the last five
minutes. The route that the `--path` page matches is shown beside the lab's
bytes per request from `--bench`, so a page that is light in the lab but heavy
in production (or the reverse) stands out.

- **The token** comes from `RULLST_MONITOR_TOKEN` (one of the application's
  `RULLST_MONITOR_TOKENS`), never from the command line, so it does not end up
  in shell history or CI logs. It is sent only over HTTPS, or in plain HTTP to
  a loopback address such as an SSH tunnel, and never after a redirect.
- **The URL** is the application's origin (`https://app.example`, the monitor
  path is added) or the full monitor URL; credentials, a query or a fragment
  are refused. Apart from an `https://` `--url`, it is the only address
  outside loopback the command contacts.
- **What is read** is what the monitor publishes: route patterns and counters,
  never request data. An application whose Rullst is older than
  13.0.0-alpha.5 reports no bytes; the section then says so.
- A `401`, `403`, `404` or `429`, an answer that is not a `rullst.monitor.v1`
  document or one larger than 4 MiB leaves the section `NOT MEASURED` with the
  reason; the rest of the report is unaffected.

The JSON report's `production` section has the `source` URL, the
application's name and version, its uptime and window, `bytes_reported`,
`requests_total`, `bytes_total`, `path_route` and every route with
`requests_total`, `bytes_total`, `bytes_per_request`, `window_requests` and
`window_bytes`.

## Why results vary

Numbers depend on the CPU model and its frequency scaling, power profile,
thermal state, operating system, build profile, database contents, the request
path, concurrency, and other processes on the machine (including the load
generator). Memory figures differ by platform: Linux and macOS report the
resident set, Windows the working set. Compare runs only on the same machine
with the same setup, repeat runs and look at the spread, and publish the
report together with its `machine`, `benchmark` and `page_weight` sections.
The report header records the time, OS, architecture, logical CPU count and,
on Linux and macOS, the CPU model.

## JSON report

`--json` prints one document whose `schema` (and `schema_version`) is
`"rullst.footprint.v1"`, with `cli_version`, `measured_at` and the `machine`
section. Every key is always present (null when unknown) so scripts can rely
on the shape:

- `target`: `mode` (`release_build`, `existing_url` or, for a published
  `https://` site, `remote_url`), the URL (`null` when
  the build failed before a port was chosen), PID and how it was found.
- `measured`: `binary_size_bytes`, `image_size_bytes`, `idle_rss_bytes`,
  `dependency_count` and (since 13.0.0-alpha.6) `compiled_dependency_count`,
  each `{status, value, unit, method, reason}` with `status` `measured` or
  `not_measured`.
- `startup` (since 13.0.0-alpha.6): `status` (`measured` or
  `not_measured`), `reason`, `method`, `path`, `runs` (the starts timed, at
  most five), `samples_ms` (each start's time, in the order they ran),
  `median_ms`, `min_ms` and `max_ms`, all in milliseconds.
- `benchmark`: `status` (`run` or `not_run`), its inputs, requests, errors,
  `accept_encoding`, `content_encodings` and, in the same metric shape,
  `cpu_time_seconds`, `cpu_ms_per_request`, `response_bytes` and
  `bytes_per_request`.
- `page_weight`: `status` (`measured`, `not_run` or `not_measured`),
  `reason`, `method`, `path`, `accept_encoding`, `first_visit` and
  `repeat_visit` (`bytes`, `requests`, and for the repeat visit `from_cache`
  and `not_modified`), `resources` (each `url`, `kind`, `status`,
  `content_type`, `content_encoding`, `bytes`, `repeat` — `cache`,
  `not_modified`, `fetched` or `error` — `repeat_bytes` and `error`),
  `third_party_origins`, `third_party_references` and `unfetched`.
- `sci_estimate`: `status` (`estimate` or `not_computed`), `label` (always
  `estimate`), the standard, formula, energy and network methods, every input
  in `inputs` (`network_kwh_per_gb` and `response_bytes` included),
  `energy_kwh`, `cpu_energy_kwh`, `network_energy_kwh`, `sci_g_per_request`
  and `missing_inputs`.
- `production`: `status` (`measured`, `not_run` or `not_measured`),
  `reason` and the monitor's figures, as in
  [Production bytes from the monitor](#production-bytes-from-the-monitor).
- `budgets`: `status` (`not_checked`, `passed`, `exceeded` or `skipped`),
  `source`, the `passed`, `exceeded` and `skipped` counts and every limit, as
  in [Budgets in CI](#budgets-in-ci).
- `notes`: the fixed caveats printed under the tables.
- `failure`: `null` after a measurement; otherwise `stage` (`build`, `start`
  or `reach`), `title`, `message` (at most 20 lines of a failed build's log,
  from its first error) and `fix`.

## Exit status

`0` when the measurement ran, even with `NOT MEASURED` fields or skipped
limits; `1` when the release build failed or the app could not be started or
reached (after printing the report with `failure`), when a budget limit was
exceeded (after printing the report), or when the budget file cannot be used
(before measuring), or when an `https://` `--url` comes without
`--page-weight` or with `--bench` or `--image` (before any request); `2` for
invalid arguments, including a non-loopback `http://` `--url`, a plain-HTTP
`--monitor-url` or an estimate input without `--bench`. A published site that
cannot be reached also exits with `1`, after the report.

## Changes from `green` and the earlier `footprint`

13.0.0-alpha.5 renamed `cargo rullst green` (13.0.0-alpha.4) to
`cargo rullst footprint`, without an alias: "green" read as an environmental
claim, and the command measures resources. The flags are unchanged; the JSON
schema is `rullst.footprint.v1` instead of `rullst.green.v1` and adds
`page_weight`, the benchmark's bytes and the network term. `cargo rullst
upgrade` points at scripts, workflows, container files, Makefiles and
documentation that still run `green` (`V13-CLI-GREEN-RENAMED`).

The `footprint` command of 13.0.0-alpha.1 to alpha.3 was a different report:
it gave requests per second, latency percentiles and peak RSS, read RAPL
energy counters (whole package, including other processes), guessed an image
name from the package name and wrote `rullst.cli-footprint.v1`. None of that
returns.
