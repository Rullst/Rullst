### Changed

- The `auth_middleware` that `cargo rullst make:auth` and the SaaS starter
  generate inserts the account it loaded for the session check as
  `Extension<User>`, and the generated dashboard renders it instead of
  querying the account a second time. A signed-in request now runs the
  session lookup and one account query; the session row is still read on
  every request, so a revoked session fails on its next one. With this and
  SQLite pools that no longer ping a connection before each query, the SaaS
  starter's signed-in `GET /dashboard` serves about 1.5 times as many
  requests (5,700 to 8,500 per second on an AMD EPYC 7763 runner, 350 to
  234 µs of server CPU per request). The `Generated stack throughput` job of
  `bench.yml` now measures that page and requires its ratio to the minimal
  stack in `.github/stack-throughput/budget.toml`.
