#!/usr/bin/env bash
# Rebuild only the fixed, reviewed test source. Never accepts learner input.
set -euo pipefail
cd "$(dirname "$0")/.."
fixture_dir=$(mktemp -d)
trap 'rm -rf "$fixture_dir"' EXIT
rustc +1.96.0 rullst-labs-runner/tests/fixtures/checked_sum.rs \
  --crate-name checked_sum --crate-type cdylib --edition 2024 \
  --target wasm32-unknown-unknown \
  --remap-path-prefix=rullst-labs-runner/tests/fixtures=/fixture \
  -C opt-level=1 -C panic=abort -C debuginfo=0 -C strip=symbols \
  -C codegen-units=1 -C overflow-checks=on \
  -C target-feature=-simd128,-relaxed-simd,-multivalue,-reference-types,-tail-call,-extended-const \
  -C link-arg=-zstack-size=1048576 -C link-arg=--threads=1 \
  -C link-arg=--max-memory=4194304 -o "$fixture_dir/checked_sum.wasm"
cmp rullst-labs-runner/tests/fixtures/checked_sum.wasm "$fixture_dir/checked_sum.wasm"
echo 'Trusted Wasm fixture matches the reviewed source and pinned compiler.'
