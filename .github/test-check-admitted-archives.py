#!/usr/bin/env python3
"""Offline fixtures for matching release archives to an admitted consumer test."""

import importlib.util
import json
from pathlib import Path
import subprocess
import unittest

SCRIPT = Path(__file__).with_name("check-admitted-archives.py")
SPEC = importlib.util.spec_from_file_location("check_admitted_archives", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot import {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

SHA = "a" * 40
CHECKSUMS = "1111  rullst-13.0.0.crate\n"


def fake_github(runs, jobs, artifacts):
    """runs: list of run dicts; jobs: run id -> job conclusion; artifacts: run id -> text."""

    def run(*args):
        if args[0] == "api" and "/actions/workflows/ci.yml/runs" in args[1]:
            return json.dumps({"workflow_runs": runs})
        if args[0] == "api" and "/jobs" in args[1]:
            run_id = int(args[1].split("/runs/")[1].split("/")[0])
            return json.dumps({"jobs": [{"name": MODULE.JOB, "conclusion": jobs[run_id]}]})
        if args[0] == "run" and args[1] == "download":
            run_id = int(args[2])
            if run_id not in artifacts:
                raise subprocess.CalledProcessError(1, "gh")
            directory = Path(args[args.index("--dir") + 1])
            (directory / f"{MODULE.ARTIFACT}.txt").write_text(artifacts[run_id])
            return ""
        raise AssertionError(args)

    return run


def ci_run(run_id, conclusion="success", sha=SHA, branch="main"):
    return {"id": run_id, "head_sha": sha, "head_branch": branch,
            "event": "workflow_dispatch", "conclusion": conclusion}


class AdmittedArchiveTests(unittest.TestCase):
    def match(self, runs, jobs, artifacts):
        github = fake_github(runs, jobs, artifacts)
        return MODULE.matching_run(github, "Rullst/Rullst", SHA, "main", CHECKSUMS)

    def test_identical_archives_from_an_admitted_run_match(self):
        self.assertEqual(self.match([ci_run(7)], {7: "success"}, {7: CHECKSUMS}), 7)

    def test_different_archives_do_not_match(self):
        self.assertIsNone(self.match([ci_run(7)], {7: "success"}, {7: "2222  rullst-13.0.0.crate\n"}))

    def test_failed_consumer_job_or_missing_artifact_does_not_match(self):
        self.assertIsNone(self.match([ci_run(7)], {7: "failure"}, {7: CHECKSUMS}))
        self.assertIsNone(self.match([ci_run(7)], {7: "success"}, {}))

    def test_other_sha_branch_or_failed_run_is_ignored(self):
        runs = [ci_run(1, sha="b" * 40), ci_run(2, branch="feature"), ci_run(3, conclusion="failure"),
                ci_run(4)]
        jobs = {run_id: "success" for run_id in (1, 2, 3, 4)}
        artifacts = {run_id: CHECKSUMS for run_id in (1, 2, 3)}
        self.assertIsNone(self.match(runs, jobs, artifacts))
        artifacts[4] = CHECKSUMS
        self.assertEqual(self.match(runs, jobs, artifacts), 4)


if __name__ == "__main__":
    unittest.main()
