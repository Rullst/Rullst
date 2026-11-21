### Fixed

- The Performance section of the comparison page (`https://rullst.win/compare/`
  and `/pt-br/comparar/`) shows the current Rullst/Benchmarks results again: a
  table of the overall and category scores (Hello world, Database,
  Application, Memory, Startup) with position ranges, tie margins, the stability
  result and every flag explained (‡ included), for Rullst as `cargo rullst new`
  generates it, Loco, Topcoat, Actix Web and Gin, plus Rullst (minimal) as a
  diagnostic; the RAM under load and after the logins; and the generated "What
  this means for Rullst" reading in both languages. It reads the compact
  `results/site.json` (schema `rullst.benchmarks.site.v2`). The page had kept
  the 15 November 2026 measurement of 13.0.0-alpha.5. Data with an unknown
  schema, flag, category or status, or measured more than 90 days earlier, now
  leaves the "no current results" text instead of older numbers. The landing
  page no longer dates the cross-framework comparison to June 2026.
