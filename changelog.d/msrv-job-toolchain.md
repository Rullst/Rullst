### Fixed

- The required `Check MSRV (Rust 1.96.0)` CI job compiled the workspace with
  the repository's pinned compiler, not Rust 1.96.0: `rust-toolchain.toml`
  takes precedence over the `rustup default` the job set, so the declared
  MSRV went unchecked from July 2026, when the pin was added (Rust 1.97.1, now
  1.100.0). The job now selects 1.96.0 with `RUSTUP_TOOLCHAIN` and fails
  unless `rustc` reports that version.
