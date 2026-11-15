### Fixed

- `rullst-auth`: Cargo 1.100 writes test and bench binaries to
  `target/<profile>/build/<package>/<hash>/out/<name>-<hash>` instead of
  `target/<profile>/deps`. The development `APP_KEY` resolution now treats
  both locations as a test harness, so `cargo test` on Cargo 1.100 again keeps
  a generated key in memory instead of writing it to the per-user state
  directory.
