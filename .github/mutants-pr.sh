#!/usr/bin/env bash
# Mutation testing of the Rust lines a pull request changes (mutants-pr.yml).
#
#   mutants-pr.sh list      JSON inventory of the changed-line mutants
#   mutants-pr.sh run K/N   test shard K of N of that inventory
#
# The diff base defaults to HEAD^1, the base-branch parent of the merge commit
# GitHub checks out for a pull request; MUTANTS_PR_BASE overrides it locally.
set -euo pipefail

root=$(git rev-parse --show-toplevel)
base="${MUTANTS_PR_BASE:-HEAD^1}"
diff_file="${RUNNER_TEMP:-${TMPDIR:?set TMPDIR}}/mutants-pr.diff"
git -C "$root" diff --no-ext-diff --no-renames "$base" HEAD -- '*.rs' > "$diff_file"

command=(cargo mutants --workspace --all-features --in-diff "$diff_file")
while IFS= read -r glob; do
  command+=(--exclude "$glob")
done < <(grep -vE '^[[:space:]]*(#|$)' "$root/.github/mutants-pr-exclude.txt")

case "${1:-}" in
  list)
    "${command[@]}" --list --json
    ;;
  run)
    if [[ ! "${2:-}" =~ ^[0-9]+/[1-9][0-9]*$ ]]; then
      echo "usage: $0 run K/N" >&2
      exit 2
    fi
    # Same per-mutant bounds as a targeted mutants.yml run.
    "${command[@]}" --in-place --shard "$2" --timeout 300 --build-timeout 300
    ;;
  *)
    echo "usage: $0 list | run K/N" >&2
    exit 2
    ;;
esac
