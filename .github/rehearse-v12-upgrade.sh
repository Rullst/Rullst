#!/usr/bin/env bash
# Manual rehearsal of the v12 → v13 assisted upgrade on generated starters.
# Not a CI job: it builds the framework for each starter and needs the
# crates.io registry for third-party dependencies.
#
#   .github/rehearse-v12-upgrade.sh V12_CLI V13_CLI V13_WORKSPACE WORK_DIR [BLUEPRINT...]
#
# V12_CLI is a cargo-rullst built from `origin/v12` (for example in a
# temporary `git worktree add ... origin/v12`), V13_CLI one built from this
# checkout, V13_WORKSPACE this checkout's root. Each blueprint (default:
# blank blog) is generated with SQLite and `--default`, its Rullst
# dependencies are pointed at V13_WORKSPACE through `[patch.crates-io]`
# (the registry requirement is kept, so the upgrade edits it exactly as it
# would for a published release), then the script records the dry-run plan,
# applies the upgrade with `--keep-on-failure` and runs the project's tests.
# Set CARGO_TARGET_DIR to share one build directory between the starters.
set -euo pipefail

if [ "$#" -lt 4 ]; then
  sed -n '2,16p' "$0" >&2
  exit 2
fi
v12_cli=$(realpath "$1")
v13_cli=$(realpath "$2")
workspace=$(realpath "$3")
work=$4
shift 4
if [ "$#" -eq 0 ]; then
  set -- blank blog
fi
blueprints=("$@")
mkdir -p "$work"
work=$(realpath "$work")
export RULLST_DISABLE_UPDATE_CHECK=1 RULLST_UPDATE_CHECK=0 CI=true

for blueprint in "${blueprints[@]}"; do
  app="$work/v12-$blueprint"
  log="$work/$blueprint"
  rm -rf "$app"
  (cd "$work" && "$v12_cli" rullst new "v12-$blueprint" --default --blueprint "$blueprint" \
      --database sqlite --skip-initial-migration) >"$log.generate.log" 2>&1
  {
    printf '\n[patch.crates-io]\n'
    grep -oE '^rullst(-[a-z]+)* =' "$app/Cargo.toml" | cut -d' ' -f1 | sort -u |
      while read -r crate; do
        printf '%s = { path = "%s/%s" }\n' "$crate" "$workspace" "$crate"
      done
  } >>"$app/Cargo.toml"
  git -C "$app" init --quiet
  git -C "$app" add -A
  git -C "$app" -c user.name=Rehearsal -c user.email=rehearsal@example.invalid \
    commit --quiet -m "v12 $blueprint starter"

  (cd "$app" && "$v13_cli" rullst upgrade --dry-run --json) >"$log.plan.json"
  (cd "$app" && "$v13_cli" rullst upgrade --dry-run) >"$log.plan.txt" 2>&1
  upgrade=0
  (cd "$app" && "$v13_cli" rullst upgrade --keep-on-failure) >"$log.upgrade.log" 2>&1 || upgrade=$?
  tests=skipped
  if [ "$upgrade" -eq 0 ]; then
    tests=passed
    (cd "$app" && cargo test --locked) >"$log.test.log" 2>&1 || tests=failed
  fi
  python3 - "$log.plan.json" "$blueprint" "$upgrade" "$tests" <<'PY'
import json, sys
plan = json.load(open(sys.argv[1]))
counts = plan["finding_counts"]
print(json.dumps({
    "blueprint": sys.argv[2],
    "rule_catalog": plan["rule_catalog"],
    "must_change": counts["must_change"],
    "review": counts["review"],
    "codes": sorted({finding["code"] for finding in plan["source_findings"]}),
    "upgrade_exit": int(sys.argv[3]),
    "tests": sys.argv[4],
}))
PY
done
