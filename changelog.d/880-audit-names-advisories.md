### Changed

- `cargo rullst audit` printed only how many advisories cargo-audit reported
  (`cargo-audit reported 1 vulnerable dependency advisory(ies); run cargo
  audit for the list`), so a CI log needed a second run to know what to fix.
  It now lists each one: the advisory ID and title, the crate and its version
  in `Cargo.lock`, and the fixed versions, or that none is listed, for
  example `RUSTSEC-2026-0592 in url 2.5.8: <title>; fixed in >=2.5.9`. At
  most 50 are listed, then how many more. With `--json` the list goes to
  standard error with the other progress lines and the
  `rullst.cli-audit.v1` summary is unchanged; `--report` records one finding
  per listed advisory, and its detail says how many more there are
  ([#880](https://github.com/Rullst/Rullst/issues/880)).
