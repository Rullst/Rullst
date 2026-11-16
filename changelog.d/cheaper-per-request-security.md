### Changed

- A page or API request costs less in a generated application. The three
  middlewares of the `static/` mount (pre-compressed siblings, the cache
  policy and content-hashed names) wrap only `/static` instead of every
  route; the CSP nonce of a request is drawn only when a handler reads it or
  the response is an HTML document; the CSRF mask of each response comes
  from a ChaCha12 generator per thread, seeded and reseeded from the
  operating system, instead of a system call per response, while the cookie
  token is minted from the operating system's random source (before, from
  `rand`'s thread generator); the CSRF cookie and the access-log line are
  built without float or `format!` work. A random-source failure still
  answers `500` without any token, and every security layer stays on. On an
  AMD EPYC 7763 runner the Blank starter's `GET /json` went from 0.44 to
  0.53 times the throughput of its routes without any layer (53,700 to
  63,200 requests per second, 37.6 to 31.3 µs of server CPU per request), as
  the `Generated stack throughput` job of `bench.yml` measures, now with a
  `static/` folder as in a generated project.
