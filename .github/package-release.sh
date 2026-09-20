#!/usr/bin/env bash
# Package exactly the admitted inventory; workspace membership is not admission.
set -euo pipefail
for argument in "$@"; do
  case "$argument" in
    --no-verify|--allow-dirty) ;;
    *) echo "Unsupported package-release argument: $argument" >&2; exit 1 ;;
  esac
done
jq --exit-status '
  type == "array" and length > 0 and length == (unique | length)
  and all(.[]; type == "string" and test("^[a-z][a-z0-9-]*$"))
' .github/release-order.json > /dev/null
mapfile -t crates < <(jq -r '.[]' .github/release-order.json)
arguments=()
for crate in "${crates[@]}"; do arguments+=(--package "$crate"); done
"${CARGO:-cargo}" package "${arguments[@]}" --all-features --locked "$@"
