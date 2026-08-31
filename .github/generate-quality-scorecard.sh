#!/usr/bin/env bash
set -euo pipefail

output_path="${1:-quality-scorecard.md}"
policy_path=".github/quality-scorecard-policy.json"
release_order_path=".github/release-order.json"
sha="${RULLST_SCORECARD_SHA:-unknown}"
checked_at="${RULLST_SCORECARD_TIME:-unknown}"

status_mark() {
  case "$1" in
    success) printf 'pass' ;;
    skipped) printf 'skipped' ;;
    cancelled) printf 'cancelled' ;;
    *) printf 'fail' ;;
  esac
}

all_success() {
  local status
  for status in "$@"; do
    [[ "$status" == "success" ]] || return 1
  done
}

grade_for() {
  local score="$1"
  if ((score >= 97)); then
    printf 'A+'
  elif ((score >= 90)); then
    printf 'A'
  elif ((score >= 80)); then
    printf 'B'
  elif ((score >= 70)); then
    printf 'C'
  elif ((score >= 60)); then
    printf 'D'
  else
    printf 'F'
  fi
}

check_status="${RULLST_SCORECARD_CHECK:-unknown}"
test_status="${RULLST_SCORECARD_TESTS:-unknown}"
feature_status="${RULLST_SCORECARD_FEATURES:-unknown}"
msrv_status="${RULLST_SCORECARD_MSRV:-unknown}"
strict_status="${RULLST_SCORECARD_STRICT_DB:-unknown}"
redis_status="${RULLST_SCORECARD_REDIS:-unknown}"
threat_status="${RULLST_SCORECARD_THREAT:-unknown}"
ai_status="${RULLST_SCORECARD_AI_EVALS:-unknown}"
local_access_status="${RULLST_SCORECARD_LOCAL_ACCESS:-unknown}"

jq -e '
  .schema_version == 1
  and .dimensions == {
    "api_architecture": 20,
    "verification": 25,
    "security_failure_design": 20,
    "documentation_dx": 15,
    "operations_release": 20
  }
  and (.crates | length > 0)
  and (.crates | length == ([.[].name] | unique | length))
  and all(
    .crates[];
    (.name | test("^[a-z0-9_-]+$"))
    and (.api_architecture | type == "number" and . >= 0 and . <= 20)
    and (.verification | type == "number" and . >= 0 and . <= 25)
    and (.security_failure_design | type == "number" and . >= 0 and . <= 20)
    and (.documentation_dx | type == "number" and . >= 0 and . <= 15)
    and (.operations_release | type == "number" and . >= 0 and . <= 20)
    and (.specialist_gate | IN("none", "strict-database-redis", "redis", "redis-and-threat", "messaging-contract", "threat-minimum", "ai-evals", "provider-matrix", "release-local-access"))
    and (.finding | type == "string" and length > 0)
    and (.evidence | type == "array" and length > 0)
  )
' "$policy_path" >/dev/null || {
  echo "The quality scorecard policy is malformed." >&2
  exit 1
}

mapfile -t packages < <(jq -r '.[]' "$release_order_path")
mapfile -t audited_packages < <(jq -r '.crates[].name' "$policy_path")
if ! diff -u \
  <(printf '%s\n' "${packages[@]}" | sort) \
  <(printf '%s\n' "${audited_packages[@]}" | sort); then
  echo "The scorecard must audit exactly the publishable release train." >&2
  exit 1
fi

while IFS= read -r evidence_path; do
  [[ -e "$evidence_path" ]] || {
    echo "Missing scorecard evidence path: $evidence_path" >&2
    exit 1
  }
done < <(jq -r '.crates[].evidence[]' "$policy_path" | sort -u)

{
  echo "# Rullst quality scorecard"
  echo
  echo "- Commit: \`$sha\`"
  echo "- Generated: $checked_at"
  echo "- Audit policy date: $(jq -r '.audit_date' "$policy_path")"
  echo "- Scope: audited per-crate ceilings constrained by this exact workflow run"
  echo
  echo "> A score is not feature completeness, production readiness, security certification, provider homologation, or a comparison with another framework. A green gate cannot award more than the committed audited ceiling."
  echo
  echo "| Crate | Score | Grade | API /20 | Verification /25 | Security /20 | Docs /15 | Operations /20 | Specialist gate | Audited finding |"
  echo "| :--- | ---: | :---: | ---: | ---: | ---: | ---: | ---: | :--- | :--- |"
} >"$output_path"

repository_achieved=0
for package in "${packages[@]}"; do
  record="$(jq -c --arg package "$package" '.crates[] | select(.name == $package)' "$policy_path")"
  api_ceiling="$(jq -r '.api_architecture' <<<"$record")"
  verification_ceiling="$(jq -r '.verification' <<<"$record")"
  security_ceiling="$(jq -r '.security_failure_design' <<<"$record")"
  docs_score="$(jq -r '.documentation_dx' <<<"$record")"
  operations_ceiling="$(jq -r '.operations_release' <<<"$record")"
  specialist="$(jq -r '.specialist_gate' <<<"$record")"
  finding="$(jq -r '.finding' <<<"$record" | tr '|' '/')"

  api_score=0
  api_mark="fail"
  if all_success "$check_status" "$feature_status" "$msrv_status"; then
    api_score="$api_ceiling"
    api_mark="pass"
  fi

  verification_score=0
  verification_mark="fail"
  if [[ "$test_status" == "success" ]]; then
    verification_score="$verification_ceiling"
    verification_mark="pass"
  fi

  specialist_status="success"
  case "$specialist" in
    none|messaging-contract|provider-matrix)
      specialist_status="$test_status"
      ;;
    strict-database-redis)
      all_success "$strict_status" "$redis_status" || specialist_status="failure"
      ;;
    redis)
      specialist_status="$redis_status"
      ;;
    redis-and-threat)
      all_success "$redis_status" "$threat_status" || specialist_status="failure"
      ;;
    threat-minimum)
      specialist_status="$threat_status"
      ;;
    ai-evals)
      specialist_status="$ai_status"
      ;;
    release-local-access)
      specialist_status="$local_access_status"
      ;;
  esac

  security_score=0
  security_mark="fail"
  if all_success "$check_status" "$specialist_status"; then
    security_score="$security_ceiling"
    security_mark="pass"
  fi

  operations_score=0
  operations_mark="fail"
  if all_success "$feature_status" "$msrv_status" "$specialist_status"; then
    operations_score="$operations_ceiling"
    operations_mark="pass"
  fi

  score=$((api_score + verification_score + security_score + docs_score + operations_score))
  grade="$(grade_for "$score")"
  repository_achieved=$((repository_achieved + score))

  printf '| `%s` | **%d/100** | **%s** | %d (%s) | %d (%s) | %d (%s) | %d | %d (%s) | %s: %s | %s |\n' \
    "$package" "$score" "$grade" "$api_score" "$api_mark" \
    "$verification_score" "$verification_mark" "$security_score" \
    "$security_mark" "$docs_score" "$operations_score" "$operations_mark" \
    "$specialist" "$(status_mark "$specialist_status")" "$finding" >>"$output_path"
done

repository_denominator=$((${#packages[@]} * 100))
repository_score=$(((repository_achieved * 100 + repository_denominator / 2) / repository_denominator))
repository_grade="$(grade_for "$repository_score")"

{
  echo
  echo "## Repository score"
  echo
  echo "**$repository_score/100 ($repository_grade)** — $repository_achieved/$repository_denominator audited points across ${#packages[@]} crates."
  echo
  echo "The repository score is the equal-crate aggregate. Capability progress is reported separately. Failed/cancelled/skipped applicable gates suppress the dimensions they are meant to prove; they never increase a ceiling."
  echo
  echo "Global evidence: check=$(status_mark "$check_status"), tests=$(status_mark "$test_status"), features=$(status_mark "$feature_status"), MSRV=$(status_mark "$msrv_status"), threat=$(status_mark "$threat_status"), AI evals=$(status_mark "$ai_status"), strict databases=$(status_mark "$strict_status"), Redis live=$(status_mark "$redis_status"), release local-access=$(status_mark "$local_access_status")."
} >>"$output_path"
