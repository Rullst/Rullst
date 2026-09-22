# Trusted Wasm compatibility fixture

`checked_sum.wasm` is the committed output of the adjacent five-line trusted
addition fixture with Rust 1.96.0 and wasm32-unknown-unknown. It exercises the
Rust ABI/standard-library output, in addition to the hand-encoded minimal Wasm
unit vector. Only these fixed fixtures run in local interpreter unit tests;
adversarial submissions use the isolated acceptance harness. The compiled fixture
contains Rust standard-library code; its MIT notice is in `RUST-LICENSE-MIT`.

The required Labs acceptance job runs `.github/check-labs-fixture.sh` to rebuild
and compare every byte before acceptance. This fixture is not a production
executable or a downloaded student submission. OpenSSF Scorecard detects its
binary format; `.github/scorecard.yml` records the test-only purpose without
filtering scanner output. SHA-256 of the reviewed fixture:
`b89aa864df353309d8b8bdbcddd061d93c28d5c08a902ed779e261f2a6bc765c`.

Regenerate from the repository root:

```sh
rustc +1.96.0 rullst-labs-runner/tests/fixtures/checked_sum.rs --crate-name checked_sum --crate-type cdylib --edition 2024 --target wasm32-unknown-unknown --remap-path-prefix=rullst-labs-runner/tests/fixtures=/fixture -C opt-level=1 -C panic=abort -C debuginfo=0 -C strip=symbols -C codegen-units=1 -C overflow-checks=on -C target-feature=-simd128,-relaxed-simd,-multivalue,-reference-types,-tail-call,-extended-const -C link-arg=-zstack-size=1048576 -C link-arg=--threads=1 -C link-arg=--max-memory=4194304 -o rullst-labs-runner/tests/fixtures/checked_sum.wasm
```
