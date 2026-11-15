### Added

- The production monitor serves a second document next to `rullst.monitor.v1`:
  `GET /_rullst/monitor/v2` (`rullst.monitor.v2`, `MONITOR_V2_PATH`) has every
  version 1 field plus `traces` and `security`, with the same credentials,
  limits and headers. `traces` lists up to 50 sampled request summaries
  (method, route pattern, status, duration, ORM operations, possible N+1; at
  most one ordinary request per 100 ms and one `5xx` or slow request per
  10 ms) and per-kind totals of the `SpanCollector`, never span names.
  `security` reports the `SecurityStore` counters, the active honeypot ban
  count, the events recorded since start and the type and time of the kept
  events, never their details or client addresses. Version 1 is unchanged.
- The standalone Studio reads `rullst.monitor.v2` and has two new project
  pages: **Traces** (`/studio/traces`, the sampled requests and span totals)
  and **Security** (`/studio/security`, the security radar). For an
  application before 13.0.0-alpha.6 it falls back to `rullst.monitor.v1` when
  version 2 answers `404`, remembers that for ten minutes, and the two pages
  say that the application serves version 1 only.
