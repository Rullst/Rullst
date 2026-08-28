#!/usr/bin/env bash
set -euo pipefail

policy_path=".github/crate-architecture-policy.json"
metadata_path="$(mktemp)"
actual_path="$(mktemp)"
expected_path="$(mktemp)"
trap 'rm -f -- "$metadata_path" "$actual_path" "$expected_path"' EXIT

jq -e '
  .schema_version == 1
  and (.allowed_internal_dependencies | length > 0)
  and (.allowed_internal_dependencies | length == (unique | length))
  and all(
    .allowed_internal_dependencies[];
    (.from | test("^[a-z0-9_-]+$"))
    and (.to | test("^(rullst|cargo-rullst)[a-z0-9_-]*$"))
    and (.optional | type == "boolean")
  )
' "$policy_path" >/dev/null || {
  echo "The crate architecture policy is malformed or contains duplicate edges."
  exit 1
}

cargo metadata --locked --format-version 1 --no-deps >"$metadata_path"

jq -r '
  .allowed_internal_dependencies[]
  | [.from, .to, (.optional | tostring)]
  | @tsv
' "$policy_path" | sort >"$expected_path"

jq -r '
  .packages[]
  | select(.publish != []) as $package
  | $package.dependencies[]
  | select(
      .path != null
      and (.name == "cargo-rullst" or (.name | startswith("rullst")))
    )
  | [$package.name, .name, (.optional | tostring)]
  | @tsv
' "$metadata_path" | sort >"$actual_path"

if ! diff -u "$expected_path" "$actual_path"; then
  echo "The publishable workspace crate graph changed without architecture-policy review."
  exit 1
fi

echo "The publishable workspace crate graph matches the reviewed architecture policy."
