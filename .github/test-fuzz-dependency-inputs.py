#!/usr/bin/env python3
"""Regression controls for dependency-scoped evidence, using real Git trees."""

import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from fuzz_evidence_inputs import ROOT, Snapshot
from fuzz_dependency_inputs import tree_digest


class DependencyInputTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="rullst-fuzz-dependencies-")
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.git("init", "-q")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Fixture")
        self.inventory = json.loads((ROOT / ".github/fuzz-targets.json").read_text())
        members = {item["dir"].split("/")[0] for item in self.inventory}
        members |= {"rullst-auth", "rullst-helper"}
        self.write("Cargo.toml", "[workspace]\nmembers=" + json.dumps(sorted(members)))
        self.write(".github/fuzz-targets.json", json.dumps(self.inventory))
        self.write(".github/release-required-workflows.json",
                   '{"schema_version":4,"required_major":12,"required_branch":"v12"}')
        self.write(".github/workflows/fuzzing.yml", (ROOT / ".github/workflows/fuzzing.yml").read_text())
        for directory in members:
            self.write(directory + "/Cargo.toml", '[package]\nname="' + directory + '"\nversion="1.0.0"\n')
            self.write(directory + "/src/lib.rs", "pub fn fixture() {}\n")
        self.append("rullst/Cargo.toml", '\n[dependencies]\nsigning={package="rullst-auth",path="../rullst-auth",optional=true}\n')
        self.append("rullst-nexus/Cargo.toml", '\n[target.\'cfg(unix)\'.build-dependencies]\nhelper={path="../rullst-helper"}\n')
        self.append("rullst-helper/Cargo.toml", '\n[dev-dependencies]\nsigning={path="../rullst-auth"}\n')
        for item in self.inventory:
            directory = item["dir"]
            self.write(directory + "/Cargo.toml", '[package]\nname="fixture"\n[dependencies]\nparent={path=".."}\n')
            self.write(directory + "/Cargo.lock", "retained lock\n")
            self.write(f'{directory}/fuzz_targets/{item["target"]}.rs', "fn main() {}\n")
        self.review = {
            "schema_version": 1, "procedural_macros": {}, "doctest_modules": {}, "cargo_configs": {},
            "scoped_packages": ["rullst-auth"], "source_contexts": {},
        }
        review = patch("fuzz_evidence_inputs.dependency_scope_review", return_value=self.review)
        review.start()
        self.addCleanup(review.stop)
        self.base = self.snapshot(approve=True)
        self.assertIsNotNone(self.base.dependency_scope)

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, stderr=subprocess.PIPE).decode().strip()

    def write(self, path, content):
        file = self.root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(content)

    def append(self, path, content):
        with (self.root / path).open("a") as output:
            output.write(content)

    def snapshot(self, approve=False):
        self.git("add", ".")
        self.git("commit", "-qm", "test(fuzz): update dependency fixture")
        sha = self.git("rev-parse", "HEAD")
        snapshot = Snapshot(sha, self.root)
        if approve:
            self.review["source_contexts"] = {
                directory: [digest] for directory, digest in snapshot.dependency_scope.contexts.items()
            }
            snapshot = Snapshot(sha, self.root)
        return snapshot

    def changes(self):
        current = self.snapshot()
        return {item["dir"] for item in current.inventory
                if current.fingerprint(item["dir"]) != self.base.fingerprint(item["dir"])}

    def reset(self):
        self.git("reset", "--hard", self.base.sha)

    def test_two_reviewed_macro_trees_do_not_admit_future_metadata_or_source(self):
        macro = "rullst-helper"
        self.append(macro + "/Cargo.toml", "\n[lib]\nproc-macro=true\n")
        self.write(macro + "/README.md", "original metadata\n")
        first = self.snapshot()
        self.assertIsNone(first.dependency_scope)
        first_tree = tree_digest(first.files, macro)
        self.review["procedural_macros"][macro] = first_tree
        self.assertIsNotNone(Snapshot(first.sha, self.root).dependency_scope)
        self.write(macro + "/README.md", "reviewed metadata\n")
        second = self.snapshot()
        self.assertIsNone(second.dependency_scope)
        second_tree = tree_digest(second.files, macro)
        self.review["procedural_macros"][macro] = [first_tree, second_tree]
        for sha in (first.sha, second.sha):
            self.assertIsNotNone(Snapshot(sha, self.root).dependency_scope)
        for path in (macro + "/README.md", macro + "/src/lib.rs"):
            self.git("reset", "--hard", second.sha)
            self.write(path, "unreviewed next contents\n")
            self.assertIsNone(self.snapshot().dependency_scope)
        for invalid in ([], [first_tree, first_tree], [first_tree, second_tree, "a" * 64], ["invalid"], 42):
            self.review["procedural_macros"][macro] = invalid
            with self.assertRaises(ValueError):
                Snapshot(first.sha, self.root)

    def test_auth_changes_reach_direct_optional_and_transitive_target_build_consumers(self):
        self.write("rullst-auth/src/lib.rs", "pub fn corrected_auth() {}\n")
        self.assertEqual(self.changes(), {"rullst/fuzz", "rullst-nexus/fuzz"})

    def test_new_deleted_mode_changed_and_auxiliary_auth_inputs_follow_the_same_closure(self):
        for action in ("new", "delete", "mode", "fixture"):
            with self.subTest(action=action):
                self.reset()
                if action == "new":
                    self.write("rullst-auth/src/new.rs", "new auth code")
                elif action == "delete":
                    (self.root / "rullst-auth/src/lib.rs").unlink()
                elif action == "mode":
                    (self.root / "rullst-auth/src/lib.rs").chmod(0o755)
                else:
                    self.write("rullst-auth/tests/session.rs", "new contract")
                self.assertEqual(self.changes(), {"rullst/fuzz", "rullst-nexus/fuzz"})

    def test_workspace_inheritance_and_renames_are_resolved(self):
        self.append("Cargo.toml", '\n[workspace.dependencies]\nidentity={package="rullst-auth",path="rullst-auth"}\n')
        self.append("rullst-mail/Cargo.toml", '\n[dependencies]\nidentity={workspace=true}\n')
        self.base = self.snapshot(approve=True)
        self.write("rullst-auth/src/lib.rs", "changed")
        self.assertEqual(self.changes(), {"rullst/fuzz", "rullst-nexus/fuzz", "rullst-mail/fuzz"})

    def test_cross_package_literal_inclusion_is_a_dependency_even_without_manifest_edge(self):
        self.write("rullst-iot/src/lib.rs", 'pub const CODE: &str=include_str!("../../rullst-auth/src/lib.rs");')
        self.base = self.snapshot(approve=True)
        self.write("rullst-auth/src/lib.rs", "changed")
        self.assertEqual(self.changes(), {"rullst/fuzz", "rullst-nexus/fuzz", "rullst-iot/fuzz"})

    def test_shared_manifest_toolchain_execution_and_unknown_inputs_still_invalidate_all(self):
        for path in ("Cargo.toml", "rullst-auth/Cargo.toml", "rust-toolchain.toml",
                     ".cargo/config.toml", "unclassified-file"):
            with self.subTest(path=path):
                self.reset()
                if path.endswith("Cargo.toml"):
                    self.append(path, "\n# dependency contract changed\n")
                else:
                    self.write(path, "changed")
                self.assertEqual(self.changes(), self.base.directories)

    def test_harness_and_lock_changes_stay_with_their_package(self):
        for path in ("rullst-mail/fuzz/Cargo.lock", "rullst-mail/fuzz/fuzz_targets/fuzz_mail.rs"):
            with self.subTest(path=path):
                self.reset()
                self.write(path, "changed")
                self.assertEqual(self.changes(), {"rullst-mail/fuzz"})

    def test_unknown_source_consumers_never_enable_narrow_reuse(self):
        for path, value in (
            ("rullst-mail/build.rs", 'fn main() {}'),
            ("rullst-mail/src/lib.rs", 'const S: &str=include_str!(concat!("../../", "rullst-auth/src/lib.rs"));'),
            ("rullst-mail/src/lib.rs", '#[path="../../rullst-auth/src/lib.rs"] mod other;'),
            ("rullst-mail/src/lib.rs", '#[cfg_attr(unix, path="../../rullst-auth/src/lib.rs")] mod other;'),
            ("rullst-mail/src/lib.rs", 'const S: &str=include_str!("/outside/file");'),
        ):
            with self.subTest(path=path, value=value):
                self.reset()
                self.write(path, value)
                self.base_unknown = self.snapshot()
                self.assertIsNone(self.base_unknown.dependency_scope)
                self.write("rullst-auth/src/lib.rs", "changed auth after unknown consumer was added")
                current = self.snapshot()
                self.assertEqual({d for d in current.directories
                                  if current.fingerprint(d) != self.base_unknown.fingerprint(d)},
                                 current.directories)

    def test_patches_external_dependencies_and_new_procedural_macros_fail_closed(self):
        for path, addition in (
            ("Cargo.toml", '\n[patch.crates-io]\nfoo={path="rullst-auth"}\n'),
            ("rullst-mail/Cargo.toml", '\n[dependencies]\nexternal={path="../../outside"}\n'),
            ("rullst-mail/Cargo.toml", '\n[lib]\nproc-macro=true\n'),
            ("rullst-mail/Cargo.toml", '\n[lib]\npath="../rullst-auth/src/lib.rs"\n'),
        ):
            with self.subTest(addition=addition):
                self.reset()
                self.append(path, addition)
                current = self.snapshot()
                self.assertIsNone(current.dependency_scope)
                self.assertTrue(all(current.fingerprint(d) != self.base.fingerprint(d)
                                    for d in current.directories))

    def test_unreviewed_runtime_reader_remains_affected_after_a_fresh_campaign(self):
        self.write("rullst-iot/src/lib.rs",
                   'pub fn read_source() { let _ = std::fs::read("../rullst-auth/src/lib.rs"); }')
        unknown_consumer = self.snapshot()
        self.assertIn("rullst-iot/fuzz", unknown_consumer.dependency_scope.unproven_consumers)
        self.write("rullst-auth/src/lib.rs", "changed after the reader's campaign passed")
        current = self.snapshot()
        affected = {d for d in current.directories
                    if current.fingerprint(d) != unknown_consumer.fingerprint(d)}
        self.assertEqual(affected, {"rullst/fuzz", "rullst-nexus/fuzz", "rullst-iot/fuzz"})


if __name__ == "__main__":
    unittest.main()
