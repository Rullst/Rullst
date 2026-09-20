# Privacy fuzz contracts

This unpublished workspace enables only `challenge-tokens` and its bounded age
foundation. It does not build SQLx, start a runtime, contact an age provider or
retain personal data. Its public deterministic keys, nonce and clock are test
fixtures, never production defaults.

- `fuzz_age_challenge_token` tries arbitrary transport strings, authenticates
  arbitrary JSON bytes with the fixture HMAC key and mutates a valid challenge
  or token. Acceptance requires the exact current policy/binding, threshold,
  round-trip preservation and rejection at expiry or after a tenant/policy change.
- `fuzz_age_attestation` exercises all three assurance methods and four outcomes,
  signs arbitrary or mutated issuer payloads, and corrupts signatures. Accepted
  results must have the correct decision/assurance and exact challenge; a second
  claim, including through the native declaration gate, must fail as replay.
  The bounded memory store is explicit development evidence, not durable storage.

Valid frames are constructed inside the harness, so semantic paths are reachable
without a downloaded corpus. The memory-store future must complete on one poll;
an unexpected pending result fails the harness instead of skipping its assertions.
The token and attestation harnesses cap inputs at 9,000 and 8,192 bytes respectively.
The workflow permits lengths up to 9,000 to reach the public payload/token limits.

From `rullst-privacy`, a short local diagnostic is:

```sh
cargo +nightly-2026-08-21 fuzz run --target x86_64-unknown-linux-gnu \
  fuzz_age_challenge_token -- -max_total_time=300 -max_len=9000 -timeout=10 -rss_limit_mb=2048
cargo +nightly-2026-08-21 fuzz run --target x86_64-unknown-linux-gnu \
  fuzz_age_attestation -- -max_total_time=300 -max_len=9000 -timeout=10 -rss_limit_mb=2048
```

The shared inventory and v13 publication policy require both targets. A short
diagnostic is not the complete release campaign, a provider certification or
proof for all inputs. SQLite/PostgreSQL lifecycle tests remain separate.
