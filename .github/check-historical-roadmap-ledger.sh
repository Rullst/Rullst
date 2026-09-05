#!/usr/bin/env bash
set -euo pipefail

ledger_path="${1:-docs/src/v12.md}"

if [[ ! -f "$ledger_path" ]]; then
  echo "Historical roadmap ledger not found: $ledger_path"
  exit 1
fi

historical_master_ids=(
  M1 M2 M3 M4 M5 M6 M7 M9 M10 M11 M12 M14 M15 M16 M17 M18 M19
  M24 M26 M27 M28 M29 M30 M32
)

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
declare -A master_rows=()
declare -A detail_counts=()
declare -A detail_ids=()

while IFS='|' read -r _ id state _; do
  [[ "$id" =~ \*\*(M[0-9]+)\*\* ]] || continue
  id="${BASH_REMATCH[1]}"
  master_rows["$id"]="$state"
done < <(
  sed -n \
    '/^### Reclassificação integral do roadmap mestre M1–M32$/,/^### Classificação dos dez roadmaps detalhados por crate$/p' \
    "$ledger_path"
)

master_integral=0
master_partial=0
for id in "${historical_master_ids[@]}"; do
  state="${master_rows[$id]:-}"
  if [[ -z "$state" ]]; then
    echo "Historical master claim $id is missing from $ledger_path."
    exit 1
  fi

  case "$state" in
    *Implementado*) master_integral=$((master_integral + 1)) ;;
    *Parcial*) master_partial=$((master_partial + 1)) ;;
    *)
      echo "Historical master claim $id has an unclassified state: $state"
      exit 1
      ;;
  esac
done

detail_integral=0
detail_partial=0
detail_absent=0
while IFS='|' read -r _ id _claim state _; do
  [[ "$id" =~ \*\*([A-Z]+)-([0-9]+)\*\* ]] || continue
  prefix="${BASH_REMATCH[1]}"
  id="${prefix}-${BASH_REMATCH[2]}"
  if [[ -z "${expected_detail_counts[$prefix]+present}" ]]; then
    echo "Unexpected historical detail prefix in $id."
    exit 1
  fi
  if [[ -n "${detail_ids[$id]+present}" ]]; then
    echo "Duplicate historical detail claim: $id"
    exit 1
  fi

  detail_ids["$id"]=1
  detail_counts["$prefix"]=$(( ${detail_counts[$prefix]:-0} + 1 ))
  case "$state" in
    *Integral*) detail_integral=$((detail_integral + 1)) ;;
    *Parcial*) detail_partial=$((detail_partial + 1)) ;;
    *Ausente*) detail_absent=$((detail_absent + 1)) ;;
    *)
      echo "Historical detail claim $id has an unclassified state: $state"
      exit 1
      ;;
  esac
done < <(
  sed -n \
    '/^#### Inventário item a item — `rullst-ai` histórico$/,/^O inventário deduplicado está agora/p' \
    "$ledger_path"
)

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
historical_total=$((master_total + detail_total))
integral_total=$((master_integral + detail_integral))
partial_total=$((master_partial + detail_partial))

if [[ "$master_total" -ne 24 || "$detail_total" -ne 166 || "$historical_total" -ne 190 ]]; then
  echo "Historical denominator drifted: master=$master_total detail=$detail_total total=$historical_total."
  exit 1
fi

if [[ "$integral_total" -ne 106 || "$partial_total" -ne 82 || "$detail_absent" -ne 2 ]]; then
  echo "Historical status distribution drifted: integral=$integral_total partial=$partial_total absent=$detail_absent."
  exit 1
fi

echo "Historical roadmap ledger verified: 190 claims (106 integral, 82 partial, 2 absent)."
