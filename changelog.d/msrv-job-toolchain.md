### Fixed

- **MSRV erratum:** Rust 1.96.0, the minimum supported Rust version that
  13.0.0-alpha.1 through 13.0.0-alpha.5 declare (`rust-version = "1.96.0"`),
  was not verified for any of them. The required `Check MSRV (Rust 1.96.0)`
  CI job installed 1.96.0 with `rustup default`, but `rust-toolchain.toml`
  overrides the default, so the job compiled the workspace with the
  repository's pinned toolchain instead (Rust 1.99.0 for alpha.1, 1.99.1 for
  alpha.5), as it had since the file was added with Rust 1.97.1 on
  25 July 2026. Since PR #709 the job forces 1.96.0 with `RUSTUP_TOOLCHAIN`
  and fails unless `rustc` reports that version, and the new daily
  `msrv-tests.yml` workflow runs the central crates' tests on Rust 1.96.0.
  The first check with Rust 1.96.0 ran on the 13.0.0-alpha.6 source and
  passed; the published alphas were not rechecked.
