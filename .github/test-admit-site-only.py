#!/usr/bin/env python3
"""Ensure a site-only candidate cannot inherit missing or failed runtime tests."""

import copy
from datetime import datetime, timedelta, timezone
import importlib.util
import unittest
from pathlib import Path
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location("admit_site_only", Path(__file__).with_name("admit-site-only.py"))
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot load admission policy")
POLICY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(POLICY)
NOW = datetime(2026, 9, 15, 17, tzinfo=timezone.utc)
SHA = "a" * 40


def receipt():
    run = {
        "id": 42, "run_attempt": 1, "head_sha": SHA, "head_branch": "v13",
        "event": "push", "path": ".github/workflows/ci.yml", "name": "Rust CI",
        "status": "completed", "conclusion": "success", "updated_at": NOW.isoformat(),
        "repository": {"full_name": "Rullst/Rullst"},
        "head_repository": {"full_name": "Rullst/Rullst"},
    }
    jobs = [{
        "id": i + 1, "name": name, "run_id": 42, "run_attempt": 1, "head_sha": SHA,
        "workflow_name": "Rust CI", "status": "completed", "conclusion": "success",
    } for i, name in enumerate(sorted(POLICY.RUNTIME_JOBS))]
    return run, {"total_count": len(jobs), "jobs": jobs}


class ReceiptTests(unittest.TestCase):
    def test_accepts_the_exact_full_linux_inventory(self):
        run, jobs = receipt()
        self.assertEqual(len(jobs["jobs"]), 26)
        POLICY.validate_receipt(run, jobs, SHA, "v13", NOW)

    def test_rejects_failed_active_diagnostic_foreign_or_wrong_revision_runs(self):
        changes = {
            "head_sha": "b" * 40, "head_branch": "topic", "event": "workflow_dispatch",
            "path": ".github/workflows/other.yml", "status": "in_progress",
            "conclusion": "failure", "name": "Other CI", "run_attempt": False,
            "repository": {"full_name": "other/repo"},
            "head_repository": {"full_name": "fork/Rullst"},
        }
        for field, value in changes.items():
            run, jobs = receipt()
            run[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                POLICY.validate_receipt(run, jobs, SHA, "v13", NOW)

    def test_rejects_stale_future_and_undated_receipts(self):
        for value in (None, (NOW + timedelta(seconds=1)).isoformat(),
                      (NOW - timedelta(hours=73)).isoformat()):
            run, jobs = receipt()
            run["updated_at"] = value
            with self.subTest(value=value), self.assertRaises(ValueError):
                POLICY.validate_receipt(run, jobs, SHA, "v13", NOW)

    def test_every_runtime_job_is_mandatory(self):
        for name in POLICY.RUNTIME_JOBS:
            run, jobs = receipt()
            jobs["jobs"] = [job for job in jobs["jobs"] if job["name"] != name]
            jobs["total_count"] -= 1
            with self.subTest(name=name), self.assertRaises(ValueError):
                POLICY.validate_receipt(run, jobs, SHA, "v13", NOW)

    def test_green_workflow_cannot_hide_skipped_or_failed_runtime_jobs(self):
        for outcome in ("skipped", "failure", "cancelled", "timed_out", None):
            run, jobs = receipt()
            jobs["jobs"][0]["conclusion"] = outcome
            with self.subTest(outcome=outcome), self.assertRaises(ValueError):
                POLICY.validate_receipt(run, jobs, SHA, "v13", NOW)

    def test_rejects_duplicate_unknown_and_mixed_job_provenance(self):
        for field, value in (("run_id", 43), ("run_attempt", 2), ("head_sha", "b" * 40),
                             ("workflow_name", "Other"), ("status", "queued"), ("name", "Unknown")):
            run, jobs = receipt()
            jobs["jobs"][0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                POLICY.validate_receipt(run, jobs, SHA, "v13", NOW)
        run, jobs = receipt()
        jobs["jobs"].append(copy.deepcopy(jobs["jobs"][0]))
        jobs["total_count"] += 1
        with self.assertRaises(ValueError):
            POLICY.validate_receipt(run, jobs, SHA, "v13", NOW)

    def test_presentation_only_baseline_is_not_full_runtime_evidence(self):
        run, jobs = receipt()
        for job in jobs["jobs"]:
            job["conclusion"] = "skipped"
        with self.assertRaises(ValueError):
            POLICY.validate_receipt(run, jobs, SHA, "v13", NOW)

    def test_non_candidate_or_malformed_receipt_falls_back_to_full(self):
        with patch.object(POLICY.PLAN, "observe", return_value={
            "base_sha": SHA, "head_sha": "b" * 40, "candidate_scope": "full", "fallback_reasons": []
        }):
            report = POLICY.evaluate(Path.cwd(), SHA, "b" * 40, "v13", {}, {}, NOW)
        self.assertTrue(report["runtime_required"])
        self.assertTrue(report["site_validation_required"])
        self.assertFalse(report["release_evidence_eligible"])


class CommittedInputTests(unittest.TestCase):
    def setUp(self):
        fixtures = POLICY.sibling("test-plan-verification")
        self.fixture = fixtures.GitPolicyTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.run_record, self.jobs = receipt()
        self.run_record["head_sha"] = self.fixture.base
        for job in self.jobs["jobs"]:
            job["head_sha"] = self.fixture.base

    def evaluate(self, base=None):
        return POLICY.evaluate(self.fixture.root, base or self.fixture.base, "HEAD", "v13",
                               self.run_record, self.jobs, NOW)

    def test_real_site_diff_with_valid_receipt_admits_development_only(self):
        self.fixture.write("docs/site.css", "body { color: green; }\n")
        head = self.fixture.commit()
        report = self.evaluate()
        self.assertFalse(report["runtime_required"])
        self.assertTrue(report["site_validation_required"])
        self.assertFalse(report["release_evidence_eligible"])
        self.assertEqual(report["head_sha"], head)
        self.assertEqual(report["baseline_run_id"], 42)

    def test_unverified_intermediate_runtime_commit_cannot_be_hidden_by_css(self):
        self.fixture.write("leaf/src/lib.rs", "pub fn changed_runtime() {}\n")
        intermediate = self.fixture.commit()
        self.fixture.write("docs/site.css", "body { color: orange; }\n")
        self.fixture.commit()
        self.assertTrue(self.evaluate()["runtime_required"])
        # Comparing only the last CSS commit also fails: the receipt is for an older SHA.
        self.assertTrue(self.evaluate(base=intermediate)["runtime_required"])

    def test_same_sha_failed_runtime_receipt_never_admits(self):
        self.fixture.write("docs/site.css", "body { color: blue; }\n")
        self.fixture.commit()
        self.jobs["jobs"][0]["conclusion"] = "failure"
        self.assertTrue(self.evaluate()["runtime_required"])

    def test_executable_site_file_mode_never_admits(self):
        self.fixture.git("update-index", "--chmod=+x", "docs/site.css")
        self.fixture.git("commit", "-qm", "test(policy): executable site file")
        self.assertTrue(self.evaluate()["runtime_required"])

    def test_policy_change_plus_css_never_admits(self):
        self.fixture.write(".github/anything.json", "{}\n")
        self.fixture.write("docs/site.css", "body { color: green; }\n")
        self.fixture.commit()
        self.assertTrue(self.evaluate()["runtime_required"])


if __name__ == "__main__":
    unittest.main()
