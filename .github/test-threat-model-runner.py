#!/usr/bin/env python3
"""Prove runner wiring, exact inventory and unchanged hash shards without builds.

The Cargo double is only orchestration evidence. test-exact-rust-test.py also
exercises the evidence parser with a real compiled Rust test harness.
"""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = json.loads((ROOT / ".github/threat-model-release-minimum.json").read_text())

FAKE_CARGO = r'''#!/usr/bin/env python3
import json, os, sys
args = sys.argv[1:]
with open(os.environ["RULLST_CARGO_CALLS"], "a") as log:
    log.write(json.dumps(args) + "\n")
if args[0] == "fetch":
    raise SystemExit(0)
if args[0] != "test" or "--list" in args:
    raise SystemExit(80)
name = args[args.index("--") - 1]
mode = os.environ.get("RULLST_CARGO_TEST_MODE", "pass")
if mode == "missing":
    print("running 0 tests")
    print("test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out; finished in 0.00s")
elif mode == "ignored":
    print("running 1 test")
    print(f"test {name} ... ignored")
    print("test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 20 filtered out; finished in 0.00s")
else:
    print("running 1 test")
    print(f"test {name} ... ok")
    print("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out; finished in 0.00s")
if mode == "failed_exit":
    raise SystemExit(9)
'''


def key(row):
    return ":".join(row[field] for field in ("crate", "target_kind", "target", "test_filter"))


class RunnerTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="rullst-threat-runner-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        for path in (".github", "docs/src", "evidence", "bin"):
            (self.root / path).mkdir(parents=True)
        for name in ("check-threat-model-release-minimum.sh", "assert-exact-rust-test.py"):
            shutil.copyfile(ROOT / ".github" / name, self.root / ".github" / name)
        shutil.copyfile(ROOT / "docs/src/threat-models.md", self.root / "docs/src/threat-models.md")
        manifest = json.loads(json.dumps(MANIFEST))
        for i, row in enumerate(manifest["cases"]):
            row["source"] = f"evidence/{i}.rs"
            (self.root / row["source"]).write_text(f"// {row['marker']}\n")
        (self.root / ".github/threat-model-release-minimum.json").write_text(json.dumps(manifest))
        cargo = self.root / "bin/cargo"
        cargo.write_text(FAKE_CARGO)
        cargo.chmod(0o700)
        self.calls = self.root / "calls.jsonl"
        self.env = os.environ | {"PATH": f"{self.root / 'bin'}{os.pathsep}{os.environ['PATH']}",
                                 "RULLST_CARGO_CALLS": str(self.calls)}

    def run_script(self, *args, mode="pass"):
        return subprocess.run(["bash", ".github/check-threat-model-release-minimum.sh", *args],
                              cwd=self.root, env=self.env | {"RULLST_CARGO_TEST_MODE": mode},
                              text=True, capture_output=True, timeout=90)

    def recorded(self):
        return [json.loads(line) for line in self.calls.read_text().splitlines()] if self.calls.exists() else []

    def test_complete_inventory_uses_one_cargo_test_per_unique_case(self):
        result = self.run_script()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        calls = self.recorded()
        expected = list(dict.fromkeys(key(row) for row in MANIFEST["cases"]))
        self.assertEqual(calls[0], ["fetch", "--locked"])
        self.assertEqual(len(calls[1:]), len(expected))
        self.assertEqual(len(expected), 59)
        for call, test_key in zip(calls[1:], expected):
            crate, kind, target, test = test_key.split(":", 3)
            target_args = ["--lib"] if kind == "lib" else ["--test", target]
            self.assertEqual(call, ["test", "-p", crate, "--all-features", *target_args,
                                    test, "--", "--exact", "--color", "never"])

    def test_hash_shards_partition_the_same_inventory_without_duplicates(self):
        actual = []
        unique = list(dict.fromkeys(key(row) for row in MANIFEST["cases"]))
        for index in range(4):
            if self.calls.exists():
                self.calls.unlink()
            result = self.run_script("--shard", f"{index}/4")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            expected = [name for name in unique if int(hashlib.sha256(name.encode()).hexdigest()[:8], 16) % 4 == index]
            calls = self.recorded()[1:]
            self.assertEqual([call[call.index("--") - 1] for call in calls], [name.split(":", 3)[3] for name in expected])
            actual.extend(expected)
        self.assertEqual(sorted(actual), sorted(unique))

    def test_invalid_last_evidence_row_stops_before_any_cargo_command(self):
        (self.root / f"evidence/{len(MANIFEST['cases']) - 1}.rs").write_text("// missing marker\n")
        result = self.run_script()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.recorded(), [])

    def test_nonzero_cargo_status_missing_and_ignored_tests_cannot_pass(self):
        for mode in ("failed_exit", "missing", "ignored"):
            result = self.run_script(mode=mode)
            self.assertNotEqual(result.returncode, 0, mode)

    def test_bad_shard_never_executes_cargo(self):
        for shard in ("4/4", "0/0", "-1/4", "x/4"):
            self.assertNotEqual(self.run_script("--shard", shard).returncode, 0)
        self.assertEqual(self.recorded(), [])


if __name__ == "__main__":
    unittest.main()
