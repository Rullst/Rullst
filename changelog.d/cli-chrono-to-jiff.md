### Changed

- `cargo-rullst` uses `jiff` 1 instead of `chrono` for the times it writes:
  migration file names, the daily animation stamp, the times of audit,
  SBOM, compliance and footprint reports, the upgrade backup directory and
  the HTTP dates the footprint page-weight check reads. The text is the same
  as before, byte for byte (local `YYYYMMDDHHMMSS` migration stamps, RFC 3339
  times with `+00:00` and 0, 3, 6 or 9 fraction digits).
