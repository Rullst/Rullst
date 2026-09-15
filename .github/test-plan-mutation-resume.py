#!/usr/bin/env python3
"""Regression tests for exact-run mutation campaign recovery."""

from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("plan-mutation-resume.py")
SPEC = importlib.util.spec_from_file_location("plan_mutation_resume", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot import {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def valid_run() -> dict[str, object]:
    return {
        "repository": {"full_name": "Rullst/Rullst"},
        "id": 1234,
        "path": ".github/workflows/mutants.yml",
        "workflow_id": 5678,
        "event": "workflow_dispatch",
        "status": "completed",
        "conclusion": "failure",
        "head_branch": "ci/mutation-evidence",
        "head_sha": "1" * 40,
        "run_attempt": 1,
    }


def valid_policy() -> dict[str, object]:
    return {
        "schema": 1,
        "repository": "Rullst/Rullst",
        "workflow_path": ".github/workflows/mutants.yml",
        "workflow_id": 5678,
        "run_id": 1234,
        "run_attempt": 1,
        "head_branch": "ci/mutation-evidence",
        "source_sha": "1" * 40,
        "conclusion": "failure",
        "mode": "full",
        "shard_count": 80,
        "resumable_shards": [2, 10, 65],
        "inventory_count": 14391,
        "inventory_sha256": "2" * 64,
        "cargo_mutants_version": "27.1.0",
        "finalization": {
            "repository": "Rullst/Rullst",
            "workflow_path": ".github/workflows/mutants.yml",
            "workflow_id": 5678,
            "run_id": 4321,
            "run_attempt": 2,
            "head_branch": "main",
            "head_sha": "3" * 40,
            "conclusion": "failure",
            "measured_source_sha": "1" * 40,
            "completed_fragment_count": 5,
            "incomplete_fragment": {
                "index": 130,
                "count": 160,
                "artifact": "resume-65-0",
            },
            "split_factor": 2,
            "test_timeout_seconds": 120,
            "build_timeout_seconds": 900,
        },
    }


def valid_recovery_run() -> dict[str, object]:
    return {
        "repository": {"full_name": "Rullst/Rullst"},
        "id": 4321,
        "path": ".github/workflows/mutants.yml",
        "workflow_id": 5678,
        "event": "workflow_dispatch",
        "status": "completed",
        "conclusion": "failure",
        "head_branch": "main",
        "head_sha": "3" * 40,
        "run_attempt": 2,
    }


class MutationResumePlanTests(unittest.TestCase):
    def test_valid_plan_is_sorted_and_keeps_total_shard_count(self) -> None:
        plan = MODULE.build_plan(valid_run(), valid_policy(), "65,2,10")
        self.assertEqual(plan["source_sha"], "1" * 40)
        self.assertEqual(plan["resume_shards"], "2,10,65")
        self.assertEqual(plan["artifact_count"], 83)
        self.assertEqual(
            plan["matrix"]["include"],
            [
                {"index": 4, "count": 160, "artifact": "resume-2-0"},
                {"index": 5, "count": 160, "artifact": "resume-2-1"},
                {"index": 20, "count": 160, "artifact": "resume-10-0"},
                {"index": 21, "count": 160, "artifact": "resume-10-1"},
                {"index": 130, "count": 160, "artifact": "resume-65-0"},
                {"index": 131, "count": 160, "artifact": "resume-65-1"},
            ],
        )
        self.assertEqual(plan["provenance"]["run_id"], 1234)
        self.assertEqual(plan["provenance"]["inventory_sha256"], "2" * 64)

    def test_invalid_split_factor_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "at least two"):
            MODULE.build_plan(valid_run(), valid_policy(), "2,10,65", 1)

    def test_duplicate_shards_are_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "must not be repeated"):
            MODULE.parse_shards("2,2", 80)

    def test_out_of_range_shards_are_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "between 0 and 79"):
            MODULE.parse_shards("80", 80)

    def test_noncanonical_shard_list_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "comma-separated"):
            MODULE.parse_shards("2, 4", 80)

    def test_wrong_repository_is_rejected(self) -> None:
        run = valid_run()
        run["repository"] = {"full_name": "attacker/fork"}
        with self.assertRaisesRegex(ValueError, "different repository"):
            MODULE.validate_run(run, valid_policy())

    def test_wrong_workflow_is_rejected(self) -> None:
        run = valid_run()
        run["path"] = ".github/workflows/ci.yml"
        with self.assertRaisesRegex(ValueError, "path does not match"):
            MODULE.validate_run(run, valid_policy())

    def test_invalid_source_sha_is_rejected(self) -> None:
        policy = valid_policy()
        policy["source_sha"] = "main"
        with self.assertRaisesRegex(ValueError, "valid source SHA"):
            MODULE.validate_run(valid_run(), policy)

    def test_arbitrary_branch_is_rejected(self) -> None:
        run = valid_run()
        run["head_branch"] = "attacker/branch"
        with self.assertRaisesRegex(ValueError, "head_branch does not match"):
            MODULE.validate_run(run, valid_policy())

    def test_nonfailed_campaign_is_rejected(self) -> None:
        run = valid_run()
        run["conclusion"] = "success"
        with self.assertRaisesRegex(ValueError, "conclusion does not match"):
            MODULE.validate_run(run, valid_policy())

    def test_unreviewed_shard_set_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "reviewed recovery policy"):
            MODULE.build_plan(valid_run(), valid_policy(), "2,10")

    def test_invalid_inventory_digest_is_rejected(self) -> None:
        policy = valid_policy()
        policy["inventory_sha256"] = "not-a-digest"
        with self.assertRaisesRegex(ValueError, "valid inventory digest"):
            MODULE.validate_policy(policy)

    def test_repository_policy_is_valid_and_content_bound(self) -> None:
        policy_path = SCRIPT.with_name("mutation-recovery-policy.json")
        policy = json.loads(policy_path.read_text(encoding="utf-8"))
        MODULE.validate_policy(policy)
        self.assertEqual(policy["run_id"], 34688592153)
        self.assertEqual(policy["inventory_count"], 14391)
        self.assertEqual(
            policy["inventory_sha256"],
            "986d5cc1de71f7c8afbae7823a8fca53304b84f8e81b8f9e2d1fd3949c15ef80",
        )

    def test_finalization_bisects_only_the_remaining_fragment(self) -> None:
        plan = MODULE.build_finalization_plan(
            valid_run(), valid_recovery_run(), valid_policy()
        )
        self.assertEqual(plan["artifact_count"], 84)
        self.assertEqual(plan["secondary_run_id"], 4321)
        self.assertEqual(plan["incomplete_fragment_artifact"], "resume-65-0")
        self.assertEqual(plan["test_timeout_seconds"], 120)
        self.assertEqual(
            plan["matrix"]["include"],
            [
                {"index": 260, "count": 320, "artifact": "resume-65-0-0"},
                {"index": 261, "count": 320, "artifact": "resume-65-0-1"},
            ],
        )
        self.assertEqual(plan["provenance"]["recovery_run"]["run_id"], 4321)

    def test_finalization_rejects_an_unreviewed_run(self) -> None:
        run = valid_recovery_run()
        run["head_sha"] = "4" * 40
        with self.assertRaisesRegex(ValueError, "head_sha does not match"):
            MODULE.build_finalization_plan(valid_run(), run, valid_policy())

    def test_finalization_rejects_unsafe_timeout(self) -> None:
        policy = valid_policy()
        finalization = policy["finalization"]
        assert isinstance(finalization, dict)
        finalization["test_timeout_seconds"] = 10
        with self.assertRaisesRegex(ValueError, "safe policy range"):
            MODULE.build_finalization_plan(
                valid_run(), valid_recovery_run(), policy
            )


if __name__ == "__main__":
    unittest.main()
