#!/usr/bin/env bash
# Resolve every fuzz dependency graph without compiling or running a target.
set -euo pipefail

if [[ $# -gt 1 || ( $# -eq 1 && "$1" != --offline ) ]]; then
  echo "usage: $0 [--offline]" >&2
  exit 2
fi

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
python3 .github/validate-fuzz-targets.py
fuzz_directories="$(jq -er '[.[].dir] | unique | .[]' .github/fuzz-targets.json)"

while IFS= read -r fuzz_dir; do
  printf 'Checking locked dependency graph: %s\n' "$fuzz_dir"
  # --no-deps intentionally MUST NOT be used: it accepts a stale lockfile
  # without resolving changed path/workspace dependencies. This is the same
  # explicit platform used by the campaign's build and run commands.
  cargo metadata --manifest-path "$fuzz_dir/Cargo.toml" --locked \
    --format-version 1 --filter-platform x86_64-unknown-linux-gnu "$@" > /dev/null
done <<< "$fuzz_directories"
