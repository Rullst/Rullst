# Build Times

Most of the time spent waiting on a Rullst application is `cargo build` or
`cargo rullst dev` rebuilding it after an edit. This page states what a
generated project does to keep those builds short, the measured effect on the
three starters, and other things you can try.

## What `cargo rullst new` writes

Since 13.0.0-alpha.4 every generated `Cargo.toml` has a development profile
next to the [release profile](cli_reference.md#cargo-rullst-new-name):

```toml
[profile.dev]
debug = "line-tables-only"

[profile.dev.package."*"]
debug = false
```

- **Your package keeps line tables.** Panic messages and the frames of your
  own code in a `RUST_BACKTRACE=1` backtrace still show the file and line
  (`at ./src/main.rs:2:11`). Frames inside dependencies show the function name
  without a file and line.
- **Dependencies are built without debug information.** Their debug
  information was about half of a debug binary and 41 % of `target/`: the API
  starter's binary went from 83.7 MB to 41.5 MB and its `target/` directory
  from 2.47 GB to 1.45 GB.
- **Nothing else changes.** Optimisation level, incremental compilation,
  overflow checks and debug assertions keep Cargo's development defaults.
  `cargo test` uses the same settings (Cargo's `test` profile inherits `dev`),
  and release builds are unchanged.
- **For a debugger** that shows local variables, set `debug = true` in both
  sections (or delete them). Cargo then rebuilds everything once.

The SaaS starter also no longer declares `rullst-connect`: its code does not
use it, and `cargo rullst make:oauth` enables the facade's `oauth-*` feature
instead. Its first build compiles 12 fewer packages.

The generated `.cargo/config.toml` is unchanged: on Linux it sets
`-C split-debuginfo=unpacked` and selects mold or lld when the generating
machine has one, and Git ignores it. With the profile above, split debug
information made no measurable difference on Linux (API starter: 146.0 s
with it and 149.1 s without for a cold build, 1.34 s and 1.40 s for a rebuild
after an edit). macOS already uses unpacked debug information by default and
was not measured.

## Measurements

Measured on 29 October 2026 with the 13.0.0-alpha.4 development source. The
numbers describe these starters on this one machine; they are not a
prediction for other applications or hardware.

- **Applications:** `cargo rullst new <name> --default --skip-initial-migration`,
  plus `--api` or `--blueprint saas` (SQLite; `rullst` features `orm`,
  `strict-sqlite` and `studio`, and `nexus`, `auth`, `mail` and `capital` for
  SaaS), with path dependencies on the framework source and the repository's
  `Cargo.lock`, built `--offline`. *Before* is the same source without the two
  changes above.
- **Toolchain:** Rust 1.99.1, `x86_64-unknown-linux-gnu`, linked by rustc's
  bundled `rust-lld` (LLD 23.1.1; the default for this target since Rust
  1.90). Neither mold nor lld was installed, so the generated
  `.cargo/config.toml` selected no linker.
- **Machine:** AMD Ryzen 5 7520U (8 logical CPUs), 3.4 GiB RAM, NVMe disk,
  `CARGO_BUILD_JOBS=2`, a shared workstation; no other Cargo process ran
  during a measurement.
- **Commands:** wall-clock time from `/usr/bin/time` of `cargo build --offline`
  and `cargo check --offline`. *Cold* starts from an empty `target/`
  (dependencies already downloaded). *Edit* changes one string literal in a
  handler (`src/main.rs` for Blank and API, `src/controllers/auth_controller.rs`
  for SaaS) and rebuilds; *touch* only updates that file's timestamp. Cold
  builds: mean of two runs; rebuilds: median of five.

| Starter | Cold `cargo build` | Rebuild after edit | Rebuild after touch | Cold `cargo check` | `cargo check` after touch | `target/` after the build | Binary | Packages compiled |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Blank, before | 175.4 s | 2.56 s | 1.72 s | 127.4 s | 0.83 s | 2.48 GB | 83.9 MB | 310 |
| Blank, generated | 147.4 s (−16 %) | 1.35 s (−47 %) | 1.09 s | 124.2 s | 0.62 s | 1.45 GB (−41 %) | 41.7 MB | 310 |
| API, before | 173.5 s | 2.32 s | 1.38 s | 128.1 s | 1.02 s | 2.47 GB | 83.7 MB | 310 |
| API, generated | 147.5 s (−15 %) | 1.34 s (−42 %) | 1.22 s | 123.6 s | 0.50 s | 1.45 GB (−41 %) | 41.5 MB | 310 |
| SaaS, before | 202.0 s | 4.29 s | 3.11 s | 137.9 s | 1.28 s | 2.99 GB | 113.7 MB | 349 |
| SaaS, generated | 163.2 s (−19 %) | 2.14 s (−50 %) | 2.19 s | 132.6 s | 1.05 s | 1.74 GB (−42 %) | 65.7 MB | 337 |

Cold checks are one run each; checks after touch are the median of three.
`cargo check` generates no code and links nothing, so the profile changes it
little; its `target/` still went from 1.11 to 0.81 GB (SaaS: 1.26 to
0.92 GB). Removing `rullst-connect` alone, with full debug information, took
the cold SaaS build from 203.7 s to 194.1 s and its cold check from 137.9 s
to 135.2 s (one run each).

Other development settings, measured the same way on the API starter (one
run each):

| `[profile.dev]` | Cold `cargo build` | Rebuild after edit | Binary | `target/` |
| :--- | ---: | ---: | ---: | ---: |
| Cargo's default (full debug information), as generated before | 173.9 s | 2.61 s | 83.7 MB | 2.47 GB |
| The same, without the generated `.cargo/config.toml` (as on a fresh clone or in CI) | 177.0 s | 5.52 s | 269.0 MB | 2.76 GB |
| `debug = "line-tables-only"` for every package | 156.5 s | 1.78 s | 71.3 MB | 1.93 GB |
| **Generated:** line tables for the application, none for dependencies | 146.0 s | 1.34 s | 41.5 MB | 1.45 GB |
| `debug = false` for every package | 146.8 s | 1.34 s | 35.2 MB | 1.44 GB |

Line tables for your own package cost nothing measurable, while those of the
dependencies cost about ten seconds of every cold build. Without split debug
information (no `.cargo/config.toml`), the full-debug link was also the
largest process of the build: 1.2 GiB, against 0.7 to 0.8 GiB for every
other setting.

### Where a cold build spends its time

`cargo build --timings` writes `target/cargo-timings/cargo-timing.html` with
the time of every compiled unit. In the generated API and SaaS builds above
the longest single unit was the build script of `aws-lc-sys`, the
cryptography library behind the HTTPS client, at about 52 s; then
`rullst-core` (about 13 s), `jsonschema` (about 9 s) and the SQLite library
bundled by `libsqlite3-sys` (about 6 s). Some crates, SQLite among them, are
compiled twice: once for SQLx's compile-time macros and once for the
application. None of this depends on your code, so it is paid again only
after `cargo clean`, a toolchain update or a change of features or profile.

## Tips

- **Use `cargo check` while you edit.** It skips code generation and
  linking: a check after an edit took 0.6 to 1.0 s in these projects (one run
  each), and a cold check was 23 to 31 seconds shorter than a cold build.
  rust-analyzer in your editor runs the same check when you save.
- **Keep `cargo rullst dev` running.** It reuses `target/`, so after the
  first build only your package is rebuilt on each save (the *edit* column
  above), then it restarts the server; restart time was not measured here.
- **Linker.** On `x86_64-unknown-linux-gnu`, Rust 1.90 and later already
  link with the bundled `rust-lld`, which all the measurements above used.
  On other hosts (for example aarch64 Linux) mold or lld may shorten links:
  on Linux, `cargo rullst doctor` suggests one when rustc does not already
  use LLD and no linker is configured, and a project generated on a machine
  with one selects it in `.cargo/config.toml`. Not measured here.
- **sccache** can share compiled dependencies between projects on the same
  machine (`RUSTC_WRAPPER=sccache`). Not measured here.

## Existing applications

`cargo rullst upgrade` does not edit build profiles. To adopt the faster
development builds, copy the `[profile.dev]` and `[profile.dev.package."*"]`
sections above into your `Cargo.toml`. In an application created from the
SaaS starter before 13.0.0-alpha.4, you can remove the `rullst-connect`
dependency when no source file uses `rullst_connect`; `cargo check` tells you
whether one does.
