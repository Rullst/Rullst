#!/usr/bin/env python3
"""Require candidate workflows; independently verify equivalent fuzz inputs."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any, NoReturn


SHA = re.compile(r"[0-9A-Fa-f]{40}")
REPOSITORY = re.compile(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+")
WORKFLOW = re.compile(r"[A-Za-z0-9_.-]+\.ya?ml")
ALLOWED_EVENTS = {"push", "workflow_dispatch"}
MANDATORY_MANUAL_WORKFLOWS = frozenset(
    {
        "ci.yml",
        "dast-zap.yml",
        "fuzzing.yml",
        "kani.yml",
        "miri.yml",
        "omni-android.yml",
        "omni-desktop.yml",
        "omni-ios.yml",
        "proptest.yml",
        "sanitizers.yml",
    }
)


@dataclass(frozen=True)
class WorkflowRequirement:
    workflow: str
    event: str
    required_jobs: tuple[str, ...]


def fail(message: str) -> NoReturn:
    raise SystemExit(f"release admission: {message}")


def load_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        fail(f"cannot read {path}: {error}")
    if not isinstance(value, dict):
        fail(f"{path} must contain a JSON object")
    return value


def validate_policy(policy: dict[str, Any]) -> tuple[str, list[WorkflowRequirement]]:
    if policy.get("schema_version") != 2:
        fail("unsupported policy schema")
    branch = policy.get("required_branch")
    workflows = policy.get("workflows")
    if branch != "main":
        fail("required_branch must be main")
    if not isinstance(workflows, list) or not workflows:
        fail("workflows must be a non-empty list")

    requirements: list[WorkflowRequirement] = []
    seen: set[str] = set()
    for item in workflows:
        if not isinstance(item, dict):
            fail("each workflow requirement must be an object")
        workflow = item.get("workflow")
        event = item.get("event")
        required_jobs = item.get("required_jobs", [])
        if not isinstance(workflow, str) or WORKFLOW.fullmatch(workflow) is None:
            fail("workflow file name is invalid")
        if workflow in seen:
            fail("workflow file names must be unique")
        if event not in ALLOWED_EVENTS:
            fail(f"unsupported event for {workflow}")
        if (
            not isinstance(required_jobs, list)
            or any(not isinstance(job, str) or not job.strip() for job in required_jobs)
            or len(required_jobs) != len(set(required_jobs))
        ):
            fail(f"required_jobs for {workflow} must be a unique string list")
        if workflow == "ci.yml" and (event != "workflow_dispatch" or not required_jobs):
            fail("ci.yml must require the explicit full-platform manual matrix")
        expected_event = (
            "workflow_dispatch"
            if workflow in MANDATORY_MANUAL_WORKFLOWS
            else "push"
        )
        if event != expected_event:
            fail(f"{workflow} must use the {expected_event} event")
        if workflow == "fuzzing.yml":
            evidence_job = "Fuzz campaign evidence boundary"
            fuzz_jobs = [
                job
                for job in required_jobs
                if job.startswith("Fuzz ") and job != evidence_job
            ]
            if (
                len(fuzz_jobs) != 40
                or evidence_job not in required_jobs
            ):
                fail(
                    "fuzzing.yml must require all 40 target jobs and the evidence boundary"
                )
        seen.add(workflow)
        requirements.append(
            WorkflowRequirement(workflow, event, tuple(sorted(required_jobs)))
        )

    missing_manual = sorted(MANDATORY_MANUAL_WORKFLOWS - seen)
    if missing_manual:
        fail(
            "mandatory manual release workflows are missing: "
            + ", ".join(missing_manual)
        )
    return branch, sorted(requirements, key=lambda requirement: requirement.workflow)


def fetch_runs(
    api_url: str,
    repository: str,
    workflow: str,
    sha: str,
    branch: str,
    event: str,
    token: str,
) -> dict[str, Any]:
    query = urllib.parse.urlencode(
        {
            "branch": branch,
            "event": event,
            "head_sha": sha,
            "status": "completed",
            "per_page": 100,
        }
    )
    workflow_id = urllib.parse.quote(workflow, safe="")
    url = f"{api_url.rstrip('/')}/repos/{repository}/actions/workflows/{workflow_id}/runs?{query}"
    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {token}",
            "User-Agent": "rullst-release-admission/1",
            "X-GitHub-Api-Version": "2022-11-28",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            payload = json.load(response)
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as error:
        fail(f"cannot query {workflow}: {error}")
    if not isinstance(payload, dict):
        fail(f"GitHub returned an invalid response for {workflow}")
    return payload


def exact_successful_run_ids(
    payload: dict[str, Any], sha: str, branch: str, event: str
) -> list[int]:
    runs = payload.get("workflow_runs")
    if not isinstance(runs, list):
        return []
    return [
        run_id
        for run in runs
        if isinstance(run, dict)
        and run.get("head_sha") == sha
        and run.get("head_branch") == branch
        and run.get("event") == event
        and run.get("status") == "completed"
        and run.get("conclusion") == "success"
        and isinstance((run_id := run.get("id")), int)
        and run_id > 0
    ]


def fetch_jobs(
    api_url: str, repository: str, run_id: int, token: str
) -> dict[str, Any]:
    url = (
        f"{api_url.rstrip('/')}/repos/{repository}/actions/runs/{run_id}/jobs"
        "?filter=latest&per_page=100"
    )
    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {token}",
            "User-Agent": "rullst-release-admission/1",
            "X-GitHub-Api-Version": "2022-11-28",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            payload = json.load(response)
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as error:
        fail(f"cannot query jobs for run {run_id}: {error}")
    if not isinstance(payload, dict):
        fail(f"GitHub returned an invalid jobs response for run {run_id}")
    return payload


def required_jobs_succeeded(payload: dict[str, Any], required_jobs: tuple[str, ...]) -> bool:
    jobs = payload.get("jobs")
    total_count = payload.get("total_count")
    if not isinstance(jobs, list) or not isinstance(total_count, int):
        return False
    if total_count != len(jobs):
        return False

    observed: dict[str, list[str | None]] = {}
    for job in jobs:
        if not isinstance(job, dict) or not isinstance(job.get("name"), str):
            return False
        observed.setdefault(job["name"], []).append(job.get("conclusion"))
    return all(observed.get(name) == ["success"] for name in required_jobs)


def equivalent_fuzz_jobs(repository: str, sha: str, token: str, api_url: str,
                         required_jobs: tuple[str, ...]) -> bool:
    # A current successful fuzz workflow/boundary is still mandatory. This
    # recomputes coverage from original Git objects and attempt-specific jobs,
    # never from a caller-supplied report or a chain of reuse receipts.
    from fuzz_evidence import GitHub, Snapshot, plan, write_report

    candidate = Snapshot(sha)
    expected = {"Fuzz campaign evidence boundary", *(f"Fuzz {item['target']}" for item in candidate.inventory)}
    if set(required_jobs) != expected:
        raise ValueError("release policy does not match the complete fuzz inventory")
    report = plan(candidate, GitHub(repository, token, api_url))
    write_report(report, Path("release-fuzz-evidence.json"))
    return not report["selected"]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository", default=os.environ.get("GITHUB_REPOSITORY", ""))
    parser.add_argument("--sha", default=os.environ.get("GITHUB_SHA", ""))
    parser.add_argument("--token", default=os.environ.get("GITHUB_TOKEN", ""))
    parser.add_argument("--api-url", default=os.environ.get("GITHUB_API_URL", "https://api.github.com"))
    parser.add_argument(
        "--policy",
        type=Path,
        default=Path(".github/release-required-workflows.json"),
    )
    parser.add_argument(
        "--fixture-dir",
        type=Path,
        help="read WORKFLOW.json fixtures instead of querying GitHub",
    )
    args = parser.parse_args()

    if REPOSITORY.fullmatch(args.repository) is None:
        fail("repository must be OWNER/REPO")
    if SHA.fullmatch(args.sha) is None:
        fail("sha must be a full 40-character commit SHA")
    branch, requirements = validate_policy(load_object(args.policy))
    if args.fixture_dir is None and not args.token:
        fail("GITHUB_TOKEN is required for live admission")

    missing: list[str] = []
    for requirement in requirements:
        workflow = requirement.workflow
        if args.fixture_dir is not None:
            payload = load_object(args.fixture_dir / f"{workflow}.json")
        else:
            payload = fetch_runs(
                args.api_url,
                args.repository,
                workflow,
                args.sha,
                branch,
                requirement.event,
                args.token,
            )

        run_ids = exact_successful_run_ids(
            payload, args.sha, branch, requirement.event
        )
        if not run_ids:
            missing.append(workflow)
            continue
        if not requirement.required_jobs:
            continue

        has_complete_jobs = False
        for run_id in run_ids:
            if args.fixture_dir is not None:
                jobs_payload = load_object(
                    args.fixture_dir / f"{workflow}.{run_id}.jobs.json"
                )
            else:
                jobs_payload = fetch_jobs(
                    args.api_url, args.repository, run_id, args.token
                )
            if required_jobs_succeeded(jobs_payload, requirement.required_jobs):
                has_complete_jobs = True
                break
            if (workflow == "fuzzing.yml" and args.fixture_dir is None
                    and required_jobs_succeeded(jobs_payload, ("Fuzz campaign evidence boundary",))):
                try:
                    has_complete_jobs = equivalent_fuzz_jobs(
                        args.repository, args.sha, args.token, args.api_url, requirement.required_jobs
                    )
                except (ValueError, KeyError, IndexError, OSError) as error:
                    fail(f"cannot verify equivalent fuzz evidence: {error}")
                if has_complete_jobs:
                    break
        if not has_complete_jobs:
            missing.append(f"{workflow} (required job matrix incomplete)")

    if missing:
        print(
            "release admission: missing successful exact-SHA workflow evidence for:\n"
            + "\n".join(f"- {workflow}" for workflow in missing),
            file=sys.stderr,
        )
        return 1

    print(
        f"release admission verified {len(requirements)} required workflows for {args.sha}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
