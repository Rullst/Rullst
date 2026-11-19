### Added

- `cargo rullst upgrade` reports, for review, a struct literal or tuple-struct
  call with the name and fields of a Rullst type that became
  `#[non_exhaustive]` in a file that names no Rullst crate but imports a glob
  (`use super::*;`), which can bring the type in unnamed
  (`V13-NON-EXHAUSTIVE-LITERAL-GLOB`). `V13-DEPLOY-CADDY-ENCODE` and
  `V13-DEPLOY-CADDY-HEALTH` also read a `Caddyfile` in `deploy/`, `ops/`,
  `infra/` or `docker/`, besides the project root.
- The alpha.5 to alpha.6 checklist maps chrono's methods to jiff's.
