#!/usr/bin/env bash
set -euo pipefail

dataset="rullst-ai/evals/guardrails-v1.json"

jq -e '
  .schema_version == 1
  and .suite_id == "rullst-ai-guardrails-v1"
  and (.scope | type == "string" and length > 0)
  and (.cases | length >= 8)
  and ([.cases[].id] | length == (unique | length))
  and ([.cases[].category] | unique | sort == ["jailbreak", "pii", "prompt_injection"])
' "$dataset" >/dev/null || {
  echo "Versioned AI eval dataset is malformed or missing a required category."
  exit 1
}

cargo test -p rullst-ai --all-features --test versioned_evals \
  versioned_guardrail_evals_match_every_offline_provider -- --exact
