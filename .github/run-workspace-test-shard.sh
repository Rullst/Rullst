#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 <workspace|cli-standard|cli-updates|cli-profiles-basic|cli-profiles-relational|cli-profiles-polyglot|cli-lms|cli-saas-foundation|cli-saas-product|cli-saas-journey> [--release]" >&2
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
if [[ "$shard" == cli-* && "$shard" != cli-saas-journey ]]; then
  cargo fetch --locked
fi

case "$shard" in
  cli-saas-journey)
    # Explicit Linux diagnostic, outside the default matrix and release admission.
    if [[ "${#profile_args[@]}" -ne 0 ]]; then
      echo 'The SaaS journey currently measures a source-installed debug CLI only.' >&2
      exit 2
    fi
    python3 examples/saas/run.py
    ;;
  workspace)
    cargo test --workspace --exclude cargo-rullst \
      --all-features --no-fail-fast "${profile_args[@]}"
    ;;
  cli-updates)
    # A bounded native diagnostic; cli-standard still owns every CLI assertion
    # in full release matrices. This subset is never release admission evidence.
    cargo test -p cargo-rullst -p rullst-core --all-features \
      "${profile_args[@]}" --lib update::
    cargo test -p cargo-rullst -p rullst-core --all-features --no-fail-fast \
      "${profile_args[@]}" --test update_discovery_cli --test update_verification_cli \
      --test update_project_cli --test upgrade_cli
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
  cli-profiles-basic)
    RULLST_CI_PROFILE_GROUP=basic \
      cargo test -p cargo-rullst -p rullst-core --all-features --no-fail-fast \
        "${profile_args[@]}" --test generated_cli_profiles
    ;;
  cli-profiles-relational)
    RULLST_CI_PROFILE_GROUP=relational \
      cargo test -p cargo-rullst -p rullst-core --all-features --no-fail-fast \
        "${profile_args[@]}" --test generated_cli_profiles
    ;;
  cli-profiles-polyglot)
    RULLST_CI_PROFILE_GROUP=polyglot \
      cargo test -p cargo-rullst -p rullst-core --all-features --no-fail-fast \
        "${profile_args[@]}" --test generated_cli_profiles
    ;;
  cli-profiles)
    # Backwards-compatible local alias; CI uses the bounded groups above.
    cargo test -p cargo-rullst -p rullst-core --all-features --no-fail-fast \
      "${profile_args[@]}" --test generated_cli_profiles
    ;;
  cli-lms)
    cargo test -p cargo-rullst -p rullst-core --all-features --no-fail-fast \
      "${profile_args[@]}" --test generated_lms_modules_check
    ;;
  cli-saas-foundation)
    # The full matrix already materializes LMS with the same helper and case.
    # Exclude only its duplicate threat-evidence wrapper, never an application
    # assertion. --exact prevents a future similarly named test being skipped.
    RULLST_CI_GENERATED_GROUP=foundation \
      cargo test -p cargo-rullst -p rullst-core --all-features --no-fail-fast \
        "${profile_args[@]}" --test generated_saas_check \
        -- --exact --skip materialized_lms_executes_security_contracts
    ;;
  cli-saas-product)
    RULLST_CI_GENERATED_GROUP=product \
      cargo test -p cargo-rullst -p rullst-core --all-features --no-fail-fast \
        "${profile_args[@]}" --test generated_saas_check \
        -- --exact --skip materialized_lms_executes_security_contracts
    ;;
  cli-saas)
    # Backwards-compatible local alias; CI uses the bounded groups above.
    cargo test -p cargo-rullst -p rullst-core --all-features --no-fail-fast \
      "${profile_args[@]}" --test generated_saas_check \
      -- --exact --skip materialized_lms_executes_security_contracts
    ;;
  *)
    echo "unknown workspace test shard: $shard" >&2
    exit 2
    ;;
esac
