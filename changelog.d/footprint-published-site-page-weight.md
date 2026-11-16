### Added

- `cargo rullst footprint --page-weight` accepts the `https://` origin of a
  published site (`--url https://app.example --page-weight`), to weigh what a
  visitor downloads with the server's or CDN's compression: the same
  `Accept-Encoding: zstd, br, gzip` requests, bodies counted as sent, first
  and repeat visits. Only the page is weighed: an `https://` URL needs
  `--page-weight`, refuses `--bench` and `--image`, and reports the binary
  size, idle memory, dependency counts and CPU time as not measured, with
  target mode `remote_url` and its own report notes. Credentials, a path, a
  query or a fragment in the URL are still refused, a redirect to another
  host is listed as a third party and not followed, and a site that cannot
  be reached exits with status 1. Every page weighing now bounds one response
  at 32 MiB, a visit at 256 MiB, redirects at five in a row and both visits
  at 180 seconds, reporting what a bound leaves out as `unfetched`. Requested
  by the plataforma, vene.win and capoeira applications.
