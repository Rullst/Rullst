#!/usr/bin/env bash
set -euo pipefail

# Keep every public package usable without implicit default features. Feature
# rows below then exercise the boundaries most likely to regress when optional
# adapters or the umbrella crate change their dependency wiring.
while IFS='|' read -r package features; do
  [[ -z "$package" || "$package" == \#* ]] && continue

  targets=(--all-targets)
  if [[ "$package" == "rullst" && -n "$features" ]]; then
    # These rows verify the umbrella library's public feature forwarding. Its
    # tests/benches bring unrelated dev-dependencies into each isolated graph
    # and can rebuild large native adapters (notably DuckDB) for no extra
    # boundary coverage; the no-feature row above still checks all targets.
    targets=(--lib)
  fi

  command=(
    cargo check
    --locked
    --package "$package"
    "${targets[@]}"
    --no-default-features
  )
  if [[ -n "$features" ]]; then
    command+=(--features "$features")
  fi

  printf 'Checking %-20s features=%s\n' "$package" "${features:-<none>}"
  "${command[@]}"
done <<'MATRIX'
# All 15 publishable packages without default features.
rullst-macros|
rullst-orm-macros|
rullst-orm|
rullst-core|
rullst-connect|
rullst-iot|
rullst-security|
rullst-ai|
rullst-capital|
rullst-mail|
rullst-auth|
rullst-nexus|
rullst-studio|
rullst|
cargo-rullst|

# Independent infrastructure and adapter boundaries.
rullst-orm|redis
rullst-orm|mongodb
rullst-orm|duckdb
rullst-orm|turso
rullst-orm|surrealdb
rullst-orm|scout-http
rullst-orm|polyglot
rullst-core|orm
rullst-core|queue-sqlite
rullst-core|queue-redis
rullst-core|cache-redis
rullst-core|telemetry
rullst-connect|axum
rullst-connect|actix
rullst-connect|retry
rullst-connect|axum-session
rullst-iot|std
rullst-iot|experimental-simulators
rullst-security|redis-rate-limit
rullst-capital|axum
rullst-mail|mail-smtp
rullst-auth|jwt

# Umbrella boundaries exposed to generated applications.
rullst|orm
rullst|orm-mongodb
rullst|orm-duckdb
rullst|orm-turso
rullst|orm-surrealdb
rullst|orm-scout
# `orm-polyglot` is the exact union of the four isolated forwarding rows above
# and remains compiled by the workspace all-feature test/Clippy gates. Repeating
# it here rebuilds bundled DuckDB for a third graph without testing a new edge.
rullst|queue-sqlite
rullst|queue-redis
rullst|cache-redis
rullst|mail-smtp
rullst|auth-jwt
rullst|oauth
rullst|ai
rullst|capital
rullst|security
rullst|security-redis
rullst|iot
rullst|telemetry
rullst|nexus
rullst|studio
MATRIX

printf 'Testing  %-20s features=%s\n' "rullst-core" "<none>"
cargo test --locked --package rullst-core --no-default-features
