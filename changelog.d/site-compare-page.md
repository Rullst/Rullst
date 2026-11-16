### Added

- `https://rullst.win/compare/` (and `/pt-br/comparar/`) compares Rullst, Loco
  and Topcoat topic by topic: rendering, database, authentication and security
  defaults, background jobs, mail, billing, admin and observability, CLI,
  deployment, stability and support window, and maturity. Each Loco and
  Topcoat fact names the version it was checked against (`loco-rs 1.3.1`,
  `topcoat 0.12.1`, 14 November 2026). Its benchmark rows are rendered at site
  build time from the Rullst/Benchmarks results, and say that the first
  measurement is in progress until those results exist. A "How to read these
  numbers" block ("Como ler estes números") above the results says what
  1.00× Axum means, which direction is better for each figure (responses per
  second higher, latency and memory lower), why the ratio is compared rather
  than the absolute number (each framework ran on its own hosted machine) and
  what the two Rullst rows are. A `Rullst (minimal)` card ("Rullst
  (mínimo)") follows the Rullst one: the framework without the generated
  application's security layers, beside the application as
  `cargo rullst new` generates it; it is left out when the results have no
  minimal row.
