#!/usr/bin/env bash
set -euo pipefail

fixture_root="$(mktemp -d)"
trap 'rm -rf -- "$fixture_root"' EXIT

make_shard() {
  local directory="$1"
  local names_json="$2"
  local summaries_json="$3"
  mkdir -p "$directory"

  jq -n --argjson names "$names_json" '$names | map({name: .})' \
    >"$directory/mutants.json"
  jq -n \
    --argjson names "$names_json" \
    --argjson summaries "$summaries_json" '
      {
        cargo_mutants_version: "27.1.0",
        total_mutants: ($names | length),
        caught: ([$summaries[] | select(. == "CaughtMutant")] | length),
        missed: ([$summaries[] | select(. == "MissedMutant")] | length),
        timeout: ([$summaries[] | select(. == "Timeout")] | length),
        unviable: ([$summaries[] | select(. == "Unviable")] | length),
        outcomes: (
          [{scenario: "Baseline", summary: "Success"}] +
          [range(0; $names | length) as $index | {
            scenario: {Mutant: {name: $names[$index]}},
            summary: $summaries[$index]
          }]
        )
      }
    ' >"$directory/outcomes.json"
}

valid_root="$fixture_root/valid"
make_shard "$valid_root/shard-0" \
  '["crate/src/a.rs: first", "crate/src/a.rs: second"]' \
  '["CaughtMutant", "MissedMutant"]'
make_shard "$valid_root/shard-1" \
  '["crate/src/b.rs: third"]' \
  '["Timeout"]'

MUTATION_SUMMARY_JSON="$fixture_root/valid.json" \
MUTATION_SUMMARY_MARKDOWN="$fixture_root/valid.md" \
  bash .github/summarize-mutation-artifacts.sh \
    "$valid_root" full 2 3 27.1.0 >/dev/null
jq -e '
  .mode == "full" and
  .shards == 2 and
  .planned == 3 and
  .processed == 3 and
  .caught == 1 and
  .missed == 1 and
  .timeout == 1 and
  .unviable == 0 and
  .conservative_caught_percent == 33.33
' "$fixture_root/valid.json" >/dev/null

if MUTATION_SUMMARY_JSON="$fixture_root/drift.json" \
  MUTATION_SUMMARY_MARKDOWN="$fixture_root/drift.md" \
  bash .github/summarize-mutation-artifacts.sh \
    "$valid_root" full 2 4 27.1.0 >/dev/null 2>&1; then
  echo "Inventory drift fixture was accepted." >&2
  exit 1
fi

overlap_root="$fixture_root/overlap"
make_shard "$overlap_root/shard-0" \
  '["crate/src/a.rs: duplicate"]' \
  '["CaughtMutant"]'
make_shard "$overlap_root/shard-1" \
  '["crate/src/a.rs: duplicate"]' \
  '["CaughtMutant"]'
if MUTATION_SUMMARY_JSON="$fixture_root/overlap.json" \
  MUTATION_SUMMARY_MARKDOWN="$fixture_root/overlap.md" \
  bash .github/summarize-mutation-artifacts.sh \
    "$overlap_root" targeted 2 '' 27.1.0 >/dev/null 2>&1; then
  echo "Overlapping shard fixture was accepted." >&2
  exit 1
fi

incomplete_root="$fixture_root/incomplete"
make_shard "$incomplete_root/shard-0" \
  '["crate/src/a.rs: classified", "crate/src/a.rs: missing"]' \
  '["CaughtMutant", "MissedMutant"]'
jq '
  .total_mutants = 1 |
  .missed = 0 |
  .outcomes = .outcomes[0:2]
' "$incomplete_root/shard-0/outcomes.json" \
  >"$incomplete_root/shard-0/outcomes.partial.json"
mv "$incomplete_root/shard-0/outcomes.partial.json" \
  "$incomplete_root/shard-0/outcomes.json"
if MUTATION_SUMMARY_JSON="$fixture_root/incomplete.json" \
  MUTATION_SUMMARY_MARKDOWN="$fixture_root/incomplete.md" \
  bash .github/summarize-mutation-artifacts.sh \
    "$incomplete_root" targeted 1 '' 27.1.0 >/dev/null 2>&1; then
  echo "Incomplete shard fixture was accepted." >&2
  exit 1
fi

echo "Mutation artifact aggregation contract verified."
