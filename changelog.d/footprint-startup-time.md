### Added

- `cargo rullst footprint` reports the start-up time of the release binary it
  starts, in the terminal ("Startup time") and in JSON (`startup`, an
  additive section of `rullst.footprint.v1`): it starts the binary up to five
  times, each a new process on a new loopback port stopped before the next,
  times each start from the spawn to the first HTTP answer to `GET --path`
  (any status, probed every 5 ms), and gives the median, the fastest and the
  slowest start, every start's time and how many starts it timed (`runs`).
  It stops starting once the timed starts add up to 20 seconds, so a slow app
  is started fewer times, and measures the last start. A scale-to-zero host,
  such as Azure Container Apps, adds this time to the first request after an
  idle period. An app found with `--url` and a published `https://` site
  have no start-up time and read `NOT MEASURED` with the reason.
  `[footprint.budgets]` accepts a `startup_ms` limit on the median; the
  starter footprint job of CI reports the figure without limiting it. The
  figure is not the `startup_s` of the Rullst/Benchmarks suite, which times
  containers at a 0.25 s probe interval.
