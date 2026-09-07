#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 <workspace|cli-standard|cli-profiles|cli-lms|cli-saas> [--release]" >&2
  exit 2
fi

shard=$1
profile_args=()

case "${2:-}" in
  "")
    ;;
  --release)
    profile_args+=(--release)
    ;;
  *)
    echo "unsupported test profile: $2" >&2
    exit 2
    ;;
esac

# Generated-project checks deliberately invoke Cargo offline. The monolithic
# workspace command used to populate every locked package first; isolated CLI
# shards must preserve that precondition without recompiling the workspace.
if [[ "$shard" == cli-* ]]; then
  cargo fetch --locked
fi

case "$shard" in
  workspace)
    cargo test --workspace --exclude cargo-rullst \
      --all-features --no-fail-fast "${profile_args[@]}"
    ;;
  cli-standard)
    test_targets=(--lib --bins --examples)
    while IFS= read -r test_source; do
      test_target="$(basename "$test_source" .rs)"
      case "$test_target" in
        generated_cli_profiles|generated_lms_modules_check|generated_saas_check)
          continue
          ;;
      esac
      test_targets+=(--test "$test_target")
    done < <(find cargo-rullst/tests -maxdepth 1 -type f -name '*.rs' -print | sort)

    # Selecting Core as well preserves the all-workspace feature unification
    # seen by cargo-rullst's only local framework dependency. Core's library
    # assertions may repeat here; no CLI assertion is weakened or omitted.
    cargo test -p cargo-rullst -p rullst-core --all-features --no-fail-fast \
      "${profile_args[@]}" "${test_targets[@]}"
    cargo test -p cargo-rullst -p rullst-core --all-features \
      "${profile_args[@]}" --doc
    ;;
  cli-profiles)
    cargo test -p cargo-rullst -p rullst-core --all-features --no-fail-fast \
      "${profile_args[@]}" --test generated_cli_profiles
    ;;
  cli-lms)
    cargo test -p cargo-rullst -p rullst-core --all-features --no-fail-fast \
      "${profile_args[@]}" --test generated_lms_modules_check
    ;;
  cli-saas)
    cargo test -p cargo-rullst -p rullst-core --all-features --no-fail-fast \
      "${profile_args[@]}" --test generated_saas_check
    ;;
  *)
    echo "unknown workspace test shard: $shard" >&2
    exit 2
    ;;
esac
