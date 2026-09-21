#!/usr/bin/env bash
# Package exactly the admitted inventory; workspace membership is not admission.
set -euo pipefail
candidate=false
options=()
for argument in "$@"; do
  case "$argument" in
    --no-verify|--allow-dirty) options+=("$argument") ;;
    --v13-candidates) candidate=true ;;
    *) echo "Unsupported package-release argument: $argument" >&2; exit 1 ;;
  esac
done
jq --exit-status '
  type == "array" and length > 0 and length == (unique | length)
  and all(.[]; type == "string" and test("^[a-z][a-z0-9-]*$"))
' .github/release-order.json > /dev/null
mapfile -t crates < <(jq -r '.[]' .github/release-order.json)
if [ "$candidate" = true ]; then
  # Archive rehearsal only. The default release inventory stays unchanged.
  python3 - <<'PY'
import json, tomllib
from pathlib import Path
inventory = json.loads(Path('.github/release-order.json').read_text())
for name in ('rullst-supervision', 'rullst-media', 'rullst-labs', 'rullst-labs-runner'):
    package = tomllib.loads(Path(name, 'Cargo.toml').read_text())['package']
    assert name not in inventory, 'remove candidate mode after release admission'
    assert package['name'] == name and package['publish'] is False
PY
  crates+=(rullst-supervision rullst-media rullst-labs rullst-labs-runner)
  # Cargo deliberately omits publish=false crates from its temporary registry.
  # Resolve this unpublished candidate edge explicitly without admitting either
  # crate. Packaged manifests still have registry dependencies; the separate
  # consumer must replace this edge using only extracted archive bytes.
  options+=(--config 'patch.crates-io.rullst-labs.path="rullst-labs"')
fi
arguments=()
for crate in "${crates[@]}"; do arguments+=(--package "$crate"); done
"${CARGO:-cargo}" package "${arguments[@]}" --all-features --locked "${options[@]}"
