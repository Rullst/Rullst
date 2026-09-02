#!/usr/bin/env bash
set -euo pipefail

manifest_path=".github/threat-model-release-minimum.json"
model_path="docs/src/threat-models.md"

jq -e '
  .schema_version == 1
  and .model_version == "TM-12.4"
  and (.cases | length > 0)
  and ([.cases[].id] | unique | sort == [
    "ACADEMY-02",
    "ACADEMY-03",
    "ACADEMY-04",
    "ACADEMY-05",
    "ACADEMY-06",
    "ACADEMY-07",
    "ACADEMY-08",
    "ACADEMY-09",
    "ACADEMY-10",
    "ACADEMY-12",
    "AI-01",
    "AI-02",
    "AI-03",
    "AI-05",
    "AI-06",
    "AI-08",
    "AUTH-01",
    "AUTH-02",
    "AUTH-04",
    "AUTH-05",
    "AUTH-06",
    "AUTH-07",
    "DEPLOY-05",
    "DEPLOY-06",
    "IOT-01",
    "MAIL-01",
    "MAIL-02",
    "MAIL-03",
    "NEXUS-01",
    "NEXUS-05",
    "PAY-01",
    "PAY-06",
    "PAY-07",
    "SEC-07",
    "SEC-16",
    "STUDIO-01",
    "STUDIO-03",
    "STUDIO-04",
    "STUDIO-06",
    "TENANT-02",
    "TENANT-04"
  ])
' "$manifest_path" >/dev/null || {
  echo "Threat-model release-minimum manifest is malformed or incomplete."
  exit 1
}

model_version="$(jq -er '.model_version' "$manifest_path")"
grep -Fq -- "**Model version:** ${model_version}" "$model_path" || {
  echo "Threat-model version ${model_version} is not declared by ${model_path}."
  exit 1
}

executed_tests=()

while IFS=$'\t' read -r case_id crate target_kind target test_filter source marker; do
  if [[ ! "$case_id" =~ ^[A-Z]+-[0-9]{2}$ \
    || ! "$crate" =~ ^[a-z0-9_-]+$ \
    || ! "$target_kind" =~ ^(lib|integration)$ \
    || ! "$target" =~ ^[a-zA-Z0-9_-]*$ \
    || ! "$test_filter" =~ ^[a-zA-Z0-9_:]+$ \
    || ! "$source" =~ ^[a-zA-Z0-9_./-]+$ \
    || "$source" == *".."* \
    || "$marker" != "TM-${case_id}" ]]; then
    echo "Unsafe or inconsistent threat-model evidence row for ${case_id}."
    exit 1
  fi
  if [ ! -f "$source" ]; then
    echo "Missing threat-model evidence source: ${source}."
    exit 1
  fi
  case_reference="\`${case_id}\`"
  grep -Fq -- "$case_reference" "$model_path" || {
    echo "Threat-model case ${case_id} is not declared by ${model_path}."
    exit 1
  }
  grep -Fq -- "$marker" "$source" || {
    echo "Evidence source ${source} does not reference ${marker}."
    exit 1
  }

  test_key="${crate}:${target_kind}:${target}:${test_filter}"
  already_executed=false
  for executed_test in "${executed_tests[@]}"; do
    if [ "$executed_test" = "$test_key" ]; then
      already_executed=true
      break
    fi
  done
  if [ "$already_executed" = true ]; then
    echo "Reusing exact test for ${case_id}: ${test_filter}"
    continue
  fi
  executed_tests+=("$test_key")

  echo "Running ${case_id}: ${crate} ${test_filter}"
  case "$target_kind" in
    lib)
      test_list="$(cargo test -p "$crate" --all-features --lib -- --list)"
      if ! grep -Fxq -- "${test_filter}: test" <<<"$test_list"; then
        echo "Threat-model evidence test does not exist: ${test_filter}."
        exit 1
      fi
      cargo test -p "$crate" --all-features --lib "$test_filter" -- --exact
      ;;
    integration)
      if [ -z "$target" ]; then
        echo "Integration evidence ${case_id} is missing a target name."
        exit 1
      fi
      test_list="$(cargo test -p "$crate" --all-features --test "$target" -- --list)"
      if ! grep -Fxq -- "${test_filter}: test" <<<"$test_list"; then
        echo "Threat-model evidence test does not exist: ${test_filter}."
        exit 1
      fi
      cargo test -p "$crate" --all-features --test "$target" "$test_filter" -- --exact
      ;;
  esac
done < <(jq -r '.cases[] | [.id, .crate, .target_kind, .target, .test_filter, .source, .marker] | @tsv' "$manifest_path")
