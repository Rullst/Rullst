#!/usr/bin/env python3
"""Regression tests for exact-SHA release admission."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("check-release-admission.py")
SPEC = importlib.util.spec_from_file_location("check_release_admission", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot import {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


SHA = "0123456789abcdef0123456789abcdef01234567"


class ReleaseAdmissionTests(unittest.TestCase):
    def test_accepts_only_completed_success_for_exact_sha_and_event(self) -> None:
        payload = {
            "workflow_runs": [
                {
                    "id": 123,
                    "head_sha": SHA,
                    "head_branch": "main",
                    "event": "push",
                    "status": "completed",
                    "conclusion": "success",
                }
            ]
        }
        self.assertEqual(
            MODULE.exact_successful_run_ids(payload, SHA, "main", "push"), [123]
        )

    def test_rejects_wrong_sha_branch_event_status_or_conclusion(self) -> None:
        baseline = {
            "id": 123,
            "head_sha": SHA,
            "head_branch": "main",
            "event": "push",
            "status": "completed",
            "conclusion": "success",
        }
        for key, value in (
            ("head_sha", "f" * 40),
            ("head_branch", "feature"),
            ("event", "workflow_dispatch"),
            ("status", "in_progress"),
            ("conclusion", "failure"),
        ):
            run = dict(baseline)
            run[key] = value
            with self.subTest(key=key):
                self.assertEqual(
                    MODULE.exact_successful_run_ids(
                        {"workflow_runs": [run]}, SHA, "main", "push"
                    ),
                    [],
                )

    def test_required_jobs_reject_missing_skipped_and_duplicate_targets(self) -> None:
        required = ("Linux", "macOS", "Windows")
        complete = {
            "total_count": 3,
            "jobs": [
                {"name": "Linux", "conclusion": "success"},
                {"name": "macOS", "conclusion": "success"},
                {"name": "Windows", "conclusion": "success"},
            ],
        }
        self.assertTrue(MODULE.required_jobs_succeeded(complete, required))

        for jobs in (
            complete["jobs"][:-1],
            [*complete["jobs"][:-1], {"name": "Windows", "conclusion": "skipped"}],
            [*complete["jobs"], {"name": "Windows", "conclusion": "success"}],
        ):
            with self.subTest(jobs=jobs):
                self.assertFalse(
                    MODULE.required_jobs_succeeded(
                        {"total_count": len(jobs), "jobs": jobs}, required
                    )
                )

    def test_policy_requires_full_manual_ci_and_unique_safe_workflows(self) -> None:
        valid_ci = {
            "workflow": "ci.yml",
            "event": "workflow_dispatch",
            "required_jobs": ["Linux", "macOS", "Windows"],
        }
        full_fuzz = {
            "workflow": "fuzzing.yml",
            "event": "workflow_dispatch",
            "required_jobs": [
                "Fuzz campaign evidence boundary",
                *(f"Fuzz target-{index}" for index in range(40)),
            ],
        }
        manual_gates = [
            valid_ci,
            {"workflow": "dast-zap.yml", "event": "workflow_dispatch"},
            full_fuzz,
            {"workflow": "kani.yml", "event": "workflow_dispatch"},
            {"workflow": "miri.yml", "event": "workflow_dispatch"},
            {"workflow": "omni-android.yml", "event": "workflow_dispatch"},
            {"workflow": "omni-desktop.yml", "event": "workflow_dispatch"},
            {"workflow": "omni-ios.yml", "event": "workflow_dispatch"},
            {"workflow": "proptest.yml", "event": "workflow_dispatch"},
            {"workflow": "sanitizers.yml", "event": "workflow_dispatch"},
        ]
        with self.assertRaises(SystemExit):
            MODULE.validate_policy(
                {
                    "schema_version": 2,
                    "required_branch": "main",
                    "workflows": [valid_ci, valid_ci],
                }
            )
        with self.assertRaises(SystemExit):
            MODULE.validate_policy(
                {
                    "schema_version": 2,
                    "required_branch": "main",
                    "workflows": [
                        valid_ci,
                        {"workflow": "../audit.yml", "event": "push"},
                    ],
                }
            )
        with self.assertRaisesRegex(SystemExit, "full-platform"):
            MODULE.validate_policy(
                {
                    "schema_version": 2,
                    "required_branch": "main",
                    "workflows": [
                        {"workflow": "ci.yml", "event": "push", "required_jobs": []}
                    ],
                }
            )
        with self.assertRaisesRegex(SystemExit, "all 40 target jobs"):
            MODULE.validate_policy(
                {
                    "schema_version": 2,
                    "required_branch": "main",
                    "workflows": [
                        item if item is not full_fuzz else {
                            **full_fuzz,
                            "required_jobs": full_fuzz["required_jobs"][:-1],
                        }
                        for item in manual_gates
                    ],
                }
            )
        with self.assertRaisesRegex(SystemExit, "mandatory manual"):
            MODULE.validate_policy(
                {
                    "schema_version": 2,
                    "required_branch": "main",
                    "workflows": manual_gates[:-1],
                }
            )

        branch, requirements = MODULE.validate_policy(
            {
                "schema_version": 2,
                "required_branch": "main",
                "workflows": [
                    *manual_gates,
                    {"workflow": "audit.yml", "event": "push"},
                ],
            }
        )
        self.assertEqual(branch, "main")
        self.assertEqual(
            [item.workflow for item in requirements],
            sorted(
                ["audit.yml", *(item["workflow"] for item in manual_gates)]
            ),
        )


if __name__ == "__main__":
    unittest.main()
