### Changed

- The [compatibility policy](https://rullst.github.io/Rullst/book/compatibility-policy.html#minimum-supported-rust-version)
  makes MSRV changes in 13.x predictable: a minor release raises the MSRV only
  when a dependency, a security fix or a concrete need requires it, and only
  to a Rust release published at least six months before that minor release.
  Patch releases still do not raise it, and a raise still needs a changelog
  entry, updated manifests and documentation and a green MSRV job. The policy,
  Tutorial 01 and the REST API quickstart now name the latest stable Rust as
  the recommended toolchain; Rust 1.96.0 stays the minimum.
- A new informational workflow, `msrv-tests.yml`, runs every night and on
  demand the unit and integration tests of Core, the ORM on SQLite, Security,
  Auth, the two macro crates and the `rullst` facade, and the facade's
  doctests, on Rust 1.96.0. The required MSRV job only compiles the workspace.
