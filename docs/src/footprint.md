# Measuring an App's Footprint (`cargo rullst footprint`)

**Status:** v13 development source (`13.0.0-alpha.1`). Part of the
[green software roadmap](green-software-roadmap.md).

`cargo rullst footprint` runs a short, bounded load against your application
and prints what it measured, **how** it measured each number, and what it could
not measure. Estimates are labelled as estimates and show their inputs. The
report describes one run on one machine; it is not a certification and it does
not compare your application with other software.

```bash
# Build the release binary, start it on a free loopback port, measure, stop it
cargo rullst footprint --duration 10s

# Measure an app that is already running on loopback
cargo rullst footprint --url http://127.0.0.1:3000 --path /health

# Add an energy estimate and an SCI figure from your own inputs
cargo rullst footprint --cpu-watts 15 --grid-intensity 120 --json
```

## What happens during a run

1. **Target.** Without `--url`, footprint runs `cargo build --release` (Cargo
   reuses an up-to-date build), starts the binary from the project root with
   `RULLST_ENV=production`, `HOST=127.0.0.1` and `PORT=<free port>`, and waits
   up to 60 seconds for the first HTTP response. It stops the app with SIGTERM
   (then a kill after 5 seconds) when the run ends. With `--url`, it measures an
   app that is already running. The URL must be `http://` on a loopback address
   (`127.0.0.0/8`, `::1` or `localhost`, which is pinned to `127.0.0.1`);
   anything else is refused before a request is sent.
2. **Load.** `--concurrency` connections (default 4) each send `GET --path`
   (default `/`), wait for the whole response body, then send the next request,
   until `--duration` (default 10 s, 1 s to 10 min) ends. This is a *closed
   loop*: throughput is what the app sustains at that concurrency, not an open
   arrival rate. Proxies are ignored and redirects are not followed, so no
   request leaves loopback.
3. **Report.** Terminal tables (plain text under `NO_COLOR` or when stdout is
   not a terminal) or, with `--json`, the versioned `rullst.cli-footprint.v1`
   document.

The command makes no network calls beyond loopback and never fetches carbon
intensity data. The release build itself may download crates as any
`cargo build` does.

## What each number means

| Number | Method | Notes |
| :--- | :--- | :--- |
| Requests/s | completed responses ÷ wall time of the load window | Responses of any status count; in-flight requests finish after the deadline and are included. |
| Latency p50/p95/p99 | nearest rank over every completed request, from send to last body byte | Measured by the load generator, so it includes loopback and client overhead. |
| Errors | transport failures (refused, timeout after 10 s, truncated body) + responses with status ≥ 400 | Shown separately in the JSON (`transport_errors`, `http_errors`). |
| CPU time | `utime + stime` delta of the app process over the load window, from `/proc/<pid>/stat` and the clock tick rate | Linux only; all threads of the process, not child processes. |
| Peak RSS | `VmHWM` from `/proc/<pid>/status` | Peak resident memory since the process started, so start-up is included. |
| Idle RSS | `VmRSS` just before the load | Linux only. |
| Binary size | file size of the release binary (or `/proc/<pid>/exe` with `--url`) | Size on disk, not memory use. |
| Docker image | `docker image inspect` of an image named after the package (lowercased, as `make:k8s` names it) | Only on a local Docker socket; otherwise `NOT MEASURED`, never an error. |
| Energy | RAPL package counters, or an estimate, or `NOT MEASURED` | See below. |
| SCI | `((E × I) + M) / R` | Only when E is known and you pass `--grid-intensity`. |

With `--url`, footprint finds the process listening on the port by matching
the socket in `/proc/net/tcp{,6}` with `/proc/<pid>/fd`. It only sees your own
processes; if the process belongs to another user (for example a container
runtime) the process rows are `NOT MEASURED` with that reason.

## Energy

**Measured (RAPL).** On Linux, footprint reads the package-level powercap zones
(`/sys/class/powercap/intel-rapl:N/energy_uj`; `amd-rapl:N` zones are accepted
too) before and after the load and reports the difference, handling counter
wraparound with `max_energy_range_uj`. The label is *measured (RAPL, whole
package, includes other processes)*: it covers the whole CPU package, so the
load generator, your desktop and anything else running are included. Treat it
as an upper bound for the app and keep the machine otherwise idle.

**Permissions.** Since Linux 5.10, `energy_uj` is readable by root only, a
mitigation for the PLATYPUS power side channel (CVE-2020-8694). Running
footprint as a normal user therefore usually reports energy as `NOT MEASURED`.
On a dedicated benchmark machine you can run it as root, or grant read access
for the measurement and restore it afterwards; doing so re-opens the side
channel the restriction closes.

**Estimate (`--cpu-watts`).** When RAPL is not readable and you pass
`--cpu-watts <W>`, footprint estimates `E = process CPU time (s) × W`. The
label starts with `estimate:` and shows the formula and both values. Choose W
from your processor's documentation or your own measurements of a busy core;
the estimate ignores memory, disk, network and idle power.

Without RAPL and without `--cpu-watts`, energy is `NOT MEASURED`.

## Carbon: Software Carbon Intensity (SCI)

footprint follows the Software Carbon Intensity specification
(ISO/IEC 21031:2024):

```text
SCI = ((E × I) + M) / R
```

| Term | Source in footprint |
| :--- | :--- |
| E | energy over the load window in kWh, measured (RAPL) or estimated (`--cpu-watts`) |
| I | `--grid-intensity` in gCO2e/kWh, always user-provided |
| M | `--embodied` in gCO2e, the share of hardware emissions you allocate to this run; otherwise *not included* |
| R | requests completed in the run; the functional unit is one HTTP request |

The SCI row is computed only when E is known and I is given. Otherwise it reads
`NOT COMPUTED` and names the missing terms. The JSON lists `estimated_terms`
(for example `["E"]` with `--cpu-watts`) and `missing_terms`, and every term
keeps its source.

### Choosing a grid intensity

Use a value from a source you can cite for the place and time the software
runs, such as your electricity supplier's disclosure, your grid operator or
energy agency's published average, or a dataset your organisation already
uses. Record the value, the source, the year or hour it describes and whether
it is an average or a marginal figure next to the report. footprint never
looks it up, so the same inputs always give the same result.

## Why results vary

Numbers depend on the CPU model and its frequency scaling, power profile,
thermal state, operating system, build profile, database contents, the request
path, concurrency, and other processes on the machine (including the load
generator itself). Compare runs only on the same machine with the same setup,
repeat runs and look at the spread, and publish the report together with its
`machine` and `inputs` sections. The report header records the time, OS,
architecture, logical CPU count and CPU model.

## JSON report

`--json` prints one document with `schema_version: "rullst.cli-footprint.v1"`.
Every key is always present (null when unknown) so scripts can rely on the
shape:

- `inputs`: the URL, path, duration, concurrency and the optional
  `grid_intensity_g_per_kwh`, `cpu_watts` and `embodied_g`.
- `target`: `mode` (`release_build` or `existing_url`), the URL, PID and how it
  was found.
- `load`: method, elapsed seconds, requests, errors, requests per second and
  `latency_ms.p50/p95/p99`.
- `process` and `artifacts`: each value is `{status, value, unit, method,
  reason}` with `status` `measured` or `not_measured`.
- `energy`: `status` (`measured`, `estimate` or `not_measured`), joules, kWh,
  joules per request, method or reason.
- `carbon`: every SCI term with its source, the result, `estimated_terms` and
  `missing_terms`.
- `notes`: the fixed caveats printed under the tables.

## Exit status

`0` when the measurement ran, even with `NOT MEASURED` fields; `1` when the
release build failed or the app could not be started or reached; `2` for
invalid arguments, including a non-loopback `--url`.
