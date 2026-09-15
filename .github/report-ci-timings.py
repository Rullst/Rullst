#!/usr/bin/env python3
"""Read paginated GitHub job JSON from stdin; report timings without running CI."""

from __future__ import annotations

import argparse
import html
import json
import re
import sys
from datetime import datetime


MAX_INPUT_BYTES = 16 * 1024 * 1024
MAX_JOBS = 1000
SHA = re.compile(r"[0-9a-fA-F]{40}")


def positive_integer(value: object, field: str) -> int:
    if type(value) is not int or value <= 0:
        raise ValueError(f"{field} must be a positive integer")
    return value


def timestamp(value: object) -> datetime | None:
    if value is None:
        return None
    if not isinstance(value, str):
        raise ValueError("timestamps must be ISO-8601 strings or null")
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        raise ValueError("timestamps must include a timezone")
    return parsed


def duration(start: object, end: object) -> float | None:
    first, last = timestamp(start), timestamp(end)
    if first is None or last is None:
        return None
    seconds = (last - first).total_seconds()
    if seconds < 0:
        raise ValueError("negative time interval in job data")
    return seconds


def text(value: object, field: str) -> str:
    if not isinstance(value, str) or not value.strip() or len(value) > 2000:
        raise ValueError(f"{field} must be a nonempty, bounded string")
    return value


def load_jobs(payload: object) -> list[dict]:
    pages = payload if isinstance(payload, list) else [payload]
    if not pages or len(pages) > MAX_JOBS:
        raise ValueError("expected one job response or a nonempty array of pages")
    jobs: list[dict] = []
    total: int | None = None
    for page in pages:
        if not isinstance(page, dict) or not isinstance(page.get("jobs"), list):
            raise ValueError("each page must contain a jobs array")
        count = positive_integer(page.get("total_count"), "total_count")
        if count > MAX_JOBS or (total is not None and count != total):
            raise ValueError("job totals exceed the limit or changed between pages")
        total = count
        jobs.extend(page["jobs"])
        if len(jobs) > count:
            raise ValueError("more jobs supplied than total_count")
    if len(jobs) != total:
        raise ValueError("incomplete job inventory; use gh api --paginate --slurp")
    if any(not isinstance(job, dict) for job in jobs):
        raise ValueError("job entries must be objects")
    return jobs


def summarize(payload: object) -> dict:
    jobs = load_jobs(payload)
    identities: set[tuple[int, int, str, str]] = set()
    ids: set[int] = set()
    rows: list[dict] = []
    steps: list[dict] = []
    for job in jobs:
        job_id = positive_integer(job.get("id"), "job id")
        if job_id in ids:
            raise ValueError("duplicate job id")
        ids.add(job_id)
        sha = text(job.get("head_sha"), "head_sha")
        if SHA.fullmatch(sha) is None:
            raise ValueError("head_sha must be a full commit SHA")
        identities.add((
            positive_integer(job.get("run_id"), "run_id"),
            positive_integer(job.get("run_attempt"), "run_attempt"),
            sha.lower(),
            text(job.get("workflow_name"), "workflow_name"),
        ))
        name = text(job.get("name"), "job name")
        status = text(job.get("status"), "job status")
        completed = status == "completed"
        started = status in {"in_progress", "completed"}
        conclusion = job.get("conclusion")
        if conclusion is not None:
            conclusion = text(conclusion, "job conclusion")
        if completed and conclusion is None:
            raise ValueError("completed job is missing its conclusion")
        runtime = duration(job.get("started_at"), job.get("completed_at")) if completed else None
        wait = duration(job.get("created_at"), job.get("started_at")) if started else None
        rows.append({
            "id": job_id, "name": name, "status": status,
            "conclusion": conclusion, "wait_seconds": wait, "run_seconds": runtime,
        })
        job_steps = job.get("steps", [])
        if not isinstance(job_steps, list) or len(job_steps) > 1000:
            raise ValueError("job steps must be a bounded array")
        for step in job_steps:
            if not isinstance(step, dict):
                raise ValueError("step entries must be objects")
            if step.get("status") != "completed" or step.get("conclusion") == "skipped":
                continue
            elapsed = duration(step.get("started_at"), step.get("completed_at"))
            if elapsed is not None:
                steps.append({
                    "job_id": job_id, "job": name,
                    "name": text(step.get("name"), "step name"),
                    "conclusion": text(step.get("conclusion"), "step conclusion"),
                    "seconds": elapsed,
                })
    if len(identities) != 1:
        raise ValueError("do not mix workflows, runs, attempts or source SHAs")
    run_id, attempt, sha, workflow = identities.pop()
    completed_count = sum(row["status"] == "completed" for row in rows)
    known_runtimes = [row["run_seconds"] for row in rows if row["run_seconds"] is not None]
    return {
        "schema_version": 1, "run_id": run_id, "run_attempt": attempt,
        "head_sha": sha, "workflow": workflow, "job_count": len(rows),
        "completed_jobs": completed_count,
        "partial": completed_count != len(rows) or len(known_runtimes) != len(rows),
        "measured_runtime_jobs": len(known_runtimes),
        "measured_runner_seconds": sum(known_runtimes),
        "longest_completed_job_seconds": max(known_runtimes, default=None),
        "jobs": sorted(rows, key=lambda row: (-(row["run_seconds"] or 0), row["id"])),
        "steps": sorted(steps, key=lambda row: (-row["seconds"], row["job_id"], row["name"])),
    }


def cell(value: object) -> str:
    escaped = html.escape(str(value), quote=True)
    escaped = " ".join(escaped.split())
    for character in ("\\", "|", "`", "*", "_", "[", "]"):
        escaped = escaped.replace(character, "\\" + character)
    return escaped


def minutes(seconds: float | None) -> str:
    return "unknown" if seconds is None else f"{seconds / 60:.2f}"


def markdown(report: dict, top: int) -> str:
    lines = [
        f"# CI timing observation: {cell(report['workflow'])}", "",
        f"Run {report['run_id']}, attempt {report['run_attempt']}, source `{report['head_sha']}`.",
        f"Completed jobs: {report['completed_jobs']}/{report['job_count']}. "
        f"Snapshot: {'partial' if report['partial'] else 'complete'}.", "",
        "Times are minutes. Wait means job creation to start, including possible orchestration "
        "or dependency waits; it is not proven runner-queue time. Unknown is not zero.",
        "Execution includes setup, compilation, tests and cleanup. A combined Cargo step "
        "cannot separate compile time from test time. This report is not pass/fail or release evidence.", "",
        f"Measured runner-minutes: {minutes(report['measured_runner_seconds'])} "
        f"across {report['measured_runtime_jobs']} jobs. This sums parallel jobs; "
        "it is neither elapsed workflow time nor a billing estimate.", "",
        "## Jobs (longest completed execution first)", "",
        "| Job | State | Wait | Execution |", "| :--- | :--- | ---: | ---: |",
    ]
    for row in report["jobs"][:top]:
        lines.append(
            f"| {cell(row['name'])} | {cell(row['conclusion'] or row['status'])} | "
            f"{minutes(row['wait_seconds'])} | {minutes(row['run_seconds'])} |"
        )
    waits = sorted(
        (row for row in report["jobs"] if row["wait_seconds"] is not None),
        key=lambda row: (-row["wait_seconds"], row["id"]),
    )
    lines += ["", "## Longest measured waits", "", "| Job | Wait |", "| :--- | ---: |"]
    for row in waits[:top]:
        lines.append(f"| {cell(row['name'])} | {minutes(row['wait_seconds'])} |")
    lines += ["", "## Longest completed steps", "",
              "| Job / step | State | Execution |", "| :--- | :--- | ---: |"]
    for step in report["steps"][:top]:
        lines.append(
            f"| {cell(step['job'])} / {cell(step['name'])} | {cell(step['conclusion'])} | "
            f"{minutes(step['seconds'])} |"
        )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--format", choices=("markdown", "json"), default="markdown")
    parser.add_argument("--top", type=int, default=10, help="rows per Markdown table (1-100)")
    args = parser.parse_args()
    if not 1 <= args.top <= 100:
        parser.error("--top must be between 1 and 100")
    try:
        raw = sys.stdin.buffer.read(MAX_INPUT_BYTES + 1)
        if len(raw) > MAX_INPUT_BYTES:
            raise ValueError("input exceeds the 16 MiB limit")
        report = summarize(json.loads(raw))
    except (ValueError, UnicodeError, RecursionError) as error:
        print(f"CI timing input rejected: {error}", file=sys.stderr)
        return 1
    if args.format == "json":
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(markdown(report, args.top), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
