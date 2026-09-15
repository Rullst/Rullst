#!/usr/bin/env python3
"""Validate an interrupted mutation run and build its recovery matrix."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


SHA_RE = re.compile(r"^[0-9a-f]{40}$")
SHARDS_RE = re.compile(r"^[0-9]+(?:,[0-9]+)*$")


def parse_shards(raw: str, shard_count: int) -> list[int]:
    if not SHARDS_RE.fullmatch(raw):
        raise ValueError("resume shards must be comma-separated decimal indexes")

    shards = [int(value) for value in raw.split(",")]
    if len(shards) != len(set(shards)):
        raise ValueError("resume shard indexes must not be repeated")
    if any(index < 0 or index >= shard_count for index in shards):
        raise ValueError(f"resume shard indexes must be between 0 and {shard_count - 1}")
    return sorted(shards)


def validate_policy(policy: dict[str, Any]) -> None:
    if policy.get("schema") != 1:
        raise ValueError("unsupported mutation recovery policy schema")
    if policy.get("mode") != "full":
        raise ValueError("recovery policy must describe a full campaign")

    positive_integers = (
        "workflow_id",
        "run_id",
        "run_attempt",
        "shard_count",
        "inventory_count",
    )
    if any(
        not isinstance(policy.get(field), int) or policy[field] <= 0
        for field in positive_integers
    ):
        raise ValueError("recovery policy numeric identities must be positive integers")

    source_sha = policy.get("source_sha")
    inventory_sha256 = policy.get("inventory_sha256")
    if not isinstance(source_sha, str) or not SHA_RE.fullmatch(source_sha):
        raise ValueError("recovery policy does not contain a valid source SHA")
    if not isinstance(inventory_sha256, str) or not re.fullmatch(
        r"[0-9a-f]{64}", inventory_sha256
    ):
        raise ValueError("recovery policy does not contain a valid inventory digest")

    resumable_shards = policy.get("resumable_shards")
    if (
        not isinstance(resumable_shards, list)
        or not resumable_shards
        or any(not isinstance(index, int) for index in resumable_shards)
        or resumable_shards != sorted(set(resumable_shards))
        or any(index < 0 or index >= policy["shard_count"] for index in resumable_shards)
    ):
        raise ValueError("recovery policy resumable shards are invalid")

    required_strings = (
        "repository",
        "workflow_path",
        "head_branch",
        "conclusion",
        "cargo_mutants_version",
    )
    if any(
        not isinstance(policy.get(field), str) or not policy[field]
        for field in required_strings
    ):
        raise ValueError("recovery policy string identities must not be empty")


def validate_run(run: dict[str, Any], policy: dict[str, Any]) -> dict[str, Any]:
    validate_policy(policy)
    observed_repository = run.get("repository", {}).get("full_name")
    expected_fields = {
        "id": policy["run_id"],
        "path": policy["workflow_path"],
        "workflow_id": policy["workflow_id"],
        "run_attempt": policy["run_attempt"],
        "head_branch": policy["head_branch"],
        "head_sha": policy["source_sha"],
        "conclusion": policy["conclusion"],
        "event": "workflow_dispatch",
        "status": "completed",
    }
    if observed_repository != policy["repository"]:
        raise ValueError("resume run belongs to a different repository")
    for field, expected in expected_fields.items():
        if run.get(field) != expected:
            raise ValueError(
                f"resume run {field} does not match the reviewed recovery policy"
            )
    return {
        "repository": policy["repository"],
        "workflow_path": policy["workflow_path"],
        "workflow_id": policy["workflow_id"],
        "run_id": policy["run_id"],
        "run_attempt": policy["run_attempt"],
        "head_branch": policy["head_branch"],
        "source_sha": policy["source_sha"],
        "conclusion": policy["conclusion"],
        "mode": policy["mode"],
        "inventory_count": policy["inventory_count"],
        "inventory_sha256": policy["inventory_sha256"],
        "cargo_mutants_version": policy["cargo_mutants_version"],
    }


def validate_finalization_run(
    run: dict[str, Any], policy: dict[str, Any]
) -> dict[str, Any]:
    validate_policy(policy)
    finalization = policy.get("finalization")
    if not isinstance(finalization, dict):
        raise ValueError("recovery policy does not authorize finalization")

    observed_repository = run.get("repository", {}).get("full_name")
    if observed_repository != finalization.get("repository"):
        raise ValueError("finalization run belongs to a different repository")
    expected_fields = {
        "id": finalization.get("run_id"),
        "path": finalization.get("workflow_path"),
        "workflow_id": finalization.get("workflow_id"),
        "run_attempt": finalization.get("run_attempt"),
        "head_branch": finalization.get("head_branch"),
        "head_sha": finalization.get("head_sha"),
        "conclusion": finalization.get("conclusion"),
        "event": "workflow_dispatch",
        "status": "completed",
    }
    for field, expected in expected_fields.items():
        if run.get(field) != expected:
            raise ValueError(
                f"finalization run {field} does not match the reviewed recovery policy"
            )

    if finalization.get("measured_source_sha") != policy["source_sha"]:
        raise ValueError("finalization source does not match the original campaign")
    fragment = finalization.get("incomplete_fragment")
    if not isinstance(fragment, dict):
        raise ValueError("finalization policy does not identify an incomplete fragment")
    fragment_index = fragment.get("index")
    fragment_count = fragment.get("count")
    artifact = fragment.get("artifact")
    if (
        not isinstance(fragment_index, int)
        or not isinstance(fragment_count, int)
        or fragment_index < 0
        or fragment_count <= 0
        or fragment_index >= fragment_count
        or not isinstance(artifact, str)
        or not re.fullmatch(r"resume-[0-9]+-[0-9]+", artifact)
    ):
        raise ValueError("finalization policy contains an invalid fragment")

    positive_fields = (
        "completed_fragment_count",
        "split_factor",
        "test_timeout_seconds",
        "build_timeout_seconds",
    )
    if any(
        not isinstance(finalization.get(field), int) or finalization[field] <= 0
        for field in positive_fields
    ):
        raise ValueError("finalization limits must be positive integers")
    if finalization["split_factor"] < 2:
        raise ValueError("finalization split factor must be at least two")
    if not 30 <= finalization["test_timeout_seconds"] <= 600:
        raise ValueError("finalization test timeout is outside the safe policy range")
    if not 60 <= finalization["build_timeout_seconds"] <= 1800:
        raise ValueError("finalization build timeout is outside the safe policy range")
    return finalization


def build_plan(
    run: dict[str, Any],
    policy: dict[str, Any],
    raw_shards: str,
    split_factor: int = 2,
) -> dict[str, Any]:
    if split_factor < 2:
        raise ValueError("resume split factor must be at least two")
    provenance = validate_run(run, policy)
    shard_count = policy["shard_count"]
    shards = parse_shards(raw_shards, shard_count)
    if shards != policy["resumable_shards"]:
        raise ValueError("resume shards do not match the reviewed recovery policy")
    resumed_count = shard_count * split_factor
    fragments = [
        {
            "index": original * split_factor + offset,
            "count": resumed_count,
            "artifact": f"resume-{original}-{offset}",
        }
        for original in shards
        for offset in range(split_factor)
    ]
    return {
        "source_sha": provenance["source_sha"],
        "resume_shards": ",".join(str(index) for index in shards),
        "artifact_count": shard_count + len(shards) * (split_factor - 1),
        "matrix": {"include": fragments},
        "provenance": provenance,
    }


def build_finalization_plan(
    original_run: dict[str, Any],
    recovery_run: dict[str, Any],
    policy: dict[str, Any],
) -> dict[str, Any]:
    provenance = validate_run(original_run, policy)
    finalization = validate_finalization_run(recovery_run, policy)
    fragment = finalization["incomplete_fragment"]
    split_factor = finalization["split_factor"]
    final_count = fragment["count"] * split_factor
    fragments = [
        {
            "index": fragment["index"] * split_factor + offset,
            "count": final_count,
            "artifact": f"{fragment['artifact']}-{offset}",
        }
        for offset in range(split_factor)
    ]
    provenance["recovery_run"] = {
        "run_id": finalization["run_id"],
        "run_attempt": finalization["run_attempt"],
        "head_branch": finalization["head_branch"],
        "head_sha": finalization["head_sha"],
        "measured_source_sha": finalization["measured_source_sha"],
    }
    completed_originals = policy["shard_count"] - len(policy["resumable_shards"])
    artifact_count = (
        completed_originals
        + finalization["completed_fragment_count"]
        + len(fragments)
    )
    return {
        "source_sha": provenance["source_sha"],
        "resume_shards": ",".join(
            str(index) for index in policy["resumable_shards"]
        ),
        "artifact_count": artifact_count,
        "matrix": {"include": fragments},
        "provenance": provenance,
        "secondary_run_id": finalization["run_id"],
        "incomplete_fragment_artifact": fragment["artifact"],
        "test_timeout_seconds": finalization["test_timeout_seconds"],
        "build_timeout_seconds": finalization["build_timeout_seconds"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-json", required=True, type=Path)
    parser.add_argument("--policy-json", required=True, type=Path)
    parser.add_argument("--shards")
    parser.add_argument("--recovery-run-json", type=Path)
    parser.add_argument("--split-factor", type=int, default=2)
    args = parser.parse_args()

    try:
        run = json.loads(args.run_json.read_text(encoding="utf-8"))
        policy = json.loads(args.policy_json.read_text(encoding="utf-8"))
        if args.recovery_run_json is not None:
            recovery_run = json.loads(
                args.recovery_run_json.read_text(encoding="utf-8")
            )
            if args.shards is not None:
                raise ValueError("finalization does not accept a shard input")
            plan = build_finalization_plan(run, recovery_run, policy)
        else:
            if args.shards is None:
                raise ValueError("resume mode requires --shards")
            plan = build_plan(
                run,
                policy,
                args.shards,
                args.split_factor,
            )
    except (OSError, json.JSONDecodeError, ValueError) as error:
        parser.error(str(error))

    print(json.dumps(plan, separators=(",", ":"), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
