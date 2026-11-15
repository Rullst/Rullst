### Changed

- `rullst-auth` checks passkey (ES256) public keys and assertion signatures
  with aws-lc-rs, the library behind Rullst's TLS, instead of `p256` (the
  point check at registration and in `SqlitePasskeyStore` and the shared
  ceremony manager) and `ring` (the assertion signature). The SaaS starter,
  like any application with the `auth` or `nexus` feature, resolves 18 fewer
  packages in its `Cargo.lock` (398 to 380) and compiles 17 fewer crates.
  Every decision is the same: differential tests run both implementations on
  valid, high-S and low-S signatures, wrong messages and keys, malformed DER,
  points off the curve or with a coordinate of `p` or more, the point at
  infinity and other encodings, and both give the expected result on the 484
  Wycheproof ECDSA P-256 SHA-256 vectors. High-S signatures are still
  accepted, as WebAuthn allows. The public API is unchanged.
