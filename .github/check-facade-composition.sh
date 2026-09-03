#!/usr/bin/env bash
set -euo pipefail

cargo test --locked --package rullst --no-default-features \
  --features auth-sqlite,capital-quota-sql,mail-sqlite,messaging-sqlite,oauth-sqlite,queue-sqlite \
  --test facade_recovery \
  facade_shared_local_profile_recovers_and_fails_closed -- --exact
