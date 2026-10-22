#!/usr/bin/env bash
# TEMP-DIAG: Windows SQLite flake investigation; removed before the final commit.
set -uo pipefail
export RUST_BACKTRACE=0
budget=${DIAG_BUDGET_SECONDS:-1500}
suites=(labs media supervision privacy)

args_for() {
  case "$1" in
    labs) echo "-p rullst-labs --all-features" ;;
    media) echo "-p rullst-media --features sqlite" ;;
    supervision) echo "-p rullst-supervision --all-features" ;;
    privacy) echo "-p rullst-privacy --features sqlite,consent-sqlite,challenge-tokens" ;;
  esac
}

for suite in "${suites[@]}"; do
  # shellcheck disable=SC2046
  cargo test $(args_for "$suite") --tests --no-run || exit 2
done

stress() {
  echo "=== stress: $1 threads x $2 initializations"
  # shellcheck disable=SC2046
  STRESS_THREADS=$1 STRESS_ITERATIONS=$2 cargo test $(args_for labs) --test temp_windows_stress \
    -- --ignored --nocapture > "stress-$1.log" 2>&1
  echo "slow initializations: $(grep -c STRESS-SLOW "stress-$1.log")"
  grep STRESS-SLOW "stress-$1.log" | sed 's/.*ms=\([0-9]*\).*/\1/' | sort -n | tail -5 | tr '\n' ' '; echo
  grep -E 'STRESS-FAIL|STRESS-TOTAL|ESTABLISH-RETRY|panicked|test result' "stress-$1.log" | head -200
  echo "retries: $(grep -c ESTABLISH-RETRY "stress-$1.log")"
}
echo "TEMP=${TEMP:-} TMP=${TMP:-}"; python -c "import tempfile; print('python temp', tempfile.gettempdir())"
pressure() {
  for i in 1 2 3 4 5 6 7 8; do python .github/tmp-io-pressure.py "$1" & done
}
pressure 900
stress 4 1500
wait
pressure 1800

declare -A runs failures
for suite in "${suites[@]}"; do runs[$suite]=0; failures[$suite]=0; done
start=$SECONDS
round=0
while (( SECONDS - start < budget )); do
  round=$((round + 1))
  for suite in "${suites[@]}"; do
    # shellcheck disable=SC2046
    out="$(cargo test $(args_for "$suite") --tests -q 2>&1)"
    status=$?
    runs[$suite]=$((runs[$suite] + 1))
    if (( status != 0 )); then
      failures[$suite]=$((failures[$suite] + 1))
      echo "=== FAILURE suite=$suite round=$round"
      printf '%s\n' "$out" | grep -vE '^\s+[0-9]+: |^\s+at ' | tail -120
    fi
  done
  echo "round=$round elapsed=$((SECONDS - start))s"
done
wait
for suite in "${suites[@]}"; do
  echo "SUMMARY suite=$suite runs=${runs[$suite]} failures=${failures[$suite]}"
done
echo "Diagnostic loop finished; failing on purpose to skip the remaining steps."
exit 1
