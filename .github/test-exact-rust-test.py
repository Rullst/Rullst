#!/usr/bin/env python3
"""Parser negatives and a real offline Rust harness for exact-test evidence."""

import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("exact", ROOT / ".github/assert-exact-rust-test.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def passed(name="nested::passes"):
    return (f"running 1 test\ntest {name} ... ok\n\n"
            "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s\n")


class ParserTests(unittest.TestCase):
    def test_requires_the_exact_name_and_supports_expected_panics(self):
        MODULE.validate(passed(), "nested::passes")
        MODULE.validate(passed().replace(" ... ok", " - should panic ... ok"), "nested::passes")
        for name in ("passes", "nested", "nested::passes_extra", "--ignored", "bad\nname"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                MODULE.validate(passed(), name)

    def test_rejects_missing_ignored_failed_multiple_or_incomplete_executions(self):
        variants = ["", "nested::passes: test\n", passed() * 2,
            passed().replace("1 passed", "0 passed"),
            passed().replace("0 ignored", "1 ignored"),
            passed().replace("0 failed", "1 failed"),
            passed().replace("0 measured", "1 measured"),
            passed().replace("running 1 test", "running 0 tests"),
            passed().replace("running 1 test", "running 2 tests"),
            passed().replace("test result: ok", "test result: FAILED"),
            passed().split("test result:")[0],
            passed().replace("nested::passes", "other::passes")]
        for output in variants:
            with self.subTest(output=output), self.assertRaises(ValueError):
                MODULE.validate(output, "nested::passes")

    def test_cli_rejects_oversized_or_invalid_utf8_logs(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "output.log"
            for content in (b"\xff", b"x" * (MODULE.MAX_LOG_BYTES + 1)):
                log.write_bytes(content)
                result = subprocess.run(["python3", str(ROOT / ".github/assert-exact-rust-test.py"), str(log), "passes"], capture_output=True)
                self.assertNotEqual(result.returncode, 0)


class RealRustTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if shutil.which("rustc") is None:
            raise RuntimeError("rustc is required: do not silently skip real libtest evidence")
        cls.directory = tempfile.TemporaryDirectory(prefix="rullst-exact-test-")
        cls.addClassCleanup(cls.directory.cleanup)
        root = Path(cls.directory.name)
        source = root / "lib.rs"
        source.write_text('''
mod nested {
    #[test] fn passes() { assert_eq!(2 + 2, 4); }
    #[test] #[ignore] fn ignored() { panic!("must not count as passed"); }
    #[test] fn fails() { assert_eq!(1, 2); }
    #[test] #[should_panic] fn expected_panic() { panic!("expected"); }
}
''')
        cls.binary = root / ("tests.exe" if os.name == "nt" else "tests")
        subprocess.run(["rustc", "--test", str(source), "-o", str(cls.binary)], check=True, capture_output=True, timeout=60)

    def test_real_pass_and_expected_panic_are_accepted(self):
        for name in ("nested::passes", "nested::expected_panic"):
            result = subprocess.run([str(self.binary), name, "--exact", "--color", "never"], text=True, capture_output=True, timeout=15)
            self.assertEqual(result.returncode, 0)
            MODULE.validate(result.stdout, name)

    def test_real_ignored_and_missing_filters_return_zero_but_are_rejected(self):
        for name in ("nested::ignored", "nested::missing"):
            result = subprocess.run([str(self.binary), name, "--exact", "--color", "never"], text=True, capture_output=True, timeout=15)
            self.assertEqual(result.returncode, 0, "the fixture must reproduce libtest's zero-execution success")
            with self.assertRaises(ValueError):
                MODULE.validate(result.stdout, name)

    def test_real_failure_does_not_become_evidence(self):
        result = subprocess.run([str(self.binary), "nested::fails", "--exact", "--color", "never"], text=True, capture_output=True, timeout=15)
        self.assertNotEqual(result.returncode, 0)
        with self.assertRaises(ValueError):
            MODULE.validate(result.stdout, "nested::fails")


if __name__ == "__main__":
    unittest.main()
