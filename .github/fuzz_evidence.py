#!/usr/bin/env python3
"""Plan and independently verify fresh or equivalent complete fuzz evidence."""

from __future__ import annotations

import argparse
from datetime import datetime, timedelta, timezone
import json
import os
from pathlib import Path
import re
import sys
import urllib.parse
import urllib.request

from fuzz_evidence_inputs import (
    DOC_REVIEW, MAINTENANCE_DOC_REVIEW, ROOT, SHA, SCOPE_REVIEW,
    FuzzSurfaceChanged, Snapshot, digest,
)
from release_line import EVIDENCE_BRANCHES

SECONDS = 19_800
MAX_AGE = timedelta(days=7)
MAX_RUNS = 30
WORKFLOW = ".github/workflows/fuzzing.yml"
BOUNDARY = "Fuzz campaign evidence boundary"


def timestamp(value: object) -> datetime:
    if not isinstance(value, str):
        raise ValueError("missing evidence timestamp")
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        raise ValueError("evidence timestamp must include a timezone")
    return parsed


class GitHub:
    def __init__(self, repository: str, token: str, api: str = "https://api.github.com",
                 *, branch: str = "main"):
        if re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository) is None:
            raise ValueError("repository must be OWNER/REPO")
        if api != "https://api.github.com":
            raise ValueError("fuzz evidence accepts the GitHub.com API only")
        if not token:
            raise ValueError("GITHUB_TOKEN is required")
        if branch not in EVIDENCE_BRANCHES:
            raise ValueError("unsupported fuzz evidence release branch")
        self.repository, self.token, self.api = repository, token, api
        self.branch = branch

    def get(self, suffix: str) -> dict:
        request = urllib.request.Request(f"{self.api}/repos/{self.repository}/{suffix}", headers={
            "Accept": "application/vnd.github+json", "Authorization": f"Bearer {self.token}",
            "X-GitHub-Api-Version": "2022-11-28", "User-Agent": "rullst-fuzz-evidence/1",
        })
        with urllib.request.urlopen(request, timeout=30) as response:
            payload = json.load(response)
        if not isinstance(payload, dict):
            raise ValueError("invalid GitHub evidence response")
        return payload

    def runs(self) -> list[dict]:
        query = urllib.parse.urlencode({"branch": self.branch, "event": "workflow_dispatch",
                                      "per_page": MAX_RUNS})
        payload = self.get(f"actions/workflows/fuzzing.yml/runs?{query}")
        runs = payload.get("workflow_runs")
        if not isinstance(runs, list):
            raise ValueError("invalid workflow run inventory")
        return runs

    def jobs(self, run: dict) -> list[dict]:
        # Attempt-specific URLs prevent a rerun from mixing two job inventories.
        payload = self.get(f'actions/runs/{run["id"]}/attempts/{run["run_attempt"]}/jobs?per_page=100')
        jobs = payload.get("jobs")
        if (not isinstance(jobs, list) or type(payload.get("total_count")) is not int
                or payload["total_count"] != len(jobs)):
            raise ValueError("incomplete job inventory; cannot reuse evidence")
        return jobs


def eligible_run(run: dict, repository: str, now: datetime, branch: str = "main") -> bool:
    try:
        return (type(run.get("id")) is int and run["id"] > 0
                and type(run.get("run_attempt")) is int and run["run_attempt"] > 0
                and isinstance(run.get("head_sha"), str) and SHA.fullmatch(run["head_sha"]) is not None
                and branch in EVIDENCE_BRANCHES and run.get("head_branch") == branch
                and run.get("event") == "workflow_dispatch"
                and run.get("path") == WORKFLOW
                and run.get("repository", {}).get("full_name") == repository
                and run.get("head_repository", {}).get("full_name") == repository
                and now - MAX_AGE <= timestamp(run.get("created_at")) <= now)
    except (ValueError, TypeError, AttributeError):
        return False


def unique_jobs(jobs: list[dict]) -> dict[str, dict]:
    result = {}
    for job in jobs:
        if not isinstance(job, dict) or not isinstance(job.get("name"), str) or job["name"] in result:
            raise ValueError("invalid or duplicate job names")
        result[job["name"]] = job
    return result


def succeeded(job: dict) -> bool:
    return job.get("status") == "completed" and job.get("conclusion") == "success"


def full_duration(job: dict, now: datetime) -> bool:
    steps = job.get("steps")
    if not isinstance(steps, list):
        return False
    campaigns = [step for step in steps if isinstance(step, dict)
                 and step.get("name") == "Run bounded target campaign"]
    if len(campaigns) != 1 or not succeeded(campaigns[0]):
        return False
    try:
        start = timestamp(campaigns[0].get("started_at"))
        end = timestamp(campaigns[0].get("completed_at"))
        return end <= now and (end - start).total_seconds() >= SECONDS
    except (ValueError, TypeError):
        return False


def plan(candidate: Snapshot, github: GitHub, now: datetime | None = None,
         force_full: bool = False) -> dict:
    if github.branch != candidate.release_branch:
        raise ValueError("fuzz evidence branch differs from the candidate release policy")
    now = now or datetime.now(timezone.utc)
    reused: dict[str, dict] = {}
    blocked: set[str] = set()
    considered = []
    runs = [] if force_full else github.runs()
    runs = [run for run in runs if isinstance(run, dict)
            and eligible_run(run, github.repository, now, candidate.release_branch)]
    # Newest evidence wins; never hide a newer matching failure behind an older pass.
    runs.sort(key=lambda run: (timestamp(run.get("run_started_at", run["created_at"])), run["id"]), reverse=True)
    for run in runs[:MAX_RUNS]:
        try:
            source = Snapshot(run["head_sha"], candidate.root)
        except FuzzSurfaceChanged:
            # A pre-expansion campaign certifies none of the new surface. The
            # candidate itself is still required to have the exact reviewed count.
            continue
        if (source.release_branch != candidate.release_branch
                or not source.ancestor_of(candidate) or source.contract != candidate.contract):
            continue
        equivalent = [item for item in candidate.inventory
                      if item["dir"] in source.packages
                      and source.fingerprint(item["dir"]) == candidate.fingerprint(item["dir"])]
        if not equivalent:
            continue
        jobs = unique_jobs(github.jobs(run))
        considered.append({"run_id": run["id"], "attempt": run["run_attempt"], "sha": source.sha})
        boundary = jobs.get(BOUNDARY, {})
        # A success alone is insufficient: every credited target needs its own
        # completed long-running step and package preflight from that attempt.
        campaign_ok = (run.get("status") == "completed" and run.get("conclusion") == "success"
                       and succeeded(boundary))
        for item in equivalent:
            target, directory = item["target"], item["dir"]
            if target in reused or target in blocked:
                continue
            job = jobs.get(f"Fuzz {target}")
            if job is None or job.get("conclusion") == "skipped":
                continue  # Reused targets never create synthetic successful jobs.
            preflight = jobs.get(f"Compile fuzz package {directory}", {})
            if job.get("run_id") != run["id"] or job.get("head_sha") != source.sha:
                raise ValueError("job provenance does not match its source run")
            if (campaign_ok and succeeded(job) and succeeded(preflight) and full_duration(job, now)):
                if type(job.get("id")) is not int or job["id"] <= 0:
                    raise ValueError("invalid source job ID")
                reused[target] = {**item, "source_sha": source.sha, "run_id": run["id"],
                                  "run_attempt": run["run_attempt"], "job_id": job["id"],
                                  "input_sha256": candidate.fingerprint(directory),
                                  "run_url": f"https://github.com/{github.repository}/actions/runs/{run['id']}",
                                  "completed_at": job.get("completed_at")}
            elif job.get("conclusion") in {"failure", "timed_out", "cancelled"}:
                blocked.add(target)
            elif run.get("status") != "completed":
                blocked.add(target)  # Wait for a newer attempt, never race it.
    selected = [item for item in candidate.inventory if item["target"] not in reused]
    return {"schema_version": 1, "candidate_sha": candidate.sha, "created_at": now.isoformat(),
            "repository": github.repository, "release_branch": candidate.release_branch,
            "campaign_seconds": SECONDS,
            "max_source_age_days": MAX_AGE.days, "total_targets": len(candidate.inventory),
            "execution_sha256": candidate.contract, "global_input_sha256": candidate.global_hash,
            "selected": selected, "reused": [reused[key] for key in sorted(reused)],
            "considered": considered, "blocked_by_newer_run": sorted(blocked),
            "policy_sha256": digest({name: (ROOT / name).read_text() for name in
                                      (".github/fuzz_evidence.py", ".github/fuzz_evidence_inputs.py",
                                       ".github/release_line.py", ".github/release-required-workflows.json",
                                       ".github/fuzz_dependency_inputs.py", SCOPE_REVIEW,
                                       ".github/test-fuzz-target-quality.py",
                                       DOC_REVIEW, MAINTENANCE_DOC_REVIEW)})}


def write_report(report: dict, path: Path) -> None:
    path.write_text(json.dumps(report, indent=2) + "\n")
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a") as output:
            output.write(f"\n### Verified fuzz inputs for `{report['candidate_sha']}`\n\n")
            output.write(f"Reused: {len(report['reused'])}; new execution required: {len(report['selected'])}. "
                         "Original results expire after seven days; reuse does not renew them.\n\n")
            for item in report["reused"]:
                output.write(f"- `{item['target']}`: [run {item['run_id']}]({item['run_url']}), "
                             f"attempt {item['run_attempt']}, source `{item['source_sha']}`, "
                             f"input `{item['input_sha256']}`.\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sha", default=os.environ.get("GITHUB_SHA", ""))
    parser.add_argument("--repository", default=os.environ.get("GITHUB_REPOSITORY", ""))
    parser.add_argument("--output", type=Path, default=Path("fuzz-evidence.json"))
    parser.add_argument("--force-full", action="store_true")
    parser.add_argument("--require-complete", action="store_true")
    args = parser.parse_args()
    try:
        candidate = Snapshot(args.sha)
        github = GitHub(args.repository, os.environ.get("GITHUB_TOKEN", ""), branch=candidate.release_branch)
        report = plan(candidate, github, force_full=args.force_full)
        write_report(report, args.output)
        print(f"Fuzz evidence: {len(report['reused'])}/{report['total_targets']} reusable; {len(report['selected'])} require execution.")
        if args.require_complete and report["selected"]:
            return 1
        return 0
    except (ValueError, KeyError, IndexError, OSError) as error:
        print(f"fuzz evidence: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
