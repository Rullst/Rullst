#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -lt 3 || "$#" -gt 5 ]]; then
  echo "usage: $0 <artifact-root> <full|targeted> <expected-shards> [expected-inventory] [expected-version]" >&2
  exit 2
fi

artifact_root="$1"
mode="$2"
expected_shards="$3"
expected_inventory="${4:-}"
expected_version="${5:-27.1.0}"
summary_json="${MUTATION_SUMMARY_JSON:-mutation-summary.json}"
summary_markdown="${MUTATION_SUMMARY_MARKDOWN:-mutation-summary.md}"

if [[ ! -d "$artifact_root" ]]; then
  echo "Mutation artifact root does not exist: $artifact_root" >&2
  exit 2
fi
if [[ "$mode" != "full" && "$mode" != "targeted" ]]; then
  echo "Unsupported mutation mode: $mode" >&2
  exit 2
fi
if [[ ! "$expected_shards" =~ ^[1-9][0-9]*$ ]]; then
  echo "Expected shard count must be a positive integer." >&2
  exit 2
fi
if [[ "$mode" == "full" && ! "$expected_inventory" =~ ^[1-9][0-9]*$ ]]; then
  echo "Full campaigns require a positive expected inventory." >&2
  exit 2
fi

mapfile -d '' outcome_files < <(
  find "$artifact_root" -type f -name outcomes.json -print0 | sort -z
)

if [[ "${#outcome_files[@]}" -ne "$expected_shards" ]]; then
  echo "Mutation artifact count is ${#outcome_files[@]}; expected $expected_shards." >&2
  exit 1
fi

planned_total=0
processed_total=0
caught_total=0
missed_total=0
timeout_total=0
unviable_total=0
mutant_names_file="$(mktemp)"
trap 'rm -f "$mutant_names_file"' EXIT

for outcomes_file in "${outcome_files[@]}"; do
  artifact_dir="$(dirname "$outcomes_file")"
  mutants_file="$artifact_dir/mutants.json"
  if [[ ! -f "$mutants_file" ]]; then
    echo "Missing mutants.json beside $outcomes_file." >&2
    exit 1
  fi

  jq -e '
    . as $root |
    ($root.cargo_mutants_version | type == "string") and
    ([$root.total_mutants, $root.caught, $root.missed, $root.timeout, $root.unviable]
      | all(type == "number" and floor == . and . >= 0)) and
    ($root.outcomes | type == "array") and
    ([$root.outcomes[] | select(.scenario == "Baseline" and .summary == "Success")]
      | length == 1) and
    ([$root.outcomes[] | select(.summary == "CaughtMutant")] | length == $root.caught) and
    ([$root.outcomes[] | select(.summary == "MissedMutant")] | length == $root.missed) and
    ([$root.outcomes[] | select(.summary == "Timeout")] | length == $root.timeout) and
    ([$root.outcomes[] | select(.summary == "Unviable")] | length == $root.unviable)
  ' "$outcomes_file" >/dev/null
  jq -e 'type == "array" and all(.[]; .name | type == "string")' "$mutants_file" >/dev/null
  jq -s -e '
    .[0] as $planned |
    .[1] as $result |
    ($planned | map(.name) | sort) ==
      ($result.outcomes
       | map(select(.scenario | type == "object") | .scenario.Mutant.name)
       | sort)
  ' "$mutants_file" "$outcomes_file" >/dev/null
  jq -r '.[].name' "$mutants_file" >>"$mutant_names_file"

  version="$(jq -r '.cargo_mutants_version' "$outcomes_file")"
  if [[ "$version" != "$expected_version" ]]; then
    echo "Unexpected cargo-mutants version in $outcomes_file: $version." >&2
    exit 1
  fi

  planned="$(jq 'length' "$mutants_file")"
  processed="$(jq '.total_mutants' "$outcomes_file")"
  caught="$(jq '.caught' "$outcomes_file")"
  missed="$(jq '.missed' "$outcomes_file")"
  timed_out="$(jq '.timeout' "$outcomes_file")"
  unviable="$(jq '.unviable' "$outcomes_file")"
  classified=$((caught + missed + timed_out + unviable))

  if [[ "$processed" -ne "$classified" ]]; then
    echo "Outcome counts do not add up in $outcomes_file: processed=$processed classified=$classified." >&2
    exit 1
  fi
  if [[ "$processed" -ne "$planned" ]]; then
    echo "Shard did not classify its complete inventory in $outcomes_file: planned=$planned processed=$processed." >&2
    exit 1
  fi

  planned_total=$((planned_total + planned))
  processed_total=$((processed_total + processed))
  caught_total=$((caught_total + caught))
  missed_total=$((missed_total + missed))
  timeout_total=$((timeout_total + timed_out))
  unviable_total=$((unviable_total + unviable))
done

unique_mutants="$(sort -u "$mutant_names_file" | wc -l)"
if [[ "$unique_mutants" -ne "$planned_total" ]]; then
  echo "Mutation shards overlap: planned=$planned_total unique=$unique_mutants." >&2
  exit 1
fi

if [[ "$mode" == "full" && "$planned_total" -ne "$expected_inventory" ]]; then
  echo "Full mutation inventory drifted: observed=$planned_total expected=$expected_inventory." >&2
  exit 1
fi

jq -n \
  --arg mode "$mode" \
  --arg cargo_mutants_version "$expected_version" \
  --argjson shards "$expected_shards" \
  --argjson planned "$planned_total" \
  --argjson processed "$processed_total" \
  --argjson caught "$caught_total" \
  --argjson missed "$missed_total" \
  --argjson timeout "$timeout_total" \
  --argjson unviable "$unviable_total" \
  '{
    schema_version: 1,
    mode: $mode,
    cargo_mutants_version: $cargo_mutants_version,
    shards: $shards,
    planned: $planned,
    processed: $processed,
    caught: $caught,
    missed: $missed,
    timeout: $timeout,
    unviable: $unviable,
    conservative_caught_percent:
      (if ($caught + $missed + $timeout) == 0 then null
       else (($caught * 10000 / ($caught + $missed + $timeout) | floor) / 100)
       end)
  }' >"$summary_json"

conservative_score="$(jq -r '.conservative_caught_percent // "n/a"' "$summary_json")"
{
  echo "## Mutation campaign aggregate"
  echo
  echo "- Mode: \`$mode\`"
  echo "- cargo-mutants: \`$expected_version\`"
  echo "- Complete shards: \`$expected_shards/$expected_shards\`"
  echo "- Planned and classified: \`$processed_total/$planned_total\`"
  echo "- Caught: \`$caught_total\`"
  echo "- Missed: \`$missed_total\`"
  echo "- Timed out: \`$timeout_total\`"
  echo "- Unviable: \`$unviable_total\`"
  echo "- Conservative caught percentage (timeouts are not counted as caught): \`$conservative_score%\`"
} >"$summary_markdown"

cat "$summary_markdown"
