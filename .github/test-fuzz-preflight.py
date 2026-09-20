#!/usr/bin/env python3
"""Exercise the real workflow's build selection without running fuzz campaigns."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = (ROOT / ".github/workflows/fuzzing.yml").read_text()
INVENTORY = json.loads((ROOT / ".github/fuzz-targets.json").read_text())
STEP = WORKFLOW.split("      - name: Compile the campaign's selected targets\n", 1)[1]
STEP = STEP.split("\n      - name:", 1)[0]
SCRIPT = textwrap.dedent(STEP.split("        run: |\n", 1)[1])


class BuildSelectionTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="rullst-fuzz-preflight-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / ".github").mkdir()
        (self.root / ".github/fuzz-targets.json").write_text(json.dumps(INVENTORY))
        (self.root / "bin").mkdir()
        cargo = self.root / "bin/cargo"
        cargo.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
pathlib.Path(os.environ["RULLST_FUZZ_CALLS"]).write_text(json.dumps(sys.argv[1:]))
raise SystemExit(int(os.environ.get("RULLST_FUZZ_EXIT", "0")))
''')
        cargo.chmod(0o700)

    def run_preflight(self, mode, directory, target="", status=0):
        log = self.root / "calls.json"
        log.unlink(missing_ok=True)
        env = os.environ | {"PATH": f"{self.root / 'bin'}{os.pathsep}{os.environ['PATH']}",
                            "GITHUB_WORKSPACE": str(self.root), "CAMPAIGN_MODE": mode,
                            "FUZZ_DIRECTORY": directory, "REQUESTED_TARGET": target,
                            "FUZZ_BUILD_TARGET": "x86_64-unknown-linux-gnu",
                            "RULLST_FUZZ_CALLS": str(log), "RULLST_FUZZ_EXIT": str(status)}
        result = subprocess.run(["bash", "-euo", "pipefail", "-c", SCRIPT],
                                cwd=self.root, env=env, capture_output=True, text=True, timeout=10)
        return result, json.loads(log.read_text()) if log.exists() else None

    def test_release_builds_all_targets_in_each_reviewed_package(self):
        directories = sorted({item["dir"] for item in INVENTORY})
        self.assertEqual(len(directories), 11)
        self.assertEqual(len(INVENTORY), 42)
        for directory in directories:
            result, call = self.run_preflight("release", directory)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(call, ["+nightly-2026-08-21", "fuzz", "build", "--target",
                                    "x86_64-unknown-linux-gnu"])

    def test_diagnostic_builds_only_the_exact_requested_target(self):
        for item in INVENTORY:
            result, call = self.run_preflight("diagnostic", item["dir"], item["target"])
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(call, ["+nightly-2026-08-21", "fuzz", "build", "--target",
                                    "x86_64-unknown-linux-gnu", item["target"]])

    def test_invalid_modes_cross_package_unknown_or_flag_targets_never_build(self):
        item = INVENTORY[0]
        other = next(row for row in INVENTORY if row["dir"] != item["dir"])
        for mode, directory, target in (
            ("release", item["dir"], item["target"]),
            ("diagnostic", item["dir"], ""),
            ("diagnostic", item["dir"], "missing"),
            ("diagnostic", item["dir"], other["target"]),
            ("diagnostic", item["dir"], "--no-trace-compares"),
            ("diagnostic", item["dir"], "target\nother"),
            ("release", "unknown/fuzz", ""),
            ("", item["dir"], ""), ("other", item["dir"], ""),
        ):
            result, call = self.run_preflight(mode, directory, target)
            self.assertNotEqual(result.returncode, 0, (mode, directory, target))
            self.assertIsNone(call)

    def test_duplicate_target_is_rejected_before_cargo(self):
        (self.root / ".github/fuzz-targets.json").write_text(json.dumps([*INVENTORY, INVENTORY[0]]))
        result, call = self.run_preflight("diagnostic", INVENTORY[0]["dir"], INVENTORY[0]["target"])
        self.assertNotEqual(result.returncode, 0)
        self.assertIsNone(call)

    def test_build_failure_is_not_tolerated_and_release_budget_is_unchanged(self):
        result, _ = self.run_preflight("release", INVENTORY[0]["dir"], status=101)
        self.assertEqual(result.returncode, 101)
        self.assertIn("campaign_seconds=19800", WORKFLOW)
        self.assertIn("campaign_seconds=300", WORKFLOW)
        self.assertIn('"$TARGET_COUNT" -ne 42', WORKFLOW)
        self.assertIn("is not release evidence", WORKFLOW)
        self.assertIn("CAMPAIGN_MODE: ${{ needs.targets.outputs.mode }}", STEP)
        self.assertIn("REQUESTED_TARGET: ${{ inputs.target }}", STEP)
        self.assertIn("FUZZ_DIRECTORY: ${{ matrix.dir }}", STEP)
        self.assertIn("working-directory: ${{ matrix.dir }}", STEP)


if __name__ == "__main__":
    unittest.main()
