### Changed

- The CI workflow that `cargo rullst new` writes says, above its Rust setup
  step, that every Cargo and compiler warning fails the job (the
  `setup-rust-toolchain` default `CARGO_BUILD_WARNINGS=deny`, which it
  keeps), and that since Cargo 1.100 this includes a dependency that no
  target uses (`cargo::unused_dependencies`). It names the fix: remove the
  dependency, or allow the lint in `[lints.cargo]` when the dependency is
  intentional.
  [Testing your application](https://rullst.github.io/Rullst/book/testing-your-app.html)
  and Getting Started say the same.
