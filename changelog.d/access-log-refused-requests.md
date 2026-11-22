### Fixed

- Requests that one of `Server`'s own layers answered before the
  application's handler (the CSRF check, the authentication of
  `Server::with_machine_endpoints` routes, the WAF in staging and production,
  the rate limiter of `Server::rate_limit`, Traffic Shield and lifecycle
  admission) reached no access-log line, because the access log runs inside
  those layers; since 13.0.0-alpha.6 this also hid refused form submissions
  in development. They now get a line with their status and the layer, such
  as `[HTTP] POST /login -> 403 (0.31 ms) refused by csrf`, in every
  environment and in the hot-reload server of `cargo rullst dev`: each layer
  marks its response, and a layer outside them writes the line, with the
  usual path, `RULLST_ACCESS_LOG` and health-probe rules and without the
  request's body or headers. A refusal inside the application's own router
  is written once (#845).
