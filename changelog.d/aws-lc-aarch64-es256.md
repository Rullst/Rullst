### Fixed

- On aarch64 (Linux arm64 servers such as Graviton and Ampere, and Apple
  silicon), aws-lc-rs 1.18 with aws-lc-sys 0.44 rejected some valid ECDSA
  P-256 signatures: every signature whose s is 2^k or n − 2^k with k from 64
  to 255, because AWS-LC's aarch64 modular inverse mishandled a zero low limb
  (aws/aws-lc#3715). Passkey assertions, ES256 JWTs and TLS ECDSA signatures
  verify through it. It only rejected valid signatures, never accepted an
  invalid one, and an honest signer's s falls in that set with a probability
  of about 2^-247. `rullst-auth` (with `passkeys`) now requires aws-lc-rs
  1.19.0, which uses aws-lc-sys 0.47.0 (AWS-LC 5.12.0) with the fix; an
  application that gets aws-lc-rs only through rustls or jsonwebtoken picks
  it up with `cargo update -p aws-lc-rs`.
