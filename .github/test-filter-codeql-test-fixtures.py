#!/usr/bin/env python3
"""Lock down which CodeQL results the test-fixture filter may drop."""

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
SCRIPT = HERE / "filter-codeql-test-fixtures.py"
SPEC = importlib.util.spec_from_file_location("filter_codeql", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot import the CodeQL filter")
FILTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(FILTER)

CRYPTO = "rust/hard-coded-cryptographic-value"


def result(rule, uri):
    return {
        "ruleId": rule,
        "locations": [{"physicalLocation": {"artifactLocation": {"uri": uri}}}],
    }


class FilterTests(unittest.TestCase):
    def test_test_only_paths_are_dropped(self):
        for uri in (
            "rullst-auth/src/recovery/crypto_tests.rs",
            "rullst-auth/src/recovery/sessions/tests.rs",
            "rullst-core/src/foo/test.rs",
            "rullst-core/src/foo_test.rs",
            "rullst-security/tests/process_wide_state.rs",
            "rullst-privacy/tests/age_assurance/replay.rs",
            "tests/top_level.rs",
            "rullst-auth/benches/auth_bench.rs",
            "rullst/fuzz/src/lib.rs",
        ):
            with self.subTest(uri=uri):
                self.assertTrue(FILTER.is_test_fixture(result(CRYPTO, uri)))

    def test_production_paths_are_kept(self):
        for uri in (
            "rullst-security/src/recovery_codes.rs",
            "rullst-auth/src/auth/session.rs",
            "rullst-core/src/testing.rs",
            "rullst-core/src/contests.rs",
            "rullst-core/src/latest.rs",
            "rullst-core/src/attestation.rs",
            "examples/saas/src/main.rs",
            "cargo-rullst/src/generators/auth/auth_controller.rs.template",
            "rullst-core/src/tests_support/keys.rs",
            "cargo-rullst/src/blueprints/security_tests.rs",
            "rullst-orm-macros/src/sandbox_test.rs",
        ):
            with self.subTest(uri=uri):
                self.assertFalse(FILTER.is_test_fixture(result(CRYPTO, uri)))

    def test_production_exceptions_still_exist(self):
        root = HERE.parent
        for path in FILTER.PRODUCTION_FILES:
            with self.subTest(path=path):
                self.assertTrue((root / path).is_file(), f"{path} was moved; update PRODUCTION_FILES")

    def test_logged_fixtures_are_dropped_only_in_tests(self):
        logging = "rust/cleartext-logging"
        self.assertTrue(FILTER.is_test_fixture(result(logging, "rullst-core/src/htmx_tests.rs")))
        self.assertTrue(FILTER.is_test_fixture(result(logging, "rullst-auth/tests/a.rs")))
        self.assertFalse(FILTER.is_test_fixture(result(logging, "rullst-core/src/htmx.rs")))

    def test_other_rules_are_kept_even_in_tests(self):
        self.assertFalse(
            FILTER.is_test_fixture(result("rust/sql-injection", "rullst-auth/tests/a.rs"))
        )

    def test_results_without_a_location_are_kept(self):
        self.assertFalse(FILTER.is_test_fixture({"ruleId": CRYPTO, "locations": []}))
        self.assertFalse(FILTER.is_test_fixture({"ruleId": CRYPTO}))

    def test_cli_rewrites_the_file_and_reports_counts(self):
        sarif = {
            "runs": [
                {
                    "results": [
                        result(CRYPTO, "rullst-auth/src/recovery/crypto_tests.rs"),
                        result(CRYPTO, "rullst-auth/src/recovery/crypto_tests.rs"),
                        result(CRYPTO, "rullst-security/src/recovery_codes.rs"),
                        result("rust/xss", "rullst-auth/tests/a.rs"),
                    ]
                },
                {"results": []},
                {},
            ]
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "rust.sarif"
            path.write_text(json.dumps(sarif), encoding="utf-8")
            output = subprocess.run(
                [sys.executable, "-I", str(SCRIPT), str(path)],
                check=True,
                capture_output=True,
                text=True,
            ).stdout
            filtered = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual(
            [r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
             for r in filtered["runs"][0]["results"]],
            ["rullst-security/src/recovery_codes.rs", "rullst-auth/tests/a.rs"],
        )
        self.assertEqual(filtered["runs"][1]["results"], [])
        self.assertIn("removed 2 test-fixture result(s) (rust/cleartext-logging, "
                      "rust/hard-coded-cryptographic-value) in test-only files", output)
        self.assertEqual(output.count("crypto_tests.rs"), 1)

    def test_cli_requires_one_argument(self):
        completed = subprocess.run(
            [sys.executable, "-I", str(SCRIPT)], capture_output=True, text=True
        )
        self.assertEqual(completed.returncode, 2)
        self.assertIn("usage:", completed.stderr)


if __name__ == "__main__":
    unittest.main()
