#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
validator="$root/.github/check-historical-roadmap-ledger.sh"
ledger="$root/.github/historical-roadmap-ledger-v12.tsv"
fixtures="$(mktemp -d)"
trap 'rm -f "$fixtures/case.tsv"; rmdir "$fixtures"' EXIT

assert_rejected() {
  local description="$1"
  if bash "$validator" "$fixtures/case.tsv" >/dev/null 2>&1; then
    echo "FAIL: validator accepted $description"
    exit 1
  fi
}

bash "$validator" "$ledger"

sed '/^ORM-45/d' "$ledger" > "$fixtures/case.tsv"
assert_rejected "a missing historical claim"

awk '{ print } /^AI-01/ { print }' "$ledger" > "$fixtures/case.tsv"
assert_rejected "a duplicate claim"

sed 's/^ORM-45/ORM-99/' "$ledger" > "$fixtures/case.tsv"
assert_rejected "a replacement ID with the same denominator"

sed 's/^M32/M31/' "$ledger" > "$fixtures/case.tsv"
assert_rejected "an out-of-scope master claim"

sed $'s/^AI-01\tintegral$/AI-01\tpartial/' "$ledger" > "$fixtures/case.tsv"
assert_rejected "a changed status distribution"

sed $'s/^AI-01\tintegral$/AI-01\tcomplete/' "$ledger" > "$fixtures/case.tsv"
assert_rejected "an unknown status"

sed $'s/^AI-01\tintegral$/AI-01\tintegral\textra/' "$ledger" > "$fixtures/case.tsv"
assert_rejected "an extra column"

sed $'s/^AI-01\tintegral$/AI-01\tpartial/; s/^AI-02\tpartial$/AI-02\tintegral/' "$ledger" > "$fixtures/case.tsv"
assert_rejected "swapped classifications with unchanged totals"

awk 'BEGIN { ORS="" } { if (NR > 1) printf "\n"; print }' "$ledger" > "$fixtures/case.tsv"
bash "$validator" "$fixtures/case.tsv" >/dev/null

LC_ALL=C sort -r "$ledger" > "$fixtures/case.tsv"
bash "$validator" "$fixtures/case.tsv" >/dev/null

printf 'Historical ledger: baseline, eight invalid fixtures, final-line handling, and row reordering passed.\n'
