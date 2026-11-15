### Added

- `cargo rullst footprint` reports the compiled dependency count beside the
  `Cargo.lock` count, in the terminal ("Compiled deps") and in JSON
  (`measured.compiled_dependency_count`, an additive key of
  `rullst.footprint.v1`): the third-party packages of `Cargo.lock` that
  `cargo tree --locked --offline --edges normal,build` lists for the project's
  package, the host platform and the default features, which is what Cargo
  compiles. The lockfile count also holds other platforms' dependencies,
  dev-dependencies and optional dependencies no feature enables. Run outside
  a project directory, or before the project was built, it reads
  `NOT MEASURED` with the reason. `[footprint.budgets]` accepts a
  `compiled_dependency_count` limit, and the starter footprint budgets of CI
  record it.
