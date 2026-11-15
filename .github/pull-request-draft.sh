#!/usr/bin/env bash
# Write `draft=true|false` to $GITHUB_OUTPUT: whether this run's pull request
# is a draft now, read from the API rather than from the event payload.
#
# The payload is a snapshot of the pull request when its event was queued.
# When a push and `ready_for_review` arrive together, the concurrency group
# keeps the newest run, which can be the push whose payload still says
# `draft: true`; jobs gated on that payload are skipped and the required
# `Run Tests (...)` matrix checks never report (pull request #668). The newest
# run starts after both events, so the live state it reads here is current.
#
# Fails open: another event, a missing number or an API failure reports
# `draft=false`, so the ready pull-request jobs run rather than leave a
# required check unreported. The token needs `pull-requests: read`, which a
# fork's read-only token also has.
set -uo pipefail

draft=false
if [[ "${GITHUB_EVENT_NAME:-}" == pull_request ]]; then
  if [[ ! "${PR_NUMBER:-}" =~ ^[1-9][0-9]*$ ||
        ! "${GITHUB_REPOSITORY:-}" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
    echo "::warning::No pull request number or repository; running the ready pull-request jobs."
  elif state="$(gh api "repos/$GITHUB_REPOSITORY/pulls/$PR_NUMBER" --jq '.draft')"; then
    case "$state" in
      true) draft=true ;;
      false) ;;
      *) echo "::warning::The API returned no draft state for pull request #$PR_NUMBER; running the ready pull-request jobs." ;;
    esac
  else
    echo "::warning::Could not read pull request #$PR_NUMBER; running the ready pull-request jobs."
  fi
  echo "Pull request #${PR_NUMBER:-?}: event payload draft=${PAYLOAD_DRAFT:-unknown}, current draft=$draft"
fi
printf 'draft=%s\n' "$draft" >> "$GITHUB_OUTPUT"
