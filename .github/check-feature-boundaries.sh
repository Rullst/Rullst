#!/usr/bin/env bash
set -euo pipefail

# Keep every public package usable without implicit default features. Feature
# rows below exercise every public umbrella feature and the package boundaries
# most likely to regress when optional adapters change dependency wiring.
declare -A checked_umbrella_features=()

while IFS='|' read -r package features; do
  [[ -z "$package" || "$package" == \#* ]] && continue

  if [[ "$package" == "rullst" && -n "$features" ]]; then
    checked_umbrella_features["$features"]=1
  fi

  targets=(--all-targets)
  if [[ -n "$features" ]]; then
    # Feature rows verify an isolated public library boundary. Tests/benches
    # bring unrelated dev-dependencies into each graph and can rebuild large
    # native adapters for no extra boundary coverage; each package's
    # no-feature row above still checks all targets, while the workspace test
    # and specialist jobs exercise feature-enabled integration targets.
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
# All 16 publishable packages without default features.
rullst-macros|
rullst-orm-macros|
rullst-orm|
rullst-core|
rullst-messaging|
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
rullst-orm|pgvector
rullst-orm|qdrant
rullst-orm|polyglot
rullst-core|orm
rullst-core|queue-sqlite
rullst-core|queue-redis
rullst-core|cache-redis
rullst-core|offline-sync
rullst-core|telemetry
rullst-messaging|sqlite
rullst-connect|axum
rullst-connect|actix
rullst-connect|retry
rullst-connect|axum-session
rullst-iot|std
rullst-iot|experimental-simulators
rullst-security|redis-rate-limit
rullst-ai|sql-memory
rullst-capital|axum
rullst-capital|actix
rullst-capital|nfse
rullst-capital|quota-sql
rullst-mail|mail-smtp
rullst-auth|jwt
rullst-auth|sqlite

# Umbrella boundaries exposed to generated applications.
rullst|orm
rullst|orm-mongodb
rullst|orm-duckdb
rullst|orm-turso
rullst|orm-surrealdb
rullst|orm-scout
rullst|orm-pgvector
rullst|orm-qdrant
rullst|orm-redis
rullst|orm-polyglot
rullst|queue-sqlite
rullst|queue-redis
rullst|cache-redis
rullst|redis
rullst|offline-sync
rullst|auth
rullst|mail-smtp
rullst|mailer
rullst|mail
rullst|mail-aws-ses
rullst|messaging
rullst|messaging-sqlite
rullst|auth-jwt
rullst|auth-sqlite
rullst|oauth
rullst|ai
rullst|ai-sql-memory
rullst|capital
rullst|capital-actix
rullst|capital-nfse
rullst|capital-quota-sql
rullst|capital-pdf
rullst|capital-mail
rullst|security
rullst|security-redis
rullst|iot
rullst|telemetry
rullst|nexus
rullst|studio
rullst|strict-postgres
rullst|strict-mysql
rullst|strict-sqlite
MATRIX

# Fail when a new public umbrella feature is added without an isolated row.
# This keeps the human-readable matrix synchronized with Cargo's actual
# additive feature graph instead of relying only on the all-feature build.
mapfile -t public_umbrella_features < <(
  python3 - <<'PY'
import tomllib
from pathlib import Path

manifest = tomllib.loads(Path("rullst/Cargo.toml").read_text(encoding="utf-8"))
for feature in sorted(manifest.get("features", {})):
    if feature != "default":
        print(feature)
PY
)

for feature in "${public_umbrella_features[@]}"; do
  if [[ -z "${checked_umbrella_features[$feature]:-}" ]]; then
    echo "Missing isolated umbrella feature boundary: rullst|$feature" >&2
    exit 1
  fi
done

printf 'Testing  %-20s features=%s\n' "rullst-core" "<none>"
cargo test --locked --package rullst-core --no-default-features
