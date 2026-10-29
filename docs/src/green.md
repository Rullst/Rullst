# Efficiency Figures for an App (`cargo rullst green`)

**Status:** experimental command of the main CLI, `cargo-rullst`
(13.0.0-alpha.4 and later; it replaces `cargo rullst footprint`). Part of the
[green software roadmap](green-software-roadmap.md).

`cargo rullst green` reports efficiency figures it **measured** for your
application, says how it measured each one, and says which ones it could not
measure. It measures no energy. A carbon figure appears only as a Software
Carbon Intensity (SCI) **estimate**, computed from a power figure and a grid
intensity that you supply, and it is labelled `estimate` in the text and the
JSON together with every input. The report describes one run on one machine;
it is not a certification and it does not compare your application with other
software.

## Install

`green` is part of the CLI; nothing else to install:

```bash
cargo install cargo-rullst --version =13.0.0-alpha.4 --locked
```

Its flags are listed in [the CLI reference](cli_reference.md#cargo-rullst-green).

```bash
# Build the release binary, start it on a free loopback port, measure, stop it
cargo rullst green

# Also measure a local container image and run a 10-second benchmark
cargo rullst green --image my-app:latest --bench --duration 10s

# Measure an app that is already running on loopback
cargo rullst green --url http://127.0.0.1:3000 --path /health --bench

# Add an SCI estimate from your own power figure and grid intensity
cargo rullst green --bench --cpu-watts 15 --grid-intensity 120 --json
```

## What is measured

| Figure | Method | Notes |
| :--- | :--- | :--- |
| Binary size | file size of the release binary built by `cargo build --release` (or of `/proc/<pid>/exe` with `--url`) | Size on disk, not memory use. |
| Image size | `docker image inspect --format {{.Size}}` (then `podman`) of the image you name with `--image` | Uncompressed size reported by the local engine. Only on a local socket (`DOCKER_HOST`/`CONTAINER_HOST` unset or `unix://`/`npipe://`); otherwise, or without `--image`, `NOT MEASURED` with the reason. |
| Idle RSS | `VmRSS` from `/proc/<pid>/status` after start-up, before any benchmark request | Linux only. |
| Dependencies | packages with a registry or git `source` in the nearest `Cargo.lock` | Every third-party crate Cargo resolved, for all targets and features; path and workspace packages are not counted. |
| CPU time (with `--bench`) | `utime + stime` delta of the app process over the benchmark, from `/proc/<pid>/stat` and the clock tick rate | Linux only; all threads of the process, not child processes. |
| CPU per request (with `--bench`) | CPU time ÷ completed requests, in milliseconds | Responses of any status count. |

A value that could not be measured reads `NOT MEASURED` and keeps the reason.
Nothing else is reported: there is no throughput score, energy reading or
rating.

## What happens during a run

1. **Target.** Without `--url`, green runs `cargo build --release` (Cargo
   reuses an up-to-date build), starts the binary from the project root with
   `RULLST_ENV=production`, `HOST=127.0.0.1` and `PORT=<free port>`, and waits
   up to 60 seconds for the first HTTP response to `GET --path`. It stops the
   app with SIGTERM (then a kill after 5 seconds) when the run ends.
   Production refuses to start without production settings, such as the SaaS
   starter's real billing credentials; set them, or pass `--env staging`,
   which accepts the starters' mock credentials and is recorded in the report.
   With `--url`, it measures an app that is already running. The URL must be
   `http://` on a loopback address (`127.0.0.0/8`, `::1` or `localhost`, which
   is pinned to `127.0.0.1`); anything else is refused before a request is
   sent. green finds the process listening on the port by matching the socket
   in `/proc/net/tcp{,6}` with `/proc/<pid>/fd`; it only sees your own
   processes.
2. **Benchmark (only with `--bench`).** `--concurrency` connections (default
   4) each send `GET --path` (default `/`), wait for the whole response body,
   then send the next request, until `--duration` (default 10 s, 1 s to
   10 min) ends. Proxies are ignored and redirects are not followed, so no
   request leaves loopback. The load generator runs on the same machine; its
   own CPU use is not part of the app's CPU time.
3. **Report.** Terminal tables (plain text under `NO_COLOR`, `CI` or when
   stdout is not a terminal) or, with `--json`, the versioned
   `rullst.green.v1` document.

The command makes no network calls beyond loopback and the local container
engine, and never fetches carbon intensity data. The release build itself may
download crates as any `cargo build` does.

## What is estimated: SCI

green follows the Software Carbon Intensity specification
(ISO/IEC 21031:2024):

```text
SCI = ((E × I) + M) / R
```

| Term | Source | Kind |
| :--- | :--- | :--- |
| E | measured CPU time × `--cpu-watts`, in kWh | estimate: green measures no energy |
| I | `--grid-intensity` in gCO2e/kWh | yours, never fetched |
| M | `--embodied` in gCO2e, the share of hardware emissions you allocate to this run | yours; *not included* when absent |
| R | requests completed in the benchmark; the functional unit is one HTTP request | measured |

`--cpu-watts` and `--grid-intensity` require `--bench` (and each other in that
order), so the estimate is either computed from all of its inputs or not at
all. Without them the SCI row reads `NOT ESTIMATED` and lists the missing
inputs. The `--cpu-watts` figure is your assumption of power per busy CPU
second (from the processor's documentation or your own measurement); the
estimate ignores memory, disk, network and idle power.

### Choosing a grid intensity

Use a value from a source you can cite for the place and time the software
runs, such as your electricity supplier's disclosure, your grid operator or
energy agency's published average, or a dataset your organisation already
uses. Record the value, the source, the year or hour it describes and whether
it is an average or a marginal figure next to the report. green never looks it
up, so the same inputs always give the same result.

## Why results vary

Numbers depend on the CPU model and its frequency scaling, power profile,
thermal state, operating system, build profile, database contents, the request
path, concurrency, and other processes on the machine (including the load
generator). Compare runs only on the same machine with the same setup, repeat
runs and look at the spread, and publish the report together with its
`machine` and `benchmark` sections. The report header records the time, OS,
architecture, logical CPU count and CPU model.

## JSON report

`--json` prints one document whose `schema` (and `schema_version`) is
`"rullst.green.v1"`, with `cli_version`, `measured_at` and the `machine`
section. Every key is always present (null when unknown) so scripts can rely
on the shape:

- `target`: `mode` (`release_build` or `existing_url`), the URL, PID and how it
  was found.
- `measured`: `binary_size_bytes`, `image_size_bytes`, `idle_rss_bytes` and
  `dependency_count`, each `{status, value, unit, method, reason}` with
  `status` `measured` or `not_measured`.
- `benchmark`: `status` (`run` or `not_run`), its inputs, requests, errors,
  and `cpu_time_seconds` and `cpu_ms_per_request` in the same metric shape.
- `sci_estimate`: `status` (`estimate` or `not_computed`), `label` (always
  `estimate`), the standard, formula and energy method, every input in
  `inputs`, `energy_kwh`, `sci_g_per_request` and `missing_inputs`.
- `notes`: the fixed caveats printed under the tables.

## Exit status

`0` when the measurement ran, even with `NOT MEASURED` fields; `1` when the
release build failed or the app could not be started or reached; `2` for
invalid arguments, including a non-loopback `--url` or an estimate input
without `--bench`.

## Changes from `footprint`

`cargo rullst footprint` (13.0.0-alpha.1 to alpha.3) is now `cargo rullst
green`, in the same `cargo-rullst` package. It no longer reports requests per second,
latency percentiles or peak RSS, no longer reads RAPL energy counters (whole
package, including other processes) and no longer guesses an image name from
the package name. The benchmark runs only with `--bench`, and the JSON schema
is `rullst.green.v1` instead of `rullst.cli-footprint.v1`.
