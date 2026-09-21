#!/usr/bin/env bash
# Measure host controller paths using the real isolated journey. The identical
# binary runs every role; workers never initialize or export profile counters.
set -euo pipefail
[[ "${GITHUB_ACTIONS:-}" == true && "${RUNNER_ENVIRONMENT:-}" == github-hosted ]]
measurement_dir=$(mktemp -d "${RUNNER_TEMP:?}/rullst-labs-measured.XXXXXX")
trap 'rm -rf -- "$measurement_dir"' EXIT
cc -std=c11 -Wall -Wextra -Werror -c .github/labs-profile-runtime.c \
  -o "$measurement_dir/profile-runtime.o"
install -m 0755 .github/labs-profile-linker.sh "$measurement_dir/linker"

# External-test integration documented by cargo-llvm-cov. Preserve existing
# profiles from the workspace/default/DB suites; never clean them here.
# show-env defaults to Cargo's ordinary target, whereas standalone coverage
# commands use its llvm-cov-target child. Share the latter with the existing
# workspace campaign and the report command that runs outside this shell.
coverage_target=$(cargo metadata --no-deps --locked --offline --format-version 1 | \
  python3 -c 'import json,sys; from pathlib import Path; print(Path(json.load(sys.stdin)["target_directory"]) / "llvm-cov-target")')
CARGO_TARGET_DIR="$coverage_target" cargo llvm-cov show-env --sh > "$measurement_dir/coverage-env.sh"
source "$measurement_dir/coverage-env.sh"
test "$CARGO_LLVM_COV_TARGET_DIR" = "$coverage_target"
# Explicit target flags also invalidate an ordinary cached executable. Changing
# RUSTC_WRAPPER alone does not necessarily make Cargo rebuild an existing target.
cargo rustc --locked -p rullst-labs-runner --bin rullst-labs-runner \
  --all-features --target-dir "$CARGO_LLVM_COV_TARGET_DIR" -- \
  -C instrument-coverage -C linker-flavor=gcc -C "linker=$measurement_dir/linker"
cargo rustc --locked -p rullst-labs --example course_app \
  --all-features --target-dir "$CARGO_LLVM_COV_TARGET_DIR" -- -C instrument-coverage
readelf --wide --section-headers "$CARGO_LLVM_COV_TARGET_DIR/debug/rullst-labs-runner" > "$measurement_dir/controller-sections"
grep -q '__llvm_prf_cnts' "$measurement_dir/controller-sections"
profile_prefix="labs-controller-${GITHUB_RUN_ID:?}-${GITHUB_RUN_ATTEMPT:?}"
test -z "$(find "$CARGO_LLVM_COV_TARGET_DIR" -maxdepth 1 -name "$profile_prefix-*.profraw" -print -quit)"
sudo systemd-run --wait --collect --pipe \
  --unit="rullst-labs-acceptance-coverage-$GITHUB_RUN_ID-$GITHUB_RUN_ATTEMPT" \
  --property="User=$(id -un)" --property="Group=$(id -gn)" \
  --property=Delegate=yes --property=MemoryMax=3G \
  --property=TasksMax=256 --property=RuntimeMaxSec=15min \
  --setenv=RULLST_LABS_HOSTED_ACCEPTANCE=1 \
  --setenv="LLVM_PROFILE_FILE=$CARGO_LLVM_COV_TARGET_DIR/$profile_prefix-%p-%m.profraw" \
  --working-directory="$PWD" \
  /usr/bin/python3 .github/test-labs-isolation.py \
  --runner "$CARGO_LLVM_COV_TARGET_DIR/debug/rullst-labs-runner" \
  --app "$CARGO_LLVM_COV_TARGET_DIR/debug/examples/course_app" \
  --toolchain "$(rustc +1.96.0 --print sysroot)" \
  --launcher /usr/lib/rullst-labs-ci/bwrap \
  --evidence "$PWD/target/labs-coverage-evidence.json"
python3 - "$CARGO_LLVM_COV_TARGET_DIR" "$profile_prefix" <<'PY'
import json, sys
from pathlib import Path
evidence = json.loads(Path('target/labs-coverage-evidence.json').read_text())
assert evidence['status'] == 'passed'
profiles = list(Path(sys.argv[1]).glob(sys.argv[2] + '-*.profraw'))
assert profiles and all(p.is_file() and p.stat().st_size > 0 for p in profiles)
print('Measured real isolated controller journey:', len(evidence['checks']), 'checks;', len(profiles), 'profiles')
PY
