### Fixed

- The website (rullst.win, English and Brazilian Portuguese) no longer calls
  13.0.0-rc.1 "not yet released" and gives its publication date, 26 November
  2026. It no longer counts the access log among the layers whose cost the
  benchmark measures: the access log is on by default, but Rullst/Benchmarks
  turns per-request logging off for every implementation. It describes the
  gap between the two Rullst rows as roughly the cost of `Server`'s
  production stack, `Server` itself included, and says that since rc.1
  Rullst's pool pings a PostgreSQL connection only after 500 ms idle while
  the Axum reference pings on every acquisition. The footer's "Benchmarks"
  link opens the cross-framework results; the microbenchmark history has its
  own link. The comparison page, checked again on 27 November 2026, adds Loco
  1.3's database and Redis session stores and its other default middleware,
  says how `loco new` picks a template and what Loco's CI does with the
  generated Docker image, narrows what Topcoat lacks to user accounts,
  password handling and a built-in health endpoint, and updates the GitHub
  star counts.
