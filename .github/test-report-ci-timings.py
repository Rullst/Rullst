#!/usr/bin/env python3
"""Deterministic fixtures for read-only CI timing observations."""

from __future__ import annotations

import copy
import importlib.util
import json
import subprocess
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("report-ci-timings.py")
SPEC = importlib.util.spec_from_file_location("report_ci_timings", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot import {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def job(identifier: int = 1, **changes: object) -> dict:
    result = {
        "id": identifier, "name": f"Test {identifier}", "run_id": 42,
        "run_attempt": 1, "head_sha": "a" * 40, "workflow_name": "Rust CI",
        "status": "completed", "conclusion": "success",
        "created_at": "2026-09-15T10:00:00Z",
        "started_at": "2026-09-15T10:05:00Z",
        "completed_at": "2026-09-15T10:15:00Z",
        "steps": [{
            "name": "Cargo build and test", "status": "completed",
            "conclusion": "success", "started_at": "2026-09-15T10:06:00Z",
            "completed_at": "2026-09-15T10:14:00Z",
        }],
    }
    result.update(changes)
    return result


def response(*jobs: dict) -> dict:
    return {"total_count": len(jobs), "jobs": list(jobs)}


class TimingTests(unittest.TestCase):
    def test_separates_wait_runtime_and_combined_step(self) -> None:
        report = MODULE.summarize(response(job()))
        self.assertFalse(report["partial"])
        self.assertEqual(report["jobs"][0]["wait_seconds"], 300)
        self.assertEqual(report["jobs"][0]["run_seconds"], 600)
        self.assertEqual(report["steps"][0]["seconds"], 480)
        self.assertEqual(report["head_sha"], "a" * 40)

    def test_accepts_complete_pagination_and_sums_parallel_runner_time(self) -> None:
        report = MODULE.summarize([
            {"total_count": 2, "jobs": [job(1)]},
            {"total_count": 2, "jobs": [job(2)]},
        ])
        self.assertEqual(report["measured_runner_seconds"], 1200)
        self.assertEqual(report["longest_completed_job_seconds"], 600)
        self.assertEqual(report["job_count"], 2)

    def test_missing_timestamps_are_unknown_not_zero(self) -> None:
        report = MODULE.summarize(response(job(created_at=None, completed_at=None)))
        self.assertTrue(report["partial"])
        self.assertIsNone(report["jobs"][0]["wait_seconds"])
        self.assertIsNone(report["jobs"][0]["run_seconds"])
        self.assertEqual(report["measured_runtime_jobs"], 0)
        self.assertIn("unknown", MODULE.markdown(report, 10))

    def test_active_and_queued_jobs_are_not_counted_as_finished(self) -> None:
        report = MODULE.summarize(response(
            job(1, status="in_progress", conclusion=None, completed_at=None),
            job(2, status="queued", conclusion=None, started_at=None, completed_at=None, steps=[]),
        ))
        self.assertTrue(report["partial"])
        self.assertEqual(report["completed_jobs"], 0)
        self.assertEqual(report["measured_runner_seconds"], 0)
        self.assertEqual(report["jobs"][0]["wait_seconds"], 300)
        self.assertIsNone(report["jobs"][1]["wait_seconds"])
        self.assertEqual(len(report["steps"]), 1)

    def test_failed_cancelled_and_skipped_jobs_remain_visible(self) -> None:
        report = MODULE.summarize(response(
            job(1, conclusion="failure"), job(2, conclusion="cancelled"),
            job(3, conclusion="skipped", steps=[]),
        ))
        rendered = MODULE.markdown(report, 10)
        for state in ("failure", "cancelled", "skipped"):
            self.assertIn(state, rendered)
        self.assertNotIn("all tests passed", rendered)

    def test_incomplete_or_skipped_steps_are_not_measured(self) -> None:
        fixture = job()
        active = copy.deepcopy(fixture["steps"][0])
        active.update(status="in_progress", conclusion=None, completed_at=None)
        skipped = copy.deepcopy(fixture["steps"][0])
        skipped["conclusion"] = "skipped"
        fixture["steps"].extend([active, skipped])
        report = MODULE.summarize(response(fixture))
        self.assertEqual(len(report["steps"]), 1)

    def test_zero_duration_is_measured(self) -> None:
        report = MODULE.summarize(response(job(completed_at="2026-09-15T10:05:00Z")))
        self.assertEqual(report["jobs"][0]["run_seconds"], 0)
        self.assertEqual(MODULE.minutes(0), "0.00")

    def test_timezone_offsets_are_normalized(self) -> None:
        report = MODULE.summarize(response(job(started_at="2026-09-15T07:05:00-03:00")))
        self.assertEqual(report["jobs"][0]["run_seconds"], 600)

    def test_rejects_mixed_provenance(self) -> None:
        for field, other in (("run_id", 43), ("run_attempt", 2),
                             ("head_sha", "b" * 40), ("workflow_name", "Other CI")):
            with self.subTest(field=field), self.assertRaises(ValueError):
                MODULE.summarize(response(job(1), job(2, **{field: other})))

    def test_rejects_incomplete_duplicate_and_changed_page_inventory(self) -> None:
        cases = [
            {"total_count": 2, "jobs": [job()]},
            response(job(), job()),
            [{"total_count": 2, "jobs": [job(1)]}, {"total_count": 3, "jobs": [job(2)]}],
            {"total_count": 1, "jobs": [job(1), job(2)]},
        ]
        for payload in cases:
            with self.subTest(payload=payload), self.assertRaises(ValueError):
                MODULE.summarize(payload)

    def test_rejects_malformed_inventory_and_identifiers(self) -> None:
        cases = [None, [], {}, response(), {"total_count": True, "jobs": [job()]},
                 {"total_count": 1001, "jobs": []}, response(None), response(job(id=True)),
                 response(job(head_sha="main")), response(job(run_attempt=0)),
                 response(job(steps="not an array")), response(job(conclusion=None))]
        for payload in cases:
            with self.subTest(payload=payload), self.assertRaises(ValueError):
                MODULE.summarize(payload)

    def test_rejects_negative_or_malformed_time_intervals(self) -> None:
        for end in ("2026-09-15T10:04:00Z", "2026-09-15T10:15:00", "not a date", 4):
            with self.subTest(end=end), self.assertRaises(ValueError):
                MODULE.summarize(response(job(completed_at=end)))

    def test_markdown_escapes_untrusted_names_and_limits_rows(self) -> None:
        report = MODULE.summarize(response(job(name='<img src=x> | [link](url)\n`code`'), job(2)))
        rendered = MODULE.markdown(report, 1)
        self.assertNotIn("<img", rendered)
        self.assertIn("&lt;img", rendered)
        self.assertIn("\\|", rendered)
        self.assertIn("\\[link\\]", rendered)
        self.assertNotIn("Test 2", rendered)
        self.assertIn("neither elapsed workflow time nor a billing estimate", rendered)

    def test_cli_json_and_rejected_input(self) -> None:
        for data, expected in ((json.dumps(response(job())), 0), ("{}", 1), ("not JSON", 1)):
            with self.subTest(data=data):
                result = subprocess.run(
                    [sys.executable, str(SCRIPT), "--format", "json"],
                    input=data, text=True, capture_output=True, check=False,
                )
                self.assertEqual(result.returncode, expected, result.stderr)
                if expected == 0:
                    self.assertEqual(json.loads(result.stdout)["run_id"], 42)
                else:
                    self.assertEqual(result.stdout, "")
                    self.assertIn("rejected", result.stderr)

    def test_cli_rejects_invalid_top(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--top", "0"],
            input="", text=True, capture_output=True, check=False,
        )
        self.assertEqual(result.returncode, 2)


if __name__ == "__main__":
    unittest.main()
