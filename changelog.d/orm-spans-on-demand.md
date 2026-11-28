### Changed

- Core: the tracing subscriber that `Server::run` and
  `telemetry::init_telemetry` install creates the ORM's `rullst.orm.*` spans
  only once something in the process records them: the development dashboard
  endpoint or the production monitor's recording layer is mounted, or, in the
  `telemetry` build, from the start. Until then an ORM call creates no span,
  formats no span fields and is not entered again on every poll of SQLx's
  streams, so database requests no longer pay for spans nothing reads. An
  event logged during such an ORM call (for example SQLx's warning about a slow
  connection acquisition) is written without its `rullst.orm.query{...}:`
  prefix. `RUST_LOG` selects the same events as before, a `RUST_LOG` with a
  span directive (such as `info,[rullst.orm.query]=debug`) keeps every ORM
  span, and applications that install their own subscriber are not affected.
