#!/usr/bin/env python3
"""Exercise scope orchestration without network access and lock down CI guards."""

from datetime import datetime, timezone
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest


HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("site_fixtures", HERE / "test-admit-site-only.py")
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot import scope fixtures")
FIXTURES = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(FIXTURES)


class ShellTests(unittest.TestCase):
    def setUp(self):
        module = FIXTURES.POLICY.sibling("test-plan-verification")
        self.fixture = module.GitPolicyTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.temp = tempfile.TemporaryDirectory(prefix="rullst-scope-driver-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name in ("admit-site-only.py", "plan-verification.py", "report-ci-timings.py"):
            self.fixture.write(".github/" + name, (HERE / name).read_text())
        self.base = self.fixture.commit()
        self.fixture.write("docs/site.css", "body { color: green; }\n")
        self.head = self.fixture.commit()
        self.run_record, self.jobs = FIXTURES.receipt()
        self.run_record.update(head_sha=self.base, updated_at=datetime.now(timezone.utc).isoformat())
        for job in self.jobs["jobs"]:
            job["head_sha"] = self.base
        executable = self.root / "gh"
        executable.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
root = pathlib.Path(os.environ["RULLST_SCOPE_FIXTURE"])
(root / "api-called").touch()
if os.environ.get("RULLST_SCOPE_API_FAIL"):
    sys.exit(1)
run = json.loads((root / "run.json").read_text())
endpoint = sys.argv[-1]
if "/workflows/ci.yml/runs?" in endpoint:
    if "branch=" + run["head_branch"] + "&" not in endpoint:
        sys.exit(2)
    print(json.dumps({"workflow_runs": [run]}))
elif "/jobs?" in endpoint:
    print(json.dumps([json.loads((root / "jobs.json").read_text())]))
else:
    print(json.dumps(run))
''')
        executable.chmod(0o755)

    def execute(self, **overrides):
        (self.root / "run.json").write_text(json.dumps(self.run_record))
        (self.root / "jobs.json").write_text(json.dumps(self.jobs))
        env = dict(os.environ, PATH=str(self.root) + os.pathsep + os.environ["PATH"],
                   RULLST_SCOPE_FIXTURE=str(self.root), BASE_SHA=self.base,
                   GITHUB_EVENT_NAME="push", GITHUB_REF="refs/heads/v13",
                   GITHUB_REPOSITORY="Rullst/Rullst", GITHUB_SHA=self.head,
                   GITHUB_OUTPUT=str(self.root / "output"),
                   GITHUB_STEP_SUMMARY=str(self.root / "summary"), RUNNER_TEMP=str(self.root))
        env.update(overrides)
        result = subprocess.run(["bash", str(HERE / "plan-ci-scope.sh")],
                                cwd=self.fixture.root, env=env, capture_output=True, text=True)
        return result, (self.root / "output").read_text().strip()

    def test_valid_baseline_selects_site_validation(self):
        result, output = self.execute()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "runtime_required=false")
        report = json.loads((self.root / "site-ci-admission.json").read_text())
        self.assertFalse(report["release_evidence_eligible"])

    def test_pr_maintenance_manual_and_foreign_repositories_never_query_for_reuse(self):
        for changes in ({"GITHUB_EVENT_NAME": "pull_request"}, {"GITHUB_REF": "refs/heads/v12"},
                        {"GITHUB_EVENT_NAME": "workflow_dispatch"},
                        {"GITHUB_REPOSITORY": "fork/Rullst"}):
            with self.subTest(changes=changes):
                result, output = self.execute(**changes)
                self.assertEqual(result.returncode, 0)
                self.assertTrue(all(line == "runtime_required=true" for line in output.splitlines()))
                self.assertFalse((self.root / "api-called").exists())

    def test_main_requires_its_own_exact_runtime_receipt(self):
        result, output = self.execute(GITHUB_REF="refs/heads/main")
        self.assertEqual(result.returncode, 0)
        self.assertEqual(output, "runtime_required=true")
        self.run_record["head_branch"] = "main"
        (self.root / "output").unlink()
        result, output = self.execute(GITHUB_REF="refs/heads/main")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "runtime_required=false")

    def test_missing_history_or_invalid_sha_is_full(self):
        result, output = self.execute(BASE_SHA="f" * 40)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(output, "runtime_required=true")
        self.assertFalse((self.root / "api-called").exists())

    def test_saas_journey_requires_the_single_supported_platform(self):
        for platform in ("ubuntu-latest", "windows-latest", "macos-latest", "", "all"):
            result, output = self.execute(GITHUB_EVENT_NAME="workflow_dispatch",
                                          RULLST_CI_SHARD="cli-saas-journey",
                                          RULLST_CI_PLATFORM=platform)
            with self.subTest(platform=platform):
                self.assertEqual(result.returncode, 0 if platform == "ubuntu-latest" else 1)
                self.assertTrue(all(line == "runtime_required=true" for line in output.splitlines()))
                self.assertFalse((self.root / "api-called").exists())

    def test_package_diagnostic_rejects_selectors_that_skip_the_archive_job(self):
        for platform in ("ubuntu-latest", "windows-latest", "macos-latest", "", "all"):
            result, output = self.execute(GITHUB_EVENT_NAME="workflow_dispatch",
                                          RULLST_CI_SHARD="packaged-distribution",
                                          RULLST_CI_PLATFORM=platform)
            with self.subTest(platform=platform):
                self.assertEqual(result.returncode, 0 if platform == "all" else 1)
                self.assertTrue(all(line == "runtime_required=true" for line in output.splitlines()))
                self.assertFalse((self.root / "api-called").exists())
                if platform != "all":
                    self.assertIn("require platform=all", result.stderr)

    def test_api_failure_retains_full_ci(self):
        result, output = self.execute(RULLST_SCOPE_API_FAIL="1")
        self.assertEqual(result.returncode, 0)
        self.assertEqual(output, "runtime_required=true")

    def test_incomplete_or_failed_runtime_receipt_retains_full_ci(self):
        self.jobs["jobs"][0]["conclusion"] = "skipped"
        result, output = self.execute()
        self.assertEqual(result.returncode, 0)
        self.assertEqual(output, "runtime_required=true")

    def test_candidate_cannot_replace_its_admission_helper(self):
        self.fixture.write(".github/admit-site-only.py", 'raise RuntimeError("candidate policy executed")\n')
        self.head = self.fixture.commit()
        result, output = self.execute()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "runtime_required=true")
        self.assertNotIn("candidate policy executed", result.stderr)

    def test_late_orchestration_failure_cannot_leave_a_skip_output(self):
        result, output = self.execute(GITHUB_STEP_SUMMARY=str(self.root / "missing" / "summary"))
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(output, "runtime_required=true")


class WorkflowGuardTests(unittest.TestCase):
    def test_all_runtime_jobs_have_failure_safe_development_only_guards(self):
        text = (HERE / "workflows/ci.yml").read_text().split("\njobs:\n", 1)[1]
        entries = re.split(r"(?m)^  ([a-z][a-z0-9-]+):\n", text)
        jobs = dict(zip(entries[1::2], entries[2::2]))
        runtime = {"check", "test", "strict-database-features", "redis-rate-limit",
                   "feature-boundaries", "threat-model-release-minimum", "versioned-ai-evals",
                   "generated-release-access", "facade-composition", "labs-isolation", "storage-s3", "msrv"}
        distribution = {"native-cli-artifacts", "packaged-distribution"}
        self.assertEqual(set(jobs), runtime | distribution | {"scope", "site-validation", "quality-scorecard"})
        for name, body in jobs.items():
            self.assertEqual(len(re.findall(r"(?m)^    if:", body)), 1 if name != "scope" else 0, name)
        for name in distribution:
            guard = jobs[name].split("    runs-on:", 1)[0].split("    uses:", 1)[0]
            self.assertIn("github.event_name == 'workflow_dispatch'", guard)
            self.assertIn("inputs.platform == 'all'", guard)
            self.assertIn("inputs.shard == 'all'", guard)
        for name in runtime:
            with self.subTest(job=name):
                guard = jobs[name].split("    runs-on:", 1)[0]
                for clause in ("needs: scope", "!cancelled()", "github.event_name != 'push'",
                               "github.ref != 'refs/heads/v13'", "github.ref != 'refs/heads/main'", "needs.scope.result != 'success'",
                               "needs.scope.outputs.runtime_required != 'false'"):
                    self.assertIn(clause, guard)
        site = jobs["site-validation"]
        for required in ("github.event_name == 'push'", "refs/heads/v13", "refs/heads/main", "mdbook build docs",
                         "python3 .github/validate-site.py", "node .github/site-browser-smoke.mjs"):
            self.assertIn(required, site)
        self.assertIn("github.event_name != 'push'", jobs["quality-scorecard"])
        scorecard_guard = jobs["quality-scorecard"].split("    if: >-\n", 1)[1].split("    needs:", 1)[0]
        self.assertIn("!cancelled()", scorecard_guard)
        self.assertNotIn("always()", scorecard_guard)
        self.assertIn("RULLST_CI_SHARD: ${{ inputs.shard }}", jobs["scope"])
        self.assertIn("RULLST_CI_PLATFORM: ${{ inputs.platform }}", jobs["scope"])
        self.assertIn("inputs.shard != 'packaged-distribution' || inputs.platform == 'all'", jobs["check"])


if __name__ == "__main__":
    unittest.main()
