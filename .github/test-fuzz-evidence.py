#!/usr/bin/env python3
"""Negative controls for provenance, complete duration and evidence selection."""

import copy
from datetime import datetime, timedelta, timezone
import json
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest
from unittest.mock import patch

import fuzz_evidence as evidence
from fuzz_evidence_inputs import ROOT

NOW = datetime(2026, 9, 19, 15, tzinfo=timezone.utc)
SHA = "a" * 40
INVENTORY = json.loads((ROOT / ".github/fuzz-targets.json").read_text())


class Source:
    root = ROOT
    inventory = INVENTORY
    contract = "execution"
    global_hash = "shared"
    release_branch = "main"
    packages = {item["dir"] for item in INVENTORY}
    def __init__(self, sha=SHA, root=ROOT):
        self.sha = sha
        self.changed = set()
    def fingerprint(self, directory):
        return directory + (" changed" if directory in self.changed else " original")
    def ancestor_of(self, candidate):
        return True


def run_fixture(run_id=1):
    return {"id": run_id, "run_attempt": 1, "head_sha": SHA, "head_branch": "main",
            "event": "workflow_dispatch", "status": "completed", "conclusion": "success",
            "path": evidence.WORKFLOW, "created_at": (NOW - timedelta(hours=10)).isoformat(),
            "repository": {"full_name": "Rullst/Rullst"},
            "head_repository": {"full_name": "Rullst/Rullst"}}


def jobs_fixture(run_id=1):
    success = {"status": "completed", "conclusion": "success"}
    jobs = [{"name": evidence.BOUNDARY, **success}]
    for directory in sorted(Source.packages):
        jobs.append({"name": f"Compile fuzz package {directory}", **success})
    for number, item in enumerate(INVENTORY, 1):
        jobs.append({"name": f'Fuzz {item["target"]}', "id": number,
                     "run_id": run_id, "head_sha": SHA,
                     "completed_at": NOW.isoformat(), **success,
                     "steps": [{"name": "Run bounded target campaign", **success,
                                "started_at": (NOW - timedelta(seconds=evidence.SECONDS)).isoformat(),
                                "completed_at": NOW.isoformat()}]})
    return jobs


class FakeGitHub:
    repository = "Rullst/Rullst"
    branch = "main"
    def __init__(self):
        self.items = [run_fixture()]
        self.job_sets = {1: jobs_fixture()}
    def runs(self):
        return self.items
    def jobs(self, run):
        return self.job_sets[run["id"]]


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.github = FakeGitHub()
        self.candidate = Source("b" * 40)
        self.mock = patch.object(evidence, "Snapshot", Source)
        self.mock.start()
        self.addCleanup(self.mock.stop)
    def plan(self):
        return evidence.plan(self.candidate, self.github, NOW)

    def test_v13_evidence_cannot_be_borrowed_from_main(self):
        self.candidate.release_branch = "v13"
        self.github.branch = "v13"
        self.assertEqual(len(self.plan()["selected"]), 42)
        self.github.items[0]["head_branch"] = "v13"
        # The source itself must carry the matching release-line policy.
        self.assertEqual(len(self.plan()["selected"]), 42)
        with patch.object(Source, "release_branch", "v13"):
            report = self.plan()
        self.assertEqual(report["selected"], [])
        self.assertEqual(report["release_branch"], "v13")

    def test_v12_maintenance_cannot_borrow_main_development_evidence(self):
        self.candidate.release_branch = "v12"
        self.github.branch = "v12"
        self.assertEqual(self.plan()["selected"], INVENTORY)
        self.github.items[0]["head_branch"] = "v12"
        self.assertEqual(self.plan()["selected"], INVENTORY)
        with patch.object(Source, "release_branch", "v12"):
            self.assertEqual(self.plan()["selected"], [])

    def test_api_selection_must_match_candidate_policy_even_for_a_full_campaign(self):
        self.github.branch = "v13"
        with self.assertRaises(ValueError):
            self.plan()
        with self.assertRaises(ValueError):
            evidence.plan(self.candidate, self.github, NOW, force_full=True)

    def test_a_pre_expansion_campaign_requires_fresh_execution_not_borrowed_evidence(self):
        with patch.object(evidence, "Snapshot", side_effect=evidence.FuzzSurfaceChanged("legacy surface")):
            report = self.plan()
        self.assertEqual(report["selected"], INVENTORY)
        self.assertEqual(report["reused"], [])
        self.assertEqual(report["considered"], [])
        # Malformed history is not silently generalized into this exception.
        with patch.object(evidence, "Snapshot", side_effect=ValueError("bad history")):
            with self.assertRaisesRegex(ValueError, "bad history"):
                self.plan()

    def test_github_query_uses_only_the_selected_release_branch(self):
        github = evidence.GitHub("Rullst/Rullst", "fixture", branch="v13")
        with patch.object(github, "get", return_value={"workflow_runs": []}) as get:
            self.assertEqual(github.runs(), [])
        self.assertIn("branch=v13", get.call_args.args[0])
        with self.assertRaises(ValueError):
            evidence.GitHub("Rullst/Rullst", "fixture", branch="feature")

    def test_all_targets_reuse_original_jobs_and_keep_source_provenance(self):
        report = self.plan()
        self.assertEqual(report["selected"], [])
        self.assertEqual(len(report["reused"]), 42)
        self.assertTrue(all(item["source_sha"] == SHA and item["run_id"] == 1
                            and item["run_attempt"] == 1 for item in report["reused"]))

    def test_changed_package_selects_all_sibling_targets(self):
        self.candidate.changed.add("rullst-mail/fuzz")
        report = self.plan()
        self.assertEqual(len(report["selected"]), 4)
        self.assertEqual({item["dir"] for item in report["selected"]}, {"rullst-mail/fuzz"})
        self.assertEqual(len(report["reused"]), 38)

    def test_new_partial_campaign_combines_only_actual_jobs_with_old_equivalent_jobs(self):
        self.candidate.changed.add("rullst-mail/fuzz")
        newer = run_fixture(2)
        newer.update(head_sha=self.candidate.sha, created_at=(NOW - timedelta(hours=6)).isoformat())
        jobs = [job for job in jobs_fixture(2) if job["name"] == evidence.BOUNDARY
                or job["name"] == "Compile fuzz package rullst-mail/fuzz"
                or job["name"] in {f'Fuzz {item["target"]}' for item in INVENTORY if item["dir"] == "rullst-mail/fuzz"}]
        for job in jobs:
            job["head_sha"] = self.candidate.sha
        self.github.items.append(newer)
        self.github.job_sets[2] = jobs
        with patch.object(evidence, "Snapshot", side_effect=lambda sha, root:
                          self.candidate if sha == self.candidate.sha else Source(sha)):
            report = self.plan()
        self.assertEqual(report["selected"], [])
        for item in report["reused"]:
            self.assertEqual(item["run_id"], 2 if item["dir"] == "rullst-mail/fuzz" else 1)

    def test_no_prior_evidence_or_force_full_selects_every_target(self):
        self.assertEqual(len(evidence.plan(self.candidate, self.github, NOW, force_full=True)["selected"]), 42)
        self.github.items = []
        self.assertEqual(len(self.plan()["selected"]), 42)

    def test_short_diagnostic_skipped_or_missing_targets_are_never_credited(self):
        baseline = jobs_fixture()
        for mutation in ("missing", "skipped", "short", "missing-step", "duplicate-step", "bad-clock", "no-preflight"):
            with self.subTest(mutation=mutation):
                jobs = copy.deepcopy(baseline)
                job = jobs[-1]
                if mutation == "missing":
                    jobs.pop()
                elif mutation == "skipped":
                    job["conclusion"] = "skipped"
                elif mutation == "short":
                    job["steps"][0]["started_at"] = (NOW - timedelta(seconds=300)).isoformat()
                elif mutation == "missing-step":
                    job["steps"] = []
                elif mutation == "duplicate-step":
                    job["steps"] *= 2
                elif mutation == "bad-clock":
                    job["steps"][0]["started_at"] = "invalid"
                else:
                    jobs = [row for row in jobs if row["name"] != f'Compile fuzz package {INVENTORY[-1]["dir"]}']
                self.github.job_sets[1] = jobs
                report = self.plan()
                self.assertIn(INVENTORY[-1], report["selected"])

    def test_wrong_repository_branch_event_path_age_or_run_identity_is_rejected(self):
        for key, value in (("repository", {"full_name": "attacker/fork"}),
                           ("head_repository", {"full_name": "attacker/fork"}),
                           ("head_branch", "feature"), ("event", "pull_request"),
                           ("path", ".github/workflows/other.yml"), ("head_sha", "--help"),
                           ("id", True), ("run_attempt", 0),
                           ("created_at", (NOW - timedelta(days=8)).isoformat()),
                           ("created_at", (NOW + timedelta(seconds=1)).isoformat())):
            with self.subTest(key=key, value=value):
                run = run_fixture()
                run[key] = value
                self.github.items = [run]
                self.assertEqual(len(self.plan()["selected"]), 42)

    def test_unrelated_commit_or_execution_change_requires_fresh_evidence(self):
        with patch.object(Source, "ancestor_of", return_value=False):
            self.assertEqual(len(self.plan()["selected"]), 42)
        self.candidate.contract = "different toolchain or command"
        self.assertEqual(len(self.plan()["selected"]), 42)

    def test_incomplete_failed_or_missing_boundary_cannot_authorize_reuse(self):
        for state in ("failure", "cancelled", "skipped"):
            self.github.job_sets[1][0]["conclusion"] = state
            self.assertEqual(len(self.plan()["selected"]), 42)
        self.github.job_sets[1] = jobs_fixture()[1:]
        self.assertEqual(len(self.plan()["selected"]), 42)

    def test_newer_matching_failure_blocks_older_success(self):
        newer = run_fixture(2)
        newer.update(created_at=(NOW - timedelta(hours=1)).isoformat(), conclusion="failure")
        jobs = jobs_fixture(2)
        jobs[-1]["conclusion"] = "failure"
        self.github.items.append(newer)
        self.github.job_sets[2] = jobs
        report = self.plan()
        self.assertEqual(report["selected"], [INVENTORY[-1]])
        self.assertEqual(report["blocked_by_newer_run"], [INVENTORY[-1]["target"]])
        newer.update(created_at=(NOW - timedelta(hours=12)).isoformat(),
                     run_started_at=(NOW - timedelta(hours=1)).isoformat(), run_attempt=2)
        self.assertEqual(self.plan()["selected"], [INVENTORY[-1]])

    def test_reuse_receipt_cannot_renew_age_or_replace_original_job(self):
        self.github.items[0]["created_at"] = (NOW - timedelta(days=8)).isoformat()
        receipt = run_fixture(2)
        self.github.items.append(receipt)
        self.github.job_sets[2] = [jobs_fixture()[0]]
        self.assertEqual(len(self.plan()["selected"]), 42)

    def test_duplicate_job_names_and_truncated_api_jobs_fail_closed(self):
        self.github.job_sets[1].append(self.github.job_sets[1][-1])
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.plan()
        api = evidence.GitHub("Rullst/Rullst", "fixture-token")
        with patch.object(api, "get", return_value={"total_count": 52, "jobs": []}):
            with self.assertRaisesRegex(ValueError, "incomplete"):
                api.jobs(run_fixture())

    def test_api_requests_pin_attempt_and_reject_external_hosts(self):
        api = evidence.GitHub("Rullst/Rullst", "fixture-token")
        with patch.object(api, "get", return_value={"total_count": 0, "jobs": []}) as get:
            run = run_fixture()
            run["run_attempt"] = 3
            api.jobs(run)
            get.assert_called_once_with("actions/runs/1/attempts/3/jobs?per_page=100")
        with self.assertRaises(ValueError):
            evidence.GitHub("Rullst/Rullst", "fixture-token", "https://example.invalid")


class WorkflowBoundaryTests(unittest.TestCase):
    def test_actual_scheduler_exports_empty_partial_and_full_matrices(self):
        workflow = (ROOT / ".github/workflows/fuzzing.yml").read_text()
        step = workflow.split("      - name: Validate and export matrix\n", 1)[1].split("\n  preflight:", 1)[0]
        script = textwrap.dedent(step.split("        run: |\n", 1)[1])
        with tempfile.TemporaryDirectory(prefix="rullst-fuzz-scheduler-") as directory:
            root = Path(directory)
            (root / ".github").mkdir()
            (root / ".github/validate-fuzz-targets.py").write_text("pass\n")
            (root / ".github/fuzz-targets.json").write_text(json.dumps(INVENTORY))
            (root / ".github/fuzz_evidence.py").write_text(
                "import json, os, pathlib, sys\n"
                "plan = json.loads(os.environ['FIXTURE_PLAN'])\n"
                "pathlib.Path(sys.argv[sys.argv.index('--output')+1]).write_text(json.dumps(plan))\n"
                "pathlib.Path('args.json').write_text(json.dumps(sys.argv[1:]))\n")
            for count, force in ((0, False), (4, False), (42, True)):
                plan = {"selected": INVENTORY[:count], "reused": INVENTORY[count:]}
                output = root / "outputs"
                output.unlink(missing_ok=True)
                env = os.environ | {"CAMPAIGN_MODE": "release", "REQUESTED_TARGET": "",
                    "FORCE_FULL": str(force).lower(), "RUNNER_TEMP": directory,
                    "GITHUB_OUTPUT": str(output), "FIXTURE_PLAN": json.dumps(plan)}
                result = subprocess.run(["bash", "-euo", "pipefail", "-c", script],
                                        cwd=root, env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                values = dict(line.split("=", 1) for line in output.read_text().splitlines())
                self.assertEqual(json.loads(values["matrix"]), {"include": INVENTORY[:count]})
                self.assertEqual(values["selected_count"], str(count))
                self.assertEqual(values["target_count"], "42")
                self.assertEqual(values["reused_count"], str(42-count))
                self.assertEqual(values["campaign_seconds"], "19800")
                self.assertEqual("--force-full" in json.loads((root / "args.json").read_text()), force)

    def test_actual_shell_boundary_handles_full_partial_and_reused_only_campaigns(self):
        workflow = (ROOT / ".github/workflows/fuzzing.yml").read_text()
        step = workflow.split("      - name: Record strict result and release eligibility\n", 1)[1]
        script = textwrap.dedent(step.split("        run: |\n", 1)[1].split("\n      - name:", 1)[0])
        with tempfile.TemporaryDirectory(prefix="rullst-fuzz-boundary-") as directory:
            for selected, reused, preflight, fuzz, targets, expected in (
                (42, 0, "success", "success", "success", 0),
                (4, 38, "success", "success", "success", 0),
                (0, 42, "skipped", "skipped", "success", 0),
                (4, 38, "success", "failure", "success", 1),
                (4, 38, "skipped", "skipped", "success", 1),
                (0, 42, "skipped", "skipped", "failure", 1),
                (0, 41, "skipped", "skipped", "success", 1),
            ):
                env = os.environ | {"GITHUB_STEP_SUMMARY": str(Path(directory) / "summary.md"),
                    "CAMPAIGN_MODE": "release", "TARGET_COUNT": str(selected + reused),
                    "SELECTED_COUNT": str(selected), "REUSED_COUNT": str(reused),
                    "PREFLIGHT_RESULT": preflight, "FUZZ_RESULT": fuzz, "TARGETS_RESULT": targets}
                result = subprocess.run(["bash", "-euo", "pipefail", "-c", script],
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode, expected, ((selected, reused, preflight, fuzz, targets), result.stderr))


if __name__ == "__main__":
    unittest.main()
