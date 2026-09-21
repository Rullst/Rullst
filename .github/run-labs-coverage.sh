#!/usr/bin/env bash
# Measure the trusted controller using the real isolated journey. The worker
# remains uninstrumented; no profiler environment/path enters its sandbox.
set -euo pipefail
[[ "${GITHUB_ACTIONS:-}" == true && "${RUNNER_ENVIRONMENT:-}" == github-hosted ]]
plain_runner="${1:?usage: run-labs-coverage.sh UNINSTRUMENTED_RUNNER}"
measurement_dir=$(mktemp -d "${RUNNER_TEMP:?}/rullst-labs-measured.XXXXXX")
trap 'rm -rf -- "$measurement_dir"' EXIT
install -m 0755 "$plain_runner" "$measurement_dir/runner"
readelf --wide --section-headers "$measurement_dir/runner" > "$measurement_dir/worker-sections"
if grep -q '__llvm_prf_cnts' "$measurement_dir/worker-sections"; then
  echo 'The isolated worker must not be instrumented.' >&2
  exit 1
fi

# External-test integration documented by cargo-llvm-cov. Preserve existing
# profiles from the workspace/default/DB suites; never clean them here.
cargo llvm-cov show-env --sh > "$measurement_dir/coverage-env.sh"
source "$measurement_dir/coverage-env.sh"
cargo build --locked -p rullst-labs -p rullst-labs-runner \
  --all-features --bins --examples --target-dir "$CARGO_LLVM_COV_TARGET_DIR"
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
  --runner "$measurement_dir/runner" \
  --controller "$CARGO_LLVM_COV_TARGET_DIR/debug/rullst-labs-runner" \
  --app "$CARGO_LLVM_COV_TARGET_DIR/debug/examples/course_app" \
  --toolchain "$(rustc +1.96.0 --print sysroot)" \
  --launcher /usr/lib/rullst-labs-ci/bwrap \
  --evidence "$PWD/target/labs-coverage-evidence.json"
python3 - "$CARGO_LLVM_COV_TARGET_DIR" "$profile_prefix" <<'PY'
import json, sys
from pathlib import Path
evidence = json.loads(Path('target/labs-coverage-evidence.json').read_text())
assert evidence['status'] == 'passed' and evidence['controller_measurement_only']
profiles = list(Path(sys.argv[1]).glob(sys.argv[2] + '-*.profraw'))
assert profiles and all(p.is_file() and p.stat().st_size > 0 for p in profiles)
print('Measured real isolated controller journey:', len(evidence['checks']), 'checks;', len(profiles), 'profiles')
PY
