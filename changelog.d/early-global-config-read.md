### Fixed

- `RullstConfig::global()` read before `Server` starts (as code generated before 13.0.0-rc.1 did)
  returned the defaults and kept them, so `[app] env` and every other `Rullst.toml`
  setting read afterwards was lost without a message. `Server` now replaces those
  defaults with the loaded configuration, so later reads see it, and logs a warning that
  names the early read when the two differ. It also warns when an earlier
  `RullstConfig::set_global` installed a configuration that differs from `Rullst.toml`
  (that one stays global). The configuration structs now implement `PartialEq`.
