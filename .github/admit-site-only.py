#!/usr/bin/env python3
"""Validate a prior full Linux CI receipt for a development-only site change.

The caller must obtain run/jobs JSON from the authenticated GitHub API. This
does not authorize a release, validate the changed site or execute project code.
"""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import importlib.util
import json
from pathlib import Path
import subprocess
import sys


def sibling(name: str):
    path = Path(__file__).with_name(name + ".py")
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), path)
    if spec is None or spec.loader is None:
        raise ValueError("cannot load trusted policy helper")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


PLAN = sibling("plan-verification")
TIMING = sibling("report-ci-timings")
SHARDS = (
    "workspace", "cli-standard", "cli-profiles-basic", "cli-profiles-relational",
    "cli-profiles-polyglot", "cli-lms", "cli-saas-foundation", "cli-saas-product",
)
RUNTIME_JOBS = frozenset({
    "Code Quality & Format", "Check MSRV (Rust 1.96.0)",
    "Facade shared-local recovery composition", "Generated release access boundaries",
    "Isolated Labs acceptance (Linux)",
    "Redis live contracts", "Versioned deterministic AI evals",
    *{f"ORM {feature}" for feature in ("strict-mysql", "strict-postgres", "strict-sqlite")},
    *{f"Run Tests (ubuntu-latest / {shard})" for shard in SHARDS},
    *{f"Public feature boundaries ({i}/4)" for i in range(4)},
    *{f"Threat-model release-negative minimum ({i}/4)" for i in range(4)},
})
OPTIONAL_JOBS = {
    "Evidence quality scorecard (observational)": "skipped",
    "Verification scope": "success",
    "Verify site-only presentation": "skipped",
}


def validate_receipt(run: dict, jobs_payload: object, baseline: str,
                     branch: str, now: datetime) -> None:
    expected = {
        "head_sha": baseline, "head_branch": branch, "event": "push",
        "status": "completed", "conclusion": "success", "path": ".github/workflows/ci.yml",
        "name": "Rust CI",
    }
    if branch not in {"main", "v13"} or any(run.get(k) != v for k, v in expected.items()):
        raise ValueError("baseline is not a successful push CI on the expected source line")
    for field in ("repository", "head_repository"):
        if run.get(field, {}).get("full_name") != "Rullst/Rullst":
            raise ValueError("baseline repository mismatch")
    completed = TIMING.timestamp(run.get("updated_at"))
    if completed is None or not 0 <= (now - completed).total_seconds() <= 72 * 3600:
        raise ValueError("baseline is future-dated, stale or missing its time")
    identifier = TIMING.positive_integer(run.get("id"), "run id")
    attempt = TIMING.positive_integer(run.get("run_attempt"), "run attempt")
    jobs = TIMING.load_jobs(jobs_payload)
    names, identifiers = set(), set()
    for job in jobs:
        name = job.get("name")
        job_id = TIMING.positive_integer(job.get("id"), "job id")
        if (not isinstance(name, str) or name in names or job_id in identifiers
                or type(job.get("run_id")) is not int or type(job.get("run_attempt")) is not int
                or job.get("run_id") != identifier or job.get("run_attempt") != attempt
                or job.get("head_sha") != baseline or job.get("workflow_name") != "Rust CI"
                or job.get("status") != "completed"):
            raise ValueError("incomplete, repeated or mismatched job provenance")
        names.add(name)
        identifiers.add(job_id)
        expected_conclusion = "success" if name in RUNTIME_JOBS else OPTIONAL_JOBS.get(name)
        if expected_conclusion is None or job.get("conclusion") != expected_conclusion:
            raise ValueError("missing successful runtime job or unreviewed inventory")
    if not RUNTIME_JOBS.issubset(names):
        raise ValueError("baseline does not contain the full Linux runtime inventory")


def evaluate(root: Path, base: str, head: str, branch: str,
             run: dict, jobs: object, now: datetime) -> dict:
    report = {
        "schema": "rullst.site-ci-admission.v1", "runtime_required": True,
        "release_evidence_eligible": False, "site_validation_required": True,
        "base_sha": None, "head_sha": None, "baseline_run_id": None,
        "reason": "no-valid-baseline",
    }
    try:
        plan = PLAN.observe(root, base, head)
        report.update(base_sha=plan["base_sha"], head_sha=plan["head_sha"])
        if plan["candidate_scope"] != "site-presentation" or plan["fallback_reasons"]:
            raise ValueError("not an exclusively reviewed site change")
        git = PLAN.Git(root)
        for revision in (plan["base_sha"], plan["head_sha"]):
            modes = git.tree(revision)
            if any(modes.get(item["path"]) not in {None, "100644"} for item in plan["files"]):
                raise ValueError("presentation files must be regular non-executable blobs")
        validate_receipt(run, jobs, plan["base_sha"], branch, now)
        report.update(runtime_required=False, baseline_run_id=run["id"],
                      reason="unchanged-runtime-inputs-with-full-linux-baseline")
    except (ValueError, TypeError, KeyError, AttributeError, OSError, RecursionError,
            subprocess.TimeoutExpired):
        # A malformed receipt is an instruction to run normally, not to skip.
        pass
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--branch", required=True)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    args = parser.parse_args()
    payload = {}
    try:
        raw = sys.stdin.buffer.read(PLAN.MAX_BYTES + 1)
        if len(raw) > PLAN.MAX_BYTES:
            raise ValueError("oversized receipt")
        payload = json.loads(raw)
        if not isinstance(payload, dict):
            raise ValueError("expected receipt object")
    except (ValueError, RecursionError):
        payload = {}
    report = evaluate(args.repo, args.base, args.head, args.branch,
                      payload.get("run", {}), payload.get("jobs", {}), datetime.now(timezone.utc))
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
