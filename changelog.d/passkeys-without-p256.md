### Changed

- **Breaking:** passkeys are an opt-in feature. `rullst-auth`'s
  `auth::passkey` module (`PasskeyAuth`, `PasskeyConfig`, `Passkey`, the
  shared ceremony manager and `SqlitePasskeyStore`) compiles only with the new
  `passkeys` feature, which `sqlite` and `passkey-postgres` include; in the
  `rullst` facade, `auth` no longer brings passkeys and the new
  `auth-passkeys` feature does (`auth-sqlite` and `auth-passkey-postgres`
  include it). Nexus and the generated starters use no passkeys and do not
  enable it. Code that names `rullst::auth::passkey` without one of these
  features fails to compile (E0432/E0433); `cargo rullst upgrade` locates it
  (`V13-AUTH-PASSKEYS-FEATURE`). See the
  [migration guide](https://rullst.github.io/Rullst/book/migration-v13.html).
- Passkey public keys and assertion signatures are checked with aws-lc-rs
  instead of `p256` (the point check at registration and in the passkey
  stores) and `ring` (the signature check), and aws-lc-rs is compiled into
  `rullst-auth` only with `passkeys`. The SaaS starter, like any application
  with `auth` or `nexus` but without passkeys, resolves 18 fewer packages in
  its `Cargo.lock` (349 to 331) and compiles 17 fewer crates. Every decision
  is the same: differential tests run both implementations on valid, high-S
  and low-S signatures, wrong messages and keys, malformed DER, points off the
  curve or with a coordinate of `p` or more, the point at infinity and other
  encodings, and both give the expected result on the 484 Wycheproof ECDSA
  P-256 SHA-256 vectors. High-S signatures are still accepted, as WebAuthn
  allows.
