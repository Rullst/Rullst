#!/usr/bin/env bash
set -euo pipefail

output_path="${1:-target/package/crates-ownership.json}"
policy_path=".github/crates-ownership-policy.json"
release_order_path=".github/release-order.json"
expected_owner="$(jq -er '.expected_owner' "$policy_path")"
response_path="$(mktemp)"
rows_path="$(mktemp)"
trap 'rm -f -- "$response_path" "$rows_path"' EXIT

jq -e --slurpfile order "$release_order_path" '
  (.bootstrap_unregistered | length) == (.bootstrap_unregistered | unique | length)
  and all(.bootstrap_unregistered[]; $order[0] | index(.) != null)
' "$policy_path" >/dev/null || {
  echo "The crates.io bootstrap policy contains duplicates or unknown package names."
  exit 1
}

has_unregistered=false
while IFS= read -r crate_name; do
  if [[ ! "$crate_name" =~ ^[a-z0-9_-]+$ ]]; then
    echo "Unsafe crate name in release order: $crate_name"
    exit 1
  fi

  http_status="$(curl --silent --show-error --location \
    --retry 3 --retry-all-errors --connect-timeout 10 --max-time 30 \
    --output "$response_path" --write-out '%{http_code}' \
    --user-agent 'rullst-release-ownership-audit/1' \
    "https://crates.io/api/v1/crates/${crate_name}/owners")"

  case "$http_status" in
    200)
      owners="$(jq -cer '[.users[]?.login, .teams[]?.login] | map(select(. != null))' "$response_path")"
      if ! jq -e --arg owner "$expected_owner" \
        '[.users[]?.login, .teams[]?.login] | index($owner) != null' \
        "$response_path" >/dev/null; then
        echo "Registered crate $crate_name is not owned by expected owner $expected_owner."
        exit 1
      fi
      jq -cn \
        --arg crate "$crate_name" \
        --argjson owners "$owners" \
        '{crate: $crate, registry_status: "registered", owners: $owners}' \
        >>"$rows_path"
      printf '%s\tregistered\t%s\n' "$crate_name" "$expected_owner"
      ;;
    404)
      if ! jq -e --arg crate "$crate_name" \
        '.bootstrap_unregistered | index($crate) != null' \
        "$policy_path" >/dev/null; then
        echo "Unregistered crate $crate_name is not in the reviewed bootstrap allowlist."
        exit 1
      fi
      has_unregistered=true
      jq -cn \
        --arg crate "$crate_name" \
        '{crate: $crate, registry_status: "unregistered-reviewed-bootstrap", owners: []}' \
        >>"$rows_path"
      printf '%s\tunregistered-reviewed-bootstrap\t-\n' "$crate_name"
      ;;
    *)
      echo "crates.io owner lookup failed for $crate_name with HTTP $http_status."
      exit 1
      ;;
  esac
done < <(jq -er '.[]' "$release_order_path")

mkdir -p "$(dirname "$output_path")"
jq -s \
  --arg expected_owner "$expected_owner" \
  --arg checked_at "$(date -u +'%Y-%m-%dT%H:%M:%SZ')" \
  --argjson has_unregistered "$has_unregistered" \
  '{
    schema_version: 1,
    checked_at: $checked_at,
    expected_owner: $expected_owner,
    has_unregistered: $has_unregistered,
    packages: .
  }' "$rows_path" >"$output_path"

if [ -n "${GITHUB_OUTPUT:-}" ]; then
  echo "has_unregistered=$has_unregistered" >>"$GITHUB_OUTPUT"
fi
