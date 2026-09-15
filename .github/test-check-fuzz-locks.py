#!/usr/bin/env python3
"""Prove lock preflight resolves dependencies and stops before compilation."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / ".github/check-fuzz-locks.sh"


class LockPreflightTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="rullst-lock-preflight-")
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)

    def test_cargo_no_deps_does_not_prove_lock_consistency(self):
        for directory in (self.root, self.root / "dep"):
            (directory / "src").mkdir(parents=True)
            (directory / "src/lib.rs").write_text("pub fn example() {}\n")
        (self.root / "Cargo.toml").write_text(
            '[package]\nname="lock-probe"\nversion="0.1.0"\nedition="2024"\n'
            '[workspace]\n[dependencies]\nlock-dep={path="dep"}\n'
        )
        (self.root / "dep/Cargo.toml").write_text(
            '[package]\nname="lock-dep"\nversion="1.0.0"\nedition="2024"\n'
        )
        subprocess.run(["cargo", "generate-lockfile", "--offline"], cwd=self.root,
                       check=True, capture_output=True)
        before = (self.root / "Cargo.lock").read_bytes()
        (self.root / "dep/Cargo.toml").write_text(
            '[package]\nname="lock-dep"\nversion="1.0.1"\nedition="2024"\n'
        )
        command = ["cargo", "metadata", "--locked", "--offline", "--format-version", "1"]
        old = subprocess.run([*command, "--no-deps"], cwd=self.root, capture_output=True)
        full = subprocess.run(command, cwd=self.root, capture_output=True)
        self.assertEqual(old.returncode, 0, old.stderr)
        self.assertNotEqual(full.returncode, 0)
        self.assertIn(b"--locked", full.stderr)
        self.assertEqual((self.root / "Cargo.lock").read_bytes(), before)
        self.assertFalse((self.root / "target").exists())

    def run_preflight(self, *arguments, fail_at=""):
        recorder = self.root / "cargo"
        recorder.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
log = pathlib.Path(os.environ["RULLST_LOCK_LOG"])
with log.open("a") as output:
    output.write(json.dumps(sys.argv[1:]) + "\\n")
count = len(log.read_text().splitlines())
sys.exit(101 if str(count) == os.environ.get("RULLST_LOCK_FAIL_AT") else 0)
''')
        recorder.chmod(0o755)
        log = self.root / "calls.jsonl"
        env = dict(os.environ, PATH=str(self.root) + os.pathsep + os.environ["PATH"],
                   RULLST_LOCK_LOG=str(log), RULLST_LOCK_FAIL_AT=fail_at)
        result = subprocess.run(["bash", str(SCRIPT), *arguments], cwd=ROOT,
                                env=env, capture_output=True, text=True)
        calls = [json.loads(line) for line in log.read_text().splitlines()] if log.exists() else []
        return result, calls

    def test_every_inventory_package_gets_a_full_locked_graph_check(self):
        result, calls = self.run_preflight("--offline")
        self.assertEqual(result.returncode, 0, result.stderr)
        inventory = json.loads((ROOT / ".github/fuzz-targets.json").read_text())
        directories = sorted({item["dir"] for item in inventory})
        self.assertEqual(calls, [
            ["metadata", "--manifest-path", directory + "/Cargo.toml", "--locked",
             "--format-version", "1", "--filter-platform", "x86_64-unknown-linux-gnu", "--offline"]
            for directory in directories
        ])

    def test_stale_graph_fails_without_continuing_or_running_tests(self):
        result, calls = self.run_preflight(fail_at="2")
        self.assertEqual(result.returncode, 101)
        self.assertEqual(len(calls), 2)
        self.assertTrue(all(call[0] == "metadata" for call in calls))

    def test_unknown_options_are_rejected_before_cargo(self):
        result, calls = self.run_preflight("--no-deps")
        self.assertEqual(result.returncode, 2)
        self.assertEqual(calls, [])

    def test_ci_checks_graphs_before_clippy_and_campaigns_do_not_use_no_deps(self):
        ci = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertLess(ci.index("bash .github/check-fuzz-locks.sh"), ci.index("cargo clippy"))
        campaign = (ROOT / ".github/workflows/fuzzing.yml").read_text()
        checks = [line for line in campaign.splitlines() if " metadata " in line]
        self.assertEqual(len(checks), 2)
        self.assertTrue(all("--locked" in line and "--filter-platform" in line
                            and "--no-deps" not in line for line in checks))


if __name__ == "__main__":
    unittest.main()
