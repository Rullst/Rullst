#!/usr/bin/env python3
"""Require that an admitted ci.yml run tested these exact release archives.

release.yml no longer repeats the archive-only consumer and installed-CLI test.
It is covered only if a successful, exact-SHA ci.yml release-matrix run on the
release branch packaged byte-identical archives: that run's
`packaged-checksums` artifact must equal the release's checksums.txt.

Usage: python3 .github/check-admitted-archives.py target/package/checksums.txt
(needs GH_TOKEN with actions:read, GITHUB_REPOSITORY and GITHUB_SHA).
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Callable

from release_line import policy_line

JOB = "Packaged distribution and installed CLI"
ARTIFACT = "packaged-checksums"
Gh = Callable[..., str]


def gh(*args: str) -> str:
    return subprocess.check_output(["gh", *args], text=True, stderr=subprocess.PIPE)


def candidate_runs(run: Gh, repository: str, sha: str, branch: str) -> list[int]:
    payload = json.loads(run(
        "api",
        f"repos/{repository}/actions/workflows/ci.yml/runs"
        f"?head_sha={sha}&branch={branch}&event=workflow_dispatch&status=completed&per_page=100",
    ))
    return [
        item["id"] for item in payload.get("workflow_runs", [])
        if item.get("head_sha") == sha and item.get("head_branch") == branch
        and item.get("event") == "workflow_dispatch" and item.get("conclusion") == "success"
    ]


def tested_archives(run: Gh, repository: str, run_id: int) -> str | None:
    jobs = json.loads(run(
        "api", f"repos/{repository}/actions/runs/{run_id}/jobs?filter=latest&per_page=100"
    ))
    if not any(job.get("name") == JOB and job.get("conclusion") == "success"
               for job in jobs.get("jobs", [])):
        return None
    with tempfile.TemporaryDirectory() as directory:
        try:
            run("run", "download", str(run_id), "--repo", repository,
                "--name", ARTIFACT, "--dir", directory)
        except subprocess.CalledProcessError:
            return None
        path = Path(directory) / f"{ARTIFACT}.txt"
        return path.read_text() if path.is_file() else None


def matching_run(run: Gh, repository: str, sha: str, branch: str, checksums: str) -> int | None:
    for run_id in candidate_runs(run, repository, sha, branch):
        if tested_archives(run, repository, run_id) == checksums:
            return run_id
    return None


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    checksums = Path(sys.argv[1]).read_text()
    repository = os.environ["GITHUB_REPOSITORY"]
    sha = os.environ["GITHUB_SHA"]
    _, branch = policy_line(json.loads(Path(".github/release-required-workflows.json").read_text()))
    run_id = matching_run(gh, repository, sha, branch, checksums)
    if run_id is None:
        print(
            f"No successful exact-SHA ci.yml run on {branch} tested these archives "
            f"('{JOB}' with a matching '{ARTIFACT}' artifact). Dispatch ci.yml with "
            "platform=all and shard=all at this SHA, then rerun the release.",
            file=sys.stderr,
        )
        return 1
    print(f"Archives match the consumer test in admitted ci.yml run {run_id}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
