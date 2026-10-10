#!/usr/bin/env python3
"""The crate reproducibility check must compare two independent packagings."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / ".github/check-package-reproducibility.sh"
FAKE_CARGO = """#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
args = sys.argv[1:]
if args[0] == "metadata":
    names = ["rullst-macros", "rullst-orm-macros"]
    print(json.dumps({"packages": [{"name": n, "version": "1.2.3"} for n in names]}))
    sys.exit(0)
umask = os.umask(0)
os.umask(umask)
target = Path(os.environ["CARGO_TARGET_DIR"])
record = {"args": args, "cwd": os.getcwd(), "umask": umask, "target": str(target),
          "target_existed": target.exists()}
with open(os.environ["PACKAGE_RECEIPT"], "a") as receipt:
    receipt.write(json.dumps(record) + "\\n")
(target / "package").mkdir(parents=True)
crates = [args[i + 1] for i, value in enumerate(args) if value == "--package"]
second = Path(os.getcwd()).resolve() != Path(os.environ["REPOSITORY_ROOT"]).resolve()
for crate in crates:
    body = b"archive:" + crate.encode()
    if second and os.environ.get("TAMPER_CRATE") == crate:
        body += b"!"
    (target / "package" / f"{crate}-1.2.3.crate").write_bytes(body)
"""


class PackageReproducibilityTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.receipt = self.root / "receipt.jsonl"
        self.cargo = self.root / "cargo-probe"
        self.cargo.write_text(FAKE_CARGO)
        self.cargo.chmod(0o700)
        self.env = dict(os.environ, CARGO=str(self.cargo), PACKAGE_RECEIPT=str(self.receipt),
                        REPOSITORY_ROOT=str(ROOT))

    def run_check(self, *args, **env):
        return subprocess.run(["bash", str(SCRIPT), *args], env={**self.env, **env},
                              capture_output=True, text=True, check=False)

    def records(self):
        return [json.loads(line) for line in self.receipt.read_text().splitlines()]

    def test_identical_archives_pass_from_independent_checkouts(self):
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.count("reproducible"), 2)
        first, second = self.records()
        for record in (first, second):
            self.assertFalse(record["target_existed"])
            for flag in ("--all-features", "--locked", "--no-verify"):
                self.assertIn(flag, record["args"])
            self.assertEqual(record["args"].count("--package"), 2)
        self.assertEqual(Path(first["cwd"]).resolve(), ROOT.resolve())
        self.assertNotEqual(Path(second["cwd"]).resolve(), ROOT.resolve())
        self.assertNotEqual(first["target"], second["target"])
        self.assertEqual(second["umask"], 0o077)
        self.assertFalse(Path(second["cwd"]).exists())
        worktrees = subprocess.run(["git", "-C", str(ROOT), "worktree", "list"],
                                   capture_output=True, text=True, check=True).stdout
        self.assertNotIn(second["cwd"], worktrees)

    def test_a_different_archive_fails(self):
        result = self.run_check("rullst-macros", "rullst-orm-macros", TAMPER_CRATE="rullst-orm-macros")
        self.assertEqual(result.returncode, 1)
        self.assertIn("reproducible", result.stdout)
        self.assertIn("DIFFERENT", result.stderr)
        self.assertIn("rullst-orm-macros-1.2.3.crate", result.stderr)

    def test_unadmitted_or_malformed_crates_never_start_cargo(self):
        for crate in ["rullst-blog-example", "Bad", "$(false)", "--workspace", ""]:
            with self.subTest(crate=crate):
                self.assertEqual(self.run_check(crate).returncode, 2)
                self.assertFalse(self.receipt.exists())

    def test_ci_runs_the_check(self):
        ci = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertIn("bash .github/check-package-reproducibility.sh rullst-macros rullst-orm-macros", ci)


if __name__ == "__main__":
    unittest.main()
