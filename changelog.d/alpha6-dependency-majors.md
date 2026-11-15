### Changed

- `rullst-iot` (signed OTA verification), `rullst-connect` (the mock identity
  provider of its `axum` feature) and `cargo-rullst` (`iot:keygen`,
  `iot:sign`) use `ed25519-dalek` 3 instead of 2. Keys, signatures and the
  bytes `iot:sign` writes are unchanged, and the `no_std` build still works
  on `thumbv7em-none-eabihf`, `thumbv6m-none-eabi` and
  `riscv32imac-unknown-none-elf`. An application that turns on the
  precomputed tables through feature unification now declares
  `ed25519-dalek = { version = "3", default-features = false, features = ["fast"] }`:
  a 2.x requirement builds a second copy and leaves the tables off. The
  micro:bit V2 fixture's firmware is about 3.3 KB larger; the board was not
  re-measured.
- `rullst-connect` and `rullst-auth` (`jwt`) use `jsonwebtoken` 12, still
  with the `aws_lc_rs` backend only. ID tokens are verified as before, except
  that jsonwebtoken now refuses a token whose `crit` header names an
  extension it does not implement, and a `sub`, `iss` or `aud` claim of the
  wrong JSON type when that claim is checked, instead of skipping the check.
  `cargo rullst make:cors-jwt` adds `jsonwebtoken` 12, and its middleware
  handles `EncodingKey::from_secret` and `DecodingKey::from_secret`, which
  now return a `Result` (an empty key is refused). Middleware generated
  earlier keeps the `jsonwebtoken` 11 its project declares; moving it to 12
  takes the same two changes.
- `rullst-security` pins `jsonschema` `=0.61.2` (was `=0.60.2`), so
  `JsonSchemaPolicy` takes its validation fixes: `.` in `pattern` and
  `patternProperties` no longer matches `\r`, U+2028 or U+2029, and
  `date-time`, `date` and `time` values with `+`, `,`, `-`, `.` or `/` where
  a digit belongs, some of which 0.60 accepted, are refused. A body that
  passed only because of them now gets `422`. The pin stays exact, with its
  reason in the workspace `Cargo.toml`: the policy decides which bodies a
  route accepts, and jsonschema's 0.x patch releases change that decision.
