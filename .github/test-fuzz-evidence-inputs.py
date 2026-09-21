#!/usr/bin/env python3
"""Exercise input identity against real isolated Git histories, not path mocks."""

import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from fuzz_evidence_inputs import ROOT, Snapshot, execution_contract, reviewed_document_blobs


class InputTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="rullst-fuzz-inputs-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.run_git("init", "-q")
        self.run_git("config", "user.email", "fixture@example.invalid")
        self.run_git("config", "user.name", "Fixture")
        self.v13_inventory = json.loads((ROOT / ".github/fuzz-targets.json").read_text()) + [
            {"dir": "rullst-privacy/fuzz", "target": target}
            for target in ("fuzz_age_challenge_token", "fuzz_age_attestation")
        ]
        self.inventory = [item for item in self.v13_inventory if item["dir"] != "rullst-privacy/fuzz"]
        self.write(".github/fuzz-targets.json", json.dumps(self.inventory))
        self.write(".github/release-required-workflows.json",
                   '{"schema_version":2,"required_branch":"main"}')
        self.write(".github/workflows/fuzzing.yml", (ROOT / ".github/workflows/fuzzing.yml").read_text())
        self.write("rullst-mail/Cargo.toml", '[package]\nname="mail"\nversion="1.0.0"\n')
        self.write("rullst-core/src/lib.rs", "pub fn shared() {}\n")
        self.write("Cargo.toml", "[workspace]\n")
        self.write(".cargo/config.toml", "[build]\njobs=2\n")
        self.write("Cargo.lock", "root lock")
        for item in self.inventory:
            directory = item["dir"]
            self.write(f"{directory}/Cargo.toml", '[package]\nname="fixture"\n[dependencies]\nparent={path=".."}\n')
            self.write(f"{directory}/Cargo.lock", "locked graph")
            self.write(f'{directory}/fuzz_targets/{item["target"]}.rs', "fn main() {}\n")
        self.base = Snapshot(self.commit(), self.root)

    def run_git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, stderr=subprocess.PIPE).decode().strip()

    def write(self, path, text):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)

    def commit(self):
        self.run_git("add", ".")
        self.run_git("commit", "-qm", "test(fuzz): change fixture")
        return self.run_git("rev-parse", "HEAD")

    def changed(self):
        current = Snapshot(self.commit(), self.root)
        return {directory for directory in self.base.directories
                if self.base.fingerprint(directory) != current.fingerprint(directory)}

    def test_release_line_is_read_from_the_selected_git_object(self):
        self.write(".github/release-required-workflows.json",
                   '{"schema_version":3,"required_major":13,"required_branch":"v13"}')
        for item in self.v13_inventory:
            if item not in self.inventory:
                directory = item["dir"]
                self.write(f"{directory}/Cargo.toml", '[package]\nname="privacy-fixture"\n[dependencies]\nparent={path=".."}\n')
                self.write(f"{directory}/Cargo.lock", "locked graph")
                self.write(f'{directory}/fuzz_targets/{item["target"]}.rs', "fn main() {}\n")
        self.write(".github/fuzz-targets.json", json.dumps(self.v13_inventory))
        current = Snapshot(self.commit(), self.root)
        self.assertEqual(len(current.inventory), 42)
        self.assertEqual(len(Snapshot(self.base.sha, self.root).inventory), 40)
        self.assertEqual(current.release_branch, "v13")
        self.assertEqual(Snapshot(self.base.sha, self.root).release_branch, "main")
        self.assertNotEqual(current.global_hash, self.base.global_hash)

    def test_reviewed_fixture_and_dev_dependency_changes_preserve_fuzz_inputs(self):
        for path in ("rullst-mail/tests/feedback.rs", ".github/mobile-ui-browser-smoke.mjs",
                     ".github/billing-csp-browser-smoke.mjs", "Cargo.lock", "WORKFLOWS.md"):
            self.write(path, "changed test or non-input")
        with (self.root / "rullst-mail/Cargo.toml").open("a") as output:
            output.write('[dev-dependencies]\nrand={workspace=true}\n')
        self.assertEqual(self.changed(), set())

    def document_review_fixture(self):
        # Exercise every reviewed path against real blobs in the fixture's Git
        # history. Stub only the trusted review table, not source discovery.
        review = {}
        for path in reviewed_document_blobs():
            self.write(path, "original documentation\n")
            before = self.run_git("hash-object", path)
            self.write(path, "reviewed publication documentation\n")
            after = self.run_git("hash-object", path)
            self.write(path, "original documentation\n")
            review[path] = [before, after]
        policy = patch("fuzz_evidence_inputs.reviewed_document_blobs", return_value=review)
        policy.start()
        self.addCleanup(policy.stop)
        self.base = Snapshot(self.commit(), self.root)
        return review

    def test_exact_reviewed_document_contents_preserve_all_package_inputs(self):
        for path in self.document_review_fixture():
            self.write(path, "reviewed publication documentation\n")
        self.assertEqual(self.changed(), set())

    def test_third_document_blob_new_path_mode_and_deletion_invalidate_all(self):
        review = self.document_review_fixture()
        for path in review:
            with self.subTest(path=path):
                self.run_git("reset", "--hard", self.base.sha)
                self.write(path, "unreviewed later document contents\n")
                self.assertEqual(self.changed(), self.base.directories)
        for operation in ("delete", "executable", "new-path"):
            with self.subTest(operation=operation):
                self.run_git("reset", "--hard", self.base.sha)
                target = self.root / "README.md"
                if operation == "delete":
                    target.unlink()
                elif operation == "executable":
                    target.chmod(0o755)
                else:
                    self.write("docs/src/unreviewed.md", "reviewed publication documentation\n")
                self.assertEqual(self.changed(), self.base.directories)

    def test_new_runtime_document_consumer_invalidates_reviewed_equivalence(self):
        self.document_review_fixture()
        self.write("rullst-core/src/lib.rs", 'pub const TEXT: &str = include_str!("../../README.md");')
        self.write("README.md", "reviewed publication documentation\n")
        self.assertEqual(self.changed(), self.base.directories)

    def test_every_package_harness_and_lock_invalidates_its_entire_package(self):
        for directory in self.base.directories:
            for suffix in ("Cargo.lock", "extra-fixture.txt"):
                with self.subTest(directory=directory, suffix=suffix):
                    self.run_git("reset", "--hard", self.base.sha)
                    self.write(f"{directory}/{suffix}", "changed")
                    self.assertEqual(self.changed(), {directory})
        self.run_git("reset", "--hard", self.base.sha)
        item = self.inventory[0]
        self.write(f'{item["dir"]}/fuzz_targets/{item["target"]}.rs', "fn main() { let _ = 42; }")
        self.assertEqual(self.changed(), {item["dir"]})

    def test_runtime_toolchain_unknown_files_and_configuration_invalidate_all(self):
        for path in ("rullst-core/src/lib.rs", "new-unclassified-input", "rust-toolchain.toml",
                     ".cargo/config.toml", "Cargo.toml", "rullst-mail/build.rs",
                     "rullst-mail/tests/unreviewed.rs", ".github/validate-fuzz-targets.py"):
            with self.subTest(path=path):
                self.run_git("reset", "--hard", self.base.sha)
                self.write(path, "changed")
                self.assertEqual(self.changed(), self.base.directories)

    def test_normal_and_build_dependencies_are_not_ignored(self):
        for section in ("dependencies", "build-dependencies", 'target."cfg(unix)".dependencies'):
            self.run_git("reset", "--hard", self.base.sha)
            with (self.root / "rullst-mail/Cargo.toml").open("a") as output:
                output.write(f'[{section}]\nnew_dep="1"\n')
            self.assertEqual(self.changed(), self.base.directories)

    def test_deleted_input_and_executable_bit_are_not_ignored(self):
        (self.root / "rullst-core/src/lib.rs").unlink()
        self.assertEqual(self.changed(), self.base.directories)
        self.run_git("reset", "--hard", self.base.sha)
        (self.root / "rullst-core/src/lib.rs").chmod(0o755)
        self.assertEqual(self.changed(), self.base.directories)

    def test_unproven_package_isolation_falls_back_to_global_inputs(self):
        item = self.inventory[0]
        path = f'{item["dir"]}/fuzz_targets/{item["target"]}.rs'
        self.write(path, 'const DATA: &str = include_str!("../../other/fuzz/fixture");')
        self.assertEqual(self.changed(), self.base.directories)
        # Later changes in a different package still invalidate the consumer.
        self.base = Snapshot(self.run_git("rev-parse", "HEAD"), self.root)
        other = next(directory for directory in self.base.directories if directory != item["dir"])
        self.write(f"{other}/fixture", "changed again")
        self.assertEqual(self.changed(), self.base.directories)

    def test_shared_execution_contract_includes_budget_flags_tools_and_runner(self):
        path = ".github/workflows/fuzzing.yml"
        original = (self.root / path).read_text()
        for old, new in (("nightly-2026-08-21", "nightly-2026-08-22"),
                         ("-max_len=2048", "-max_len=1024"),
                         ("ubuntu-latest", "ubuntu-24.04"),
                         ("tool: cargo-fuzz@0.13.2", "tool: cargo-fuzz@0.13.3"),
                         ('-max_total_time="$CAMPAIGN_SECONDS"', '-max_total_time=1')):
            with self.subTest(change=old):
                self.assertNotEqual(execution_contract(original), execution_contract(original.replace(old, new)))
        legacy = original.replace("    if: needs.targets.outputs.selected_count != '0'\n", "")
        self.assertEqual(execution_contract(original), execution_contract(legacy))
        with_defaults = original.replace("\njobs:\n", "\ndefaults:\n  run:\n    shell: sh\n\njobs:\n")
        self.assertNotEqual(execution_contract(original), execution_contract(with_defaults))
        with_job = original + "\n  unknown:\n    runs-on: ubuntu-latest\n    steps: []\n"
        self.assertNotEqual(execution_contract(original), execution_contract(with_job))
        for unsupported in (original.replace("\njobs:\n", "\n'defaults': {}\n\njobs:\n"),
                            original.replace("  preflight:\n", "  'preflight':\n"),
                            original.replace("  workflow_dispatch:", "  workflow_dispatch: &hidden")):
            with self.assertRaises(ValueError):
                execution_contract(unsupported)

    def test_duplicate_target_or_unknown_inventory_shape_is_rejected(self):
        self.inventory[-1] = self.inventory[0]
        self.write(".github/fuzz-targets.json", json.dumps(self.inventory))
        with self.assertRaisesRegex(ValueError, "duplicate"):
            Snapshot(self.commit(), self.root)

    def test_v13_cannot_admit_the_legacy_forty_target_surface(self):
        self.write(".github/release-required-workflows.json",
                   '{"schema_version":3,"required_major":13,"required_branch":"v13"}')
        with self.assertRaisesRegex(ValueError, "exactly 42"):
            Snapshot(self.commit(), self.root)

    def test_v12_cannot_reinterpret_an_expanded_inventory(self):
        self.write(".github/fuzz-targets.json", json.dumps(self.v13_inventory))
        with self.assertRaisesRegex(ValueError, "exactly 40"):
            Snapshot(self.commit(), self.root)

    def test_symlink_and_missing_commit_fail_closed(self):
        (self.root / "outside").symlink_to("/tmp")
        with self.assertRaisesRegex(ValueError, "symlinks"):
            Snapshot(self.commit(), self.root)
        with self.assertRaises(ValueError):
            Snapshot("--help", self.root)


if __name__ == "__main__":
    unittest.main()
