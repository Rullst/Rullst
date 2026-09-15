#!/usr/bin/env bash
set -euo pipefail

ledger_path="${1:-.github/historical-roadmap-ledger-v12.tsv}"

if [[ ! -f "$ledger_path" ]]; then
  echo "Historical roadmap ledger not found: $ledger_path"
  exit 1
fi

historical_master_ids=(
  M1 M2 M3 M4 M5 M6 M7 M9 M10 M11 M12 M14 M15 M16 M17 M18 M19
  M24 M26 M27 M28 M29 M30 M32
)

declare -A expected_master=()
for id in "${historical_master_ids[@]}"; do
  expected_master["$id"]=1
done

declare -A expected_detail_counts=(
  [AI]=7
  [AUTH]=2
  [CAP]=13
  [CONNECT]=23
  [IOT]=15
  [MAIL]=18
  [NEXUS]=4
  [ORM]=45
  [SEC]=32
  [STUDIO]=7
)
declare -A seen=()
declare -A detail_counts=()
declare -A master_seen=()
declare -A state_counts=()

row_pattern=$'^([A-Z]+-[0-9]{2}|M[0-9]+)\t(integral|partial|absent)$'
while IFS= read -r row || [[ -n "$row" ]]; do
  [[ -z "$row" || "$row" == \#* ]] && continue
  if [[ ! "$row" =~ $row_pattern ]]; then
    echo "Malformed historical ledger row."
    exit 1
  fi
  id="${BASH_REMATCH[1]}"
  state="${BASH_REMATCH[2]}"
  if [[ -n "${seen[$id]+present}" ]]; then
    echo "Duplicate historical claim: $id"
    exit 1
  fi
  seen["$id"]=1

  case "$state" in
    integral|partial|absent) state_counts["$state"]=$(( ${state_counts[$state]:-0} + 1 )) ;;
    *)
      echo "Historical claim $id has an invalid classification: $state"
      exit 1
      ;;
  esac

  if [[ "$id" =~ ^M[0-9]+$ ]]; then
    if [[ -z "${expected_master[$id]+present}" ]]; then
      echo "Unexpected historical master claim: $id"
      exit 1
    fi
    if [[ "$state" == "absent" ]]; then
      echo "Historical master claim $id cannot use the absent classification."
      exit 1
    fi
    master_seen["$id"]=1
    continue
  fi

  if [[ "$id" =~ ^([A-Z]+)-([0-9]{2})$ ]]; then
    prefix="${BASH_REMATCH[1]}"
    number="$((10#${BASH_REMATCH[2]}))"
    if [[ -z "${expected_detail_counts[$prefix]+present}" ]]; then
      echo "Unexpected historical detail prefix in $id."
      exit 1
    fi
    if (( number < 1 || number > expected_detail_counts[$prefix] )); then
      echo "Historical detail identifier out of range: $id"
      exit 1
    fi
    detail_counts["$prefix"]=$(( ${detail_counts[$prefix]:-0} + 1 ))
    continue
  fi

  echo "Invalid historical claim identifier: $id"
  exit 1
done < "$ledger_path"

for id in "${historical_master_ids[@]}"; do
  if [[ -z "${master_seen[$id]+present}" ]]; then
    echo "Historical master claim $id is missing from $ledger_path."
    exit 1
  fi
done

detail_total=0
for prefix in "${!expected_detail_counts[@]}"; do
  actual="${detail_counts[$prefix]:-0}"
  expected="${expected_detail_counts[$prefix]}"
  if [[ "$actual" -ne "$expected" ]]; then
    echo "Historical $prefix detail count is $actual; expected $expected."
    exit 1
  fi
  detail_total=$((detail_total + actual))
done

master_total="${#historical_master_ids[@]}"
historical_total="${#seen[@]}"
if [[ "$master_total" -ne 24 || "$detail_total" -ne 166 || "$historical_total" -ne 190 ]]; then
  echo "Historical denominator drifted: master=$master_total detail=$detail_total total=$historical_total."
  exit 1
fi

integral_total="${state_counts[integral]:-0}"
partial_total="${state_counts[partial]:-0}"
absent_total="${state_counts[absent]:-0}"
if [[ "$integral_total" -ne 106 || "$partial_total" -ne 82 || "$absent_total" -ne 2 ]]; then
  echo "Historical status distribution drifted: integral=$integral_total partial=$partial_total absent=$absent_total."
  exit 1
fi

echo "Historical roadmap ledger verified: 190 claims (106 integral, 82 partial, 2 absent)."
