#!/usr/bin/env bash
# Mutation testing of the Rust lines a pull request changes (mutants-pr.yml).
#
#   mutants-pr.sh list      JSON inventory of the changed-line mutants
#   mutants-pr.sh run K/N   test shard K of N of that inventory
#
# The diff base defaults to HEAD^1, the base-branch parent of the merge commit
# GitHub checks out for a pull request; MUTANTS_PR_BASE overrides it locally.
#
# Two passes share the inventory, and `run K/N` tests shard K of N of each:
# - the workspace pass tests a mutant with its package's whole test suite. It
#   skips cargo-rullst, whose integration suites build generated projects and
#   are too slow to rerun per mutant;
# - the CLI pass mutates only the cargo-rullst files listed in
#   mutants-pr-cli.txt and tests them with the CLI's library unit tests, as a
#   targeted mutants.yml run with test_scope=lib does.
set -euo pipefail

root=$(git rev-parse --show-toplevel)
base="${MUTANTS_PR_BASE:-HEAD^1}"
scratch="${RUNNER_TEMP:-${TMPDIR:?set TMPDIR}}"
diff_file="$scratch/mutants-pr.diff"
git -C "$root" diff --no-ext-diff --no-renames "$base" HEAD -- '*.rs' > "$diff_file"

entries() {
  grep -vE '^[[:space:]]*(#|$)' "$1" || true
}

shared=(--workspace --all-features --in-diff "$diff_file")
while IFS= read -r glob; do
  shared+=(--exclude "$glob")
done < <(entries "$root/.github/mutants-pr-exclude.txt")
# Accepted equivalents (WORKFLOWS.md) are listed by their exact cargo-mutants
# name, `file:line:col: description`. A line that moves is mutated again.
while IFS= read -r name; do
  escaped=$(printf '%s' "$name" | sed -e 's/[][\.*+?(){}|^$]/\\&/g')
  shared+=(--exclude-re "^${escaped}\$")
done < <(entries "$root/.github/mutants-pr-equivalents.txt")

workspace=(cargo mutants "${shared[@]}" --exclude 'cargo-rullst/**')
cli_files=()
while IFS= read -r path; do
  cli_files+=(--file "$path")
done < <(entries "$root/.github/mutants-pr-cli.txt")
# Without a --file, cargo-mutants would mutate every file, so an empty list
# disables the CLI pass.
cli=()
if [ "${#cli_files[@]}" -gt 0 ]; then
  cli=(cargo mutants "${shared[@]}" "${cli_files[@]}")
fi

# Prints how many mutants a pass lists, optionally within one shard.
count() {
  "$@" --list --json | jq -s 'map(length) | add // 0'
}

case "${1:-}" in
  list)
    "${workspace[@]}" --list --json
    if [ "${#cli[@]}" -gt 0 ]; then
      "${cli[@]}" --list --json
    fi
    ;;
  run)
    if [[ ! "${2:-}" =~ ^[0-9]+/[1-9][0-9]*$ ]]; then
      echo "usage: $0 run K/N" >&2
      exit 2
    fi
    # Same per-mutant bounds as a targeted mutants.yml run.
    bounds=(--in-place --shard "$2" --timeout 300 --build-timeout 300)
    statuses=()
    if [ "$(count "${workspace[@]}" --shard "$2")" -gt 0 ]; then
      set +e
      "${workspace[@]}" "${bounds[@]}"
      statuses+=("$?")
      set -e
    fi
    if [ "${#cli[@]}" -gt 0 ] && [ "$(count "${cli[@]}" --shard "$2")" -gt 0 ]; then
      # A second run would rename the workspace pass's mutants.out, so the
      # CLI pass writes elsewhere and its lists are appended to mutants.out,
      # which mutants-pr.yml reads and uploads.
      cli_output="$scratch/mutants-pr-cli"
      rm -rf "$cli_output"
      set +e
      "${cli[@]}" "${bounds[@]}" --output "$cli_output" -- --lib
      statuses+=("$?")
      set -e
      mkdir -p "$root/mutants.out"
      for list in caught missed timeout unviable; do
        if [ -f "$cli_output/mutants.out/$list.txt" ]; then
          cat "$cli_output/mutants.out/$list.txt" >> "$root/mutants.out/$list.txt"
        fi
      done
      cp -R "$cli_output/mutants.out" "$root/mutants.out/cli"
    fi
    # cargo-mutants exits 0 when every mutant is caught or unviable, 2 when
    # one is missed and 3 when one only timed out; anything else is a failure.
    result=0
    for status in "${statuses[@]}"; do
      case "$status" in
        0) ;;
        2) result=2 ;;
        3) if [ "$result" -eq 0 ]; then result=3; fi ;;
        *) exit "$status" ;;
      esac
    done
    exit "$result"
    ;;
  *)
    echo "usage: $0 list | run K/N" >&2
    exit 2
    ;;
esac
