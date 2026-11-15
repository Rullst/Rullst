### Changed

- The staging and production stack of `Server` serves 1.4 to 1.8 times as
  many requests. In the Rullst/Benchmarks comparison a generated application
  answered `GET /json` at 0.21 times the requests per second of Axum on the
  same machine; on GitHub runners the same application went from 0.26 to
  0.45 times the throughput of its routes without any layer on a slower
  runner (29 300 to 52 000 requests per second, p99 6.5 ms to 3.6 ms) and
  from 0.38 to 0.53 on a faster one (89 000 to 123 000), as the new
  `Generated stack throughput` job of `bench.yml` measures and now requires. The security
  baseline (secure headers, secure framework cookies, WAF, CSRF) runs as one
  middleware instead of seven layers, in the same order and with the same
  responses (two around a CORS layer); the header values derived from
  `[security]` are computed once instead of for every response, the WAF no
  longer allocates to compare an ASCII path or `User-Agent` without case, the
  access log and `Vary: HX-Request` share one layer, the access log is
  written by a background thread, and only HTML responses render a CSP with
  the request's nonce. Every layer stays on by default.
