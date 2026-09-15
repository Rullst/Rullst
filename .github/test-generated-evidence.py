#!/usr/bin/env python3
"""Run actual generated-case selection with a recorded, non-building verifier.

This proves scheduling/inventory only, not generated application correctness.
Hosted Rust CI still materializes and tests the eight real applications; the
threat minimum separately executes the same LMS case and verification helper.
"""

import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "cargo-rullst/tests/generated_saas_check.rs"
FOCUSED = "materialized_lms_executes_security_contracts"
EXPECTED = [
    "blank-minimal|0|false|false|false|Active Record|Zero-Bundle HTMX|false",
    "blank-api-hot|0|true|true|false|Active Record|Zero-Bundle HTMX|false",
    "lms-active-htmx-hot|1|false|true|true|Active Record|Zero-Bundle HTMX|false",
    "saas-active-htmx-hot|2|false|true|true|Active Record|Zero-Bundle HTMX|false",
    "blog-active-htmx-hot|3|false|true|true|Active Record|Zero-Bundle HTMX|false",
    "portfolio-active-htmx-hot|4|false|true|true|Active Record|Zero-Bundle HTMX|false",
    "erp-active-htmx-release|5|false|false|true|Active Record|Zero-Bundle HTMX|true",
    "erp-active-htmx-hot|5|false|true|true|Active Record|Zero-Bundle HTMX|false",
]

RECORDER = r'''
use std::{fs, path::Path, io::Write};
mod rand {
    pub fn random<T: From<u64>>() -> T {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        T::from(((std::process::id() as u64) << 32)
            | NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}
fn materialize(_: GeneratedCase, project: &Path, _: &Path) {
    fs::create_dir_all(project).unwrap();
}
fn cargo_verify(case: GeneratedCase, project: &Path, _: &Path) {
    assert!(project.is_dir());
    let mut log = fs::OpenOptions::new().create(true).append(true)
        .open(std::env::var_os("RULLST_CASE_LOG").unwrap()).unwrap();
    writeln!(log, "{}|{}|{}|{}|{}|{}|{}|{}", case.name, case.blueprint, case.api, case.hot_reload,
        case.db_needed, case.orm_pattern, case.frontend, case.release).unwrap();
}
// --exact --skip must not suppress a future prefix-sharing assertion.
#[test] fn materialized_lms_executes_security_contracts_additional() {
    assert_eq!(2 + 2, 4);
}
'''

FAKE_CARGO = r'''#!/usr/bin/env python3
import os, subprocess, sys
args = sys.argv[1:]
if args == ["fetch", "--locked"]:
    raise SystemExit(0)
assert args[0] == "test" and "--all-features" in args and "--no-fail-fast" in args
assert args[args.index("--test") + 1] == "generated_saas_check"
assert "cargo-rullst" in args and "rullst-core" in args
assert ("--release" in args) == (os.environ.get("RULLST_EXPECT_RELEASE") == "1")
raise SystemExit(subprocess.run([os.environ["RULLST_SELECTION_BINARY"], *args[args.index("--") + 1:]]).returncode)
'''


class SelectionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory(prefix="rullst-generated-selection-")
        cls.addClassCleanup(cls.directory.cleanup)
        cls.root = Path(cls.directory.name)
        source = SOURCE.read_text()
        # Compile the actual inventory, environment selector, test entry points
        # and loop. Only materialization/Cargo are replaced with recorded calls.
        selection = source[source.index("#[derive(Clone, Copy, PartialEq, Eq)]"):
                           source.index("fn materialize(")]
        wrappers = source[source.index("#[test]\nfn every_blueprint_"):]
        ids = re.findall(r"pub const [A-Z]+_BLUEPRINT_ID: usize = \d+;",
                         (ROOT / "cargo-rullst/src/blueprints/mod.rs").read_text())
        if len(ids) != 6:
            raise AssertionError("review the public blueprint inventory")
        fixture = cls.root / "selection.rs"
        fixture.write_text("#![allow(dead_code)]\n" + "\n".join(ids)
                           + RECORDER + selection + wrappers)
        cls.binary = cls.root / "selection"
        subprocess.run(["rustc", "--edition=2024", "--test", str(fixture), "-o", str(cls.binary)],
                       env=os.environ | {"CARGO_MANIFEST_DIR": str(ROOT / "cargo-rullst")},
                       check=True, capture_output=True, timeout=60)
        (cls.root / ".github").mkdir()
        shutil.copyfile(ROOT / ".github/run-workspace-test-shard.sh",
                        cls.root / ".github/run-workspace-test-shard.sh")
        (cls.root / "bin").mkdir()
        cargo = cls.root / "bin/cargo"
        cargo.write_text(FAKE_CARGO)
        cargo.chmod(0o700)

    def run_selection(self, *args, group=None, shard=False, release=False):
        log = self.root / "cases.log"
        log.unlink(missing_ok=True)
        env = os.environ | {"RULLST_CASE_LOG": str(log),
                            "RULLST_SELECTION_BINARY": str(self.binary),
                            "RULLST_EXPECT_RELEASE": "1" if release else "0",
                            "PATH": f"{self.root / 'bin'}{os.pathsep}{os.environ['PATH']}"}
        env.pop("RULLST_CI_GENERATED_GROUP", None)
        if group is not None:
            env["RULLST_CI_GENERATED_GROUP"] = group
        command = ["bash", ".github/run-workspace-test-shard.sh"] if shard else [str(self.binary)]
        result = subprocess.run([*command, *args], cwd=self.root, env=env,
                                text=True, capture_output=True, timeout=15)
        return result, log.read_text().splitlines() if log.exists() else []

    def test_two_normal_shards_preserve_all_eight_cases_without_duplicate_lms(self):
        for release in (False, True):
            all_cases = []
            for group, expected in (("foundation", EXPECTED[:4]), ("product", EXPECTED[4:])):
                args = [f"cli-saas-{group}", *(["--release"] if release else [])]
                result, cases = self.run_selection(*args, shard=True, release=release)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(cases, expected)
                self.assertIn(f"test {FOCUSED}_additional ... ok", result.stdout)
                all_cases.extend(cases)
            self.assertEqual(all_cases, EXPECTED)

    def test_local_alias_preserves_complete_matrix(self):
        result, cases = self.run_selection("cli-saas", shard=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(cases, EXPECTED)

    def test_security_evidence_always_runs_only_lms_even_with_product_or_invalid_group(self):
        for group in (None, "foundation", "product", "invalid"):
            result, cases = self.run_selection(FOCUSED, "--exact", group=group)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(cases, [EXPECTED[2]])

    def test_bad_normal_group_cannot_succeed_without_cases(self):
        result, cases = self.run_selection(
            "every_blueprint_and_distinct_generated_boundary_passes_cargo_verification",
            "--exact", group="invalid")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(cases, [])

    def test_threat_manifest_maps_all_eight_lms_contracts_to_focused_execution(self):
        manifest = json.loads((ROOT / ".github/threat-model-release-minimum.json").read_text())
        rows = [row for row in manifest["cases"] if row["target"] == "generated_saas_check"]
        self.assertEqual(sorted(row["id"] for row in rows),
                         [f"ACADEMY-{n:02}" for n in (2, 3, 4, 5, 6, 7, 8, 10)])
        for row in rows:
            self.assertEqual(row["test_filter"], FOCUSED)
            self.assertEqual(row["source"], str(SOURCE.relative_to(ROOT)))
        source = SOURCE.read_text()
        self.assertNotIn("#[ignore", source)
        self.assertIn('command.arg("test").arg("--offline").arg("--all-targets")', source)
        self.assertIn('command.arg("--release")', source)


if __name__ == "__main__":
    unittest.main()
