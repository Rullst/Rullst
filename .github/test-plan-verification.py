#!/usr/bin/env python3
"""Negative fixtures and real Git histories for observation-only CI planning."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("plan-verification.py")
SPEC = importlib.util.spec_from_file_location("plan_verification", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot load verification planner")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PurePolicyTests(unittest.TestCase):
    def test_nul_paths_preserve_spaces_and_unicode(self) -> None:
        self.assertEqual(MODULE.parse_changes("M\0docs/a b é.md\0".encode()),
                         [("M", "docs/a b é.md")])

    def test_rename_tracks_both_paths(self) -> None:
        self.assertEqual(MODULE.parse_changes(b"R100\0docs/site.js\0source/file.js\0"),
                         [("R100", "docs/site.js"), ("R100", "source/file.js")])

    def test_malformed_input_never_becomes_empty_diff(self) -> None:
        for raw in (b"M\0", b"M\0x", b"R100\0x\0", b"M\0../x\0", b"Z\0x\0",
                    b"M\0/x\0", b"M\0-x\0", b"M\0x\ny\0", b"M\0a\\b\0", b"M\0\xff\0"):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                MODULE.parse_changes(raw)

    def test_cycles_and_diamonds_have_one_transitive_result(self) -> None:
        reverse = {"base": {"left", "right"}, "left": {"app"},
                   "right": {"app"}, "app": {"base"}, "other": set()}
        self.assertEqual(MODULE.reverse_closure({"base"}, reverse),
                         ["app", "base", "left", "right"])

    def test_policy_build_security_generator_unknown_and_docs_are_full(self) -> None:
        paths = {
            ".github/workflows/ci.yml": "verification-or-security-policy",
            ".github/harmless.md": "verification-or-security-policy",
            ".cargo/config.toml": "verification-or-security-policy",
            "AGENTS.md": "verification-or-security-policy",
            "SECURITY.md": "verification-or-security-policy",
            "docs/src/spec.md": "verification-or-security-policy",
            "Cargo.lock": "build-or-dependency-input",
            "crate/Cargo.toml": "build-or-dependency-input",
            "crate/build.rs": "build-or-dependency-input",
            "rust-toolchain.toml": "build-or-dependency-input",
            "docs/src/tutorials/test.md": "documentation-may-be-executable",
            "crate/tests/contract.rs": "shared-test-or-fuzz-input",
            "crate/fuzz/fuzz_targets/parser.rs": "shared-test-or-fuzz-input",
            "cargo-rullst/src/blueprints/blank.rs": "generated-application-contracts",
            "cargo-rullst/src/generators/file.rs.template": "generated-application-contracts",
            "rullst-auth/src/lib.rs": "critical-or-foundational-package",
            "some-new-file": "unknown-path",
        }
        owners = {"crate": "crate", "cargo-rullst": "cargo-rullst", "rullst-auth": "rullst-auth"}
        for path, reason in paths.items():
            with self.subTest(path=path):
                _, _, reasons = MODULE.classify([("M", path)], owners, {path: "100644"})
                self.assertIn(reason, reasons)

    def test_deleted_renamed_symlink_and_unknown_modes_force_full(self) -> None:
        path = "docs/site.js"
        for status, mode in (("D", "100644"), ("R100", "100644"), ("T", "120000"),
                             ("M", "120000"), ("A", "160000"), ("M", "missing")):
            with self.subTest(status=status, mode=mode):
                _, _, reasons = MODULE.classify([(status, path)], {}, {path: mode})
                self.assertTrue(reasons)

    def test_only_exact_site_files_are_presentation_candidates(self) -> None:
        for path in MODULE.SITE_FILES:
            _, direct, reasons = MODULE.classify([("M", path)], {}, {path: "100644"})
            self.assertFalse(direct)
            self.assertFalse(reasons)
        for path in ("docs/not-reviewed.js", "docs/site.js/child", "docs/src/site.js"):
            _, _, reasons = MODULE.classify([("M", path)], {}, {path: "100644"})
            self.assertTrue(reasons)


class GitPolicyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="rullst-verification-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.name", "Verification Fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        self.write("Cargo.toml", '''[workspace]
members = ["leaf", "bridge", "app", "other"]
resolver = "2"
[workspace.dependencies]
renamed = { package = "leaf", path = "leaf" }
''')
        for package in ("leaf", "bridge", "app", "other"):
            self.write(f"{package}/Cargo.toml", f'[package]\nname = "{package}"\nversion = "1.0.0"\n')
            self.write(f"{package}/src/lib.rs", "pub fn value() -> u8 { 1 }\n")
        self.append("bridge/Cargo.toml", '[dependencies]\nrenamed = { workspace = true, optional = true }\n')
        self.append("app/Cargo.toml", '''[target.'cfg(windows)'.build-dependencies]
bridge = { path = "../bridge" }
''')
        self.write("docs/site.css", "body { color: black; }\n")
        self.base = self.commit()

    def git(self, *args: str) -> str:
        result = subprocess.run(
            ["git", "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null",
             "-C", str(self.root), *args], capture_output=True, text=True, check=True,
        )
        return result.stdout.strip()

    def write(self, name: str, content: str) -> None:
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def append(self, name: str, content: str) -> None:
        path = self.root / name
        self.write(name, path.read_text(encoding="utf-8") + content)

    def commit(self) -> str:
        self.git("add", "--all")
        self.git("commit", "-qm", "test(policy): fixture")
        return self.git("rev-parse", "HEAD")

    def report(self) -> dict:
        report = MODULE.observe(self.root, self.base, "HEAD")
        self.assertFalse(report["may_skip_checks"])
        self.assertFalse(report["release_evidence_eligible"])
        self.assertEqual(report["enforced_scope"], "full")
        self.assertEqual(report["phase"], "observation-only")
        return report

    def test_optional_renamed_transitive_target_build_consumers(self) -> None:
        self.write("leaf/src/lib.rs", "pub fn value() -> u8 { 2 }\n")
        head = self.commit()
        report = self.report()
        self.assertEqual(report["candidate_scope"], "affected-packages")
        self.assertEqual(report["direct_packages"], ["leaf"])
        self.assertEqual(report["affected_packages"], ["app", "bridge", "leaf"])
        self.assertEqual(report["base_sha"], self.base)
        self.assertEqual(report["head_sha"], head)
        self.assertRegex(report["policy_sha256"], r"^[0-9a-f]{64}$")

    def test_dev_dependencies_are_not_omitted(self) -> None:
        self.append("other/Cargo.toml", '[dev-dependencies]\nleaf = { path = "../leaf" }\n')
        self.base = self.commit()
        self.write("leaf/src/lib.rs", "pub fn value() -> u8 { 3 }\n")
        self.commit()
        self.assertEqual(self.report()["affected_packages"], ["app", "bridge", "leaf", "other"])

    def test_site_candidate_does_not_authorize_skips(self) -> None:
        self.write("docs/site.css", "body { color: red; }\n")
        self.commit()
        report = self.report()
        self.assertEqual(report["candidate_scope"], "site-presentation")
        self.assertFalse(report["affected_packages"])

    def test_mixed_site_and_lock_changes_force_full(self) -> None:
        self.write("docs/site.css", "body { color: red; }\n")
        self.write("Cargo.lock", "version = 4\n")
        self.commit()
        report = self.report()
        self.assertEqual(report["candidate_scope"], "full")
        self.assertIn("build-or-dependency-input", report["fallback_reasons"])

    def test_new_markdown_with_executable_examples_is_full(self) -> None:
        self.write("docs/new.md", "```rust\nfn main() {}\n```\n")
        self.commit()
        self.assertIn("documentation-may-be-executable", self.report()["fallback_reasons"])

    def test_git_deletion_and_rename_are_full(self) -> None:
        self.git("mv", "docs/site.css", "docs/renamed.css")
        self.commit()
        report = self.report()
        self.assertEqual(report["candidate_scope"], "full")
        self.assertIn("deletion-rename-or-type-change", report["fallback_reasons"])

    def test_symlink_source_is_full(self) -> None:
        (self.root / "leaf/src/link.rs").symlink_to("lib.rs")
        self.commit()
        self.assertIn("non-regular-changed-file", self.report()["fallback_reasons"])

    def test_symlink_manifest_is_not_read(self) -> None:
        self.git("mv", "leaf/Cargo.toml", "leaf/actual.toml")
        (self.root / "leaf/Cargo.toml").symlink_to("actual.toml")
        self.commit()
        self.assertEqual(self.report()["fallback_reasons"], ["unavailable-or-unsupported-input"])

    def test_unsupported_glob_and_external_path_are_full(self) -> None:
        self.write("Cargo.toml", '[workspace]\nmembers = ["*"]\n')
        self.commit()
        self.assertEqual(self.report()["fallback_reasons"], ["unavailable-or-unsupported-input"])

    def test_unknown_path_dependency_is_full(self) -> None:
        self.append("other/Cargo.toml", '[dependencies]\nexternal = { path = "../../outside" }\n')
        self.commit()
        self.assertEqual(self.report()["fallback_reasons"], ["unavailable-or-unsupported-input"])

    def test_empty_diff_is_not_a_release_pass(self) -> None:
        self.assertEqual(self.report()["fallback_reasons"], ["empty-change-set"])

    def test_missing_ref_and_non_ancestor_fail_closed(self) -> None:
        missing = MODULE.observe(self.root, "f" * 40, "HEAD")
        self.assertEqual(missing["candidate_scope"], "full")
        self.assertFalse(missing["may_skip_checks"])
        self.git("checkout", "-q", "--orphan", "unrelated")
        self.git("commit", "--allow-empty", "-qm", "test(policy): unrelated")
        self.assertEqual(self.report()["fallback_reasons"], ["unavailable-or-unsupported-input"])

    def test_head_reads_committed_objects_not_dirty_files(self) -> None:
        self.write("leaf/src/lib.rs", "pub fn value() -> u8 { 4 }\n")
        self.commit()
        self.write("Cargo.toml", "not TOML at all")
        self.assertEqual(self.report()["candidate_scope"], "affected-packages")

    def test_revision_cannot_inject_git_options(self) -> None:
        report = MODULE.observe(self.root, "--help", "HEAD")
        self.assertEqual(report["fallback_reasons"], ["unavailable-or-unsupported-input"])

    def test_no_cargo_build_or_project_hook_is_executed(self) -> None:
        marker = self.root / "should-not-exist"
        hook = self.root / ".git/hooks/post-checkout"
        hook.write_text(f"#!/bin/sh\ntouch '{marker}'\n", encoding="utf-8")
        hook.chmod(0o755)
        self.write("other/build.rs", 'compile_error!("must not compile");\n')
        self.commit()
        self.assertEqual(self.report()["candidate_scope"], "full")
        self.assertFalse(marker.exists())

    def test_cli_reports_full_for_missing_history(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--repo", str(self.root), "--base", "missing"],
            capture_output=True, text=True, check=True,
        )
        report = json.loads(result.stdout)
        self.assertFalse(report["may_skip_checks"])
        self.assertEqual(report["candidate_scope"], "full")
        self.assertEqual(result.stderr, "")


if __name__ == "__main__":
    unittest.main()
