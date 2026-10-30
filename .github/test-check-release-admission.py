#!/usr/bin/env python3
"""Regression tests for exact-SHA release admission."""

from __future__ import annotations

import importlib.util
import contextlib
import io
import sys
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).with_name("check-release-admission.py")
SPEC = importlib.util.spec_from_file_location("check_release_admission", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot import {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


SHA = "0123456789abcdef0123456789abcdef01234567"


class ReleaseAdmissionTests(unittest.TestCase):
    def exercise_candidate(self, reusable, missing_workflow=None, missing_boundary=False,
                           scheduled=()):
        policy = MODULE.load_object(SCRIPT.with_name("release-required-workflows.json"))
        _, requirements = MODULE.validate_policy(policy)
        by_id = {index: requirement for index, requirement in enumerate(requirements, 1)}
        by_name = {requirement.workflow: index for index, requirement in by_id.items()}

        def runs(api, repository, workflow, sha, branch, event, token):
            # A workflow in `scheduled` only has its weekly/nightly run on the
            # candidate, even if the API ignored the requested event filter.
            return {"workflow_runs": [{"id": by_name[workflow], "head_sha":
                    "f" * 40 if workflow == missing_workflow else sha,
                    "head_branch": branch,
                    "event": "schedule" if workflow in scheduled else event,
                    "status": "completed", "conclusion": "success"}]}

        def jobs(api, repository, run_id, token):
            requirement = by_id[run_id]
            names = requirement.required_jobs
            if requirement.workflow == "fuzzing.yml":
                names = () if missing_boundary else ("Fuzz campaign evidence boundary",)
            return {"total_count": len(names), "jobs": [
                {"name": name, "conclusion": "success"} for name in names]}

        with (patch.object(sys, "argv", [str(SCRIPT), "--repository", "Rullst/Rullst",
                                         "--sha", SHA, "--token", "fixture"]),
              patch.object(MODULE, "fetch_runs", side_effect=runs),
              patch.object(MODULE, "fetch_jobs", side_effect=jobs),
              patch.object(MODULE, "equivalent_fuzz_jobs", return_value=reusable) as verifier,
              contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO())):
            result = MODULE.main()
        return result, verifier

    def test_equivalent_fuzz_jobs_require_an_independent_verifier(self):
        result, verifier = self.exercise_candidate(True)
        self.assertEqual(result, 0)
        verifier.assert_called_once()
        self.assertEqual(verifier.call_args.args[:4], ("Rullst/Rullst", SHA, "fixture", "https://api.github.com"))
        self.assertEqual(len(verifier.call_args.args[4]), 43)
        result, _ = self.exercise_candidate(False)
        self.assertEqual(result, 1)

    def test_reuse_never_bypasses_current_fuzz_workflow_or_boundary(self):
        for options in ({"missing_workflow": "fuzzing.yml"}, {"missing_boundary": True}):
            result, verifier = self.exercise_candidate(True, **options)
            self.assertEqual(result, 1)
            verifier.assert_not_called()

    def test_other_workflows_still_require_success_on_the_exact_candidate(self):
        for workflow in ("ci.yml", "miri.yml", "kani.yml", "codeql.yml"):
            result, _ = self.exercise_candidate(True, missing_workflow=workflow)
            self.assertEqual(result, 1, workflow)

    def test_scheduled_runs_never_replace_the_dispatched_admission_runs(self):
        # Miri and Kani also run weekly (sanitizers nightly, proptest weekly)
        # on main; a scheduled success on the exact candidate is not evidence.
        for workflow in ("miri.yml", "kani.yml", "sanitizers.yml", "proptest.yml", "ci.yml"):
            with self.subTest(workflow=workflow):
                stderr = io.StringIO()
                with contextlib.redirect_stderr(stderr):
                    result, _ = self.exercise_candidate(True, scheduled={workflow})
                self.assertEqual(result, 1)
        result, _ = self.exercise_candidate(True, scheduled=())
        self.assertEqual(result, 0)
        payload = {"workflow_runs": [{"id": 7, "head_sha": SHA, "head_branch": "main",
                                      "event": "schedule", "status": "completed",
                                      "conclusion": "success"}]}
        self.assertEqual(MODULE.exact_successful_run_ids(payload, SHA, "main", "workflow_dispatch"), [])
        self.assertEqual(MODULE.exact_successful_run_ids(payload, SHA, "main", "schedule"), [7])

    def test_policy_cannot_accept_scheduled_evidence(self):
        policy = MODULE.load_object(SCRIPT.with_name("release-required-workflows.json"))
        for workflow in ("miri.yml", "kani.yml"):
            entry = next(item for item in policy["workflows"] if item["workflow"] == workflow)
            entry["event"] = "schedule"
            with self.subTest(workflow=workflow), self.assertRaisesRegex(SystemExit, "unsupported event"):
                MODULE.validate_policy(policy)
            entry["event"] = "workflow_dispatch"

    def test_scheduled_manual_gates_keep_dispatch_and_their_own_concurrency_group(self):
        workflows = SCRIPT.parent / "workflows"
        for workflow in sorted(MODULE.MANDATORY_MANUAL_WORKFLOWS):
            text = (workflows / workflow).read_text(encoding="utf-8")
            triggers = "\n" + text.split("\non:\n", 1)[1].split("\nconcurrency:\n", 1)[0]
            group = text.split("\nconcurrency:\n", 1)[1].split("\n\n", 1)[0]
            with self.subTest(workflow=workflow):
                self.assertRegex(triggers, r"(?m)^  workflow_dispatch:")
                self.assertIn("group: ${{ github.workflow }}-", group)
                self.assertIn("cancel-in-progress: true", group)
                if "\n  schedule:\n" in triggers:
                    # The dispatched run on main keeps the plain group; a
                    # scheduled run on the same ref gets a suffixed one.
                    self.assertRegex(group, r"\$\{\{ github\.event_name == 'schedule' && '-[a-z]+' \|\| '' \}\}")
        for workflow in ("miri.yml", "kani.yml"):
            text = (workflows / workflow).read_text(encoding="utf-8")
            self.assertRegex(text.split("\nconcurrency:\n", 1)[0], r"(?m)^  schedule:\n    - cron: '[0-9]+ [0-9]+ \* \* [0-6]'")

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

    def test_release_cannot_omit_or_skip_the_packaged_labs_consumer(self) -> None:
        policy = MODULE.load_object(SCRIPT.parent / "release-required-workflows.json")
        _, requirements = MODULE.validate_policy(policy)
        required = next(item.required_jobs for item in requirements if item.workflow == "ci.yml")
        # The runner candidate and its isolated job were removed from 13.0; a
        # stale requirement would make every release unadmittable.
        self.assertNotIn("Isolated Labs acceptance (Linux)", required)
        labs = "Packaged distribution and installed CLI"
        self.assertIn(labs, required)
        others = [{"name": name, "conclusion": "success"} for name in required if name != labs]
        for outcome in ("missing", "skipped", "failure", "cancelled", None, "success"):
            jobs = others if outcome == "missing" else [*others, {"name": labs, "conclusion": outcome}]
            with self.subTest(outcome=outcome):
                self.assertEqual(MODULE.required_jobs_succeeded(
                    {"total_count": len(jobs), "jobs": jobs}, required
                ), outcome == "success")

    def test_package_diagnostic_cannot_admit_a_release(self) -> None:
        policy = MODULE.load_object(SCRIPT.parent / "release-required-workflows.json")
        _, requirements = MODULE.validate_policy(policy)
        required = next(item.required_jobs for item in requirements if item.workflow == "ci.yml")
        jobs = [
            {"name": "Verification scope", "conclusion": "success"},
            {"name": "Code Quality & Format", "conclusion": "success"},
            {"name": "Packaged distribution and installed CLI", "conclusion": "success"},
        ]
        self.assertIn("Packaged distribution and installed CLI", required)
        self.assertFalse(MODULE.required_jobs_succeeded(
            {"total_count": len(jobs), "jobs": jobs}, required
        ))

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
                        {"workflow": "../supply-chain.yml", "event": "push"},
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
                    {"workflow": "supply-chain.yml", "event": "push"},
                ],
            }
        )
        self.assertEqual(branch, "main")
        self.assertEqual(
            [item.workflow for item in requirements],
            sorted(
                ["supply-chain.yml", *(item["workflow"] for item in manual_gates)]
            ),
        )

    def test_policy_workflows_exist_and_declare_their_event(self):
        # Admission looks runs up by workflow file name and event, so a renamed
        # or merged workflow must be renamed in the policy too, or no run can
        # ever satisfy it.
        policy = MODULE.load_object(SCRIPT.parent / "release-required-workflows.json")
        branch, requirements = MODULE.validate_policy(policy)
        workflows = SCRIPT.parent / "workflows"
        for requirement in requirements:
            path = workflows / requirement.workflow
            with self.subTest(workflow=requirement.workflow):
                self.assertTrue(path.is_file(), f"{path} does not exist")
                text = path.read_text(encoding="utf-8")
                triggers = text.split("\non:\n", 1)[1].split("\n\n", 1)[0]
                self.assertRegex(triggers, rf"(?m)^  {requirement.event}:")
                if requirement.event == "push":
                    push = triggers.split("  push:\n", 1)[1]
                    self.assertRegex(push.split("\n  pull_request:", 1)[0],
                                     rf"branches: \[[^\]]*\b{branch}\b")
        names = {requirement.workflow for requirement in requirements}
        for retired in ("audit.yml", "cargo-deny.yml", "security-audit.yml"):
            self.assertNotIn(retired, names)
            self.assertFalse((workflows / retired).exists(), retired)

    def test_v13_policy_requires_both_additional_privacy_targets(self):
        policy = MODULE.load_object(SCRIPT.parent / "release-required-workflows.json")
        branch, _ = MODULE.validate_policy(policy)
        self.assertEqual(branch, "main")
        fuzz = next(item for item in policy["workflows"] if item["workflow"] == "fuzzing.yml")
        for target in ("Fuzz fuzz_age_challenge_token", "Fuzz fuzz_age_attestation"):
            fuzz["required_jobs"].remove(target)
            with self.assertRaisesRegex(SystemExit, "all 42 target jobs"):
                MODULE.validate_policy(policy)
            fuzz["required_jobs"].append(target)


if __name__ == "__main__":
    unittest.main()
