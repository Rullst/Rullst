#!/usr/bin/env bash
# Initial admission is deliberately limited to v13 push feedback, never release/PR runs.
set -euo pipefail

runtime_required=true
finish() {
  local scope_exit=$?
  if [[ "$scope_exit" -ne 0 ]]; then
    runtime_required=true
  fi
  printf 'runtime_required=%s\n' "$runtime_required" >> "$GITHUB_OUTPUT"
  return "$scope_exit"
}
trap finish EXIT

if [[ "${GITHUB_EVENT_NAME:-}" != push || "${GITHUB_REF:-}" != refs/heads/v13 ||
      "${GITHUB_REPOSITORY:-}" != Rullst/Rullst ]]; then
  exit 0
fi
if [[ ! "${BASE_SHA:-}" =~ ^[0-9a-f]{40}$ ||
      ! "${GITHUB_SHA:-}" =~ ^[0-9a-f]{40}$ ||
      "$BASE_SHA" == 0000000000000000000000000000000000000000 ]]; then
  exit 0
fi
if [[ "$(git rev-parse HEAD)" != "$GITHUB_SHA" ]]; then
  exit 0
fi

scope_dir="$(mktemp -d "$RUNNER_TEMP/rullst-ci-scope.XXXXXX")"
# Run policy helpers from the prior committed source, in isolated Python mode;
# a candidate must not weaken its own admission decision. A missing old helper
# or API failure simply retains the full runtime path.
for helper in admit-site-only.py plan-verification.py report-ci-timings.py; do
  if ! git show "$BASE_SHA:.github/$helper" > "$scope_dir/$helper"; then
    exit 0
  fi
done
if ! gh api --hostname github.com \
  "repos/Rullst/Rullst/actions/workflows/ci.yml/runs?branch=v13&event=push&head_sha=$BASE_SHA&status=success&per_page=5" \
  > "$scope_dir/runs.json"; then
  exit 0
fi
run_id="$(jq -r '[.workflow_runs[] | select(.conclusion == "success")][0].id // empty' "$scope_dir/runs.json")"
if [[ ! "$run_id" =~ ^[1-9][0-9]*$ ]]; then
  exit 0
fi
if ! gh api --hostname github.com "repos/Rullst/Rullst/actions/runs/$run_id" > "$scope_dir/run.json"; then
  exit 0
fi
attempt="$(jq -r '.run_attempt' "$scope_dir/run.json")"
if [[ ! "$attempt" =~ ^[1-9][0-9]*$ ]]; then
  exit 0
fi
if ! gh api --hostname github.com --paginate --slurp \
  "repos/Rullst/Rullst/actions/runs/$run_id/attempts/$attempt/jobs?per_page=100" \
  > "$scope_dir/jobs.json"; then
  exit 0
fi
if ! jq -n --slurpfile run "$scope_dir/run.json" --slurpfile jobs "$scope_dir/jobs.json" \
  '{run: $run[0], jobs: $jobs[0]}' |
  python3 -I "$scope_dir/admit-site-only.py" --base "$BASE_SHA" --head "$GITHUB_SHA" \
    --branch v13 --repo "$PWD" > "$RUNNER_TEMP/site-ci-admission.json"; then
  exit 0
fi
if jq -e '.schema == "rullst.site-ci-admission.v1" and .runtime_required == false and
          .site_validation_required == true and .release_evidence_eligible == false' \
  "$RUNNER_TEMP/site-ci-admission.json" > /dev/null; then
  runtime_required=false
fi
{
  echo '### Development-only site admission'
  echo 'Manual runs, pull requests, main and publication requirements remain unchanged.'
  echo '```json'
  jq '.' "$RUNNER_TEMP/site-ci-admission.json"
  echo '```'
} >> "$GITHUB_STEP_SUMMARY"
