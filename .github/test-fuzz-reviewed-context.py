#!/usr/bin/env python3
"""Real Git histories prove the exact transition and reject changed consumers."""

import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

from fuzz_reviewed_context import (CONTROL_PATHS, DOCUMENT_PATHS, REVIEW, context_digest, document_paths,
                                  normalized_files, validate)

ROOT = Path(__file__).resolve().parents[1]


class ContextTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="rullst-context-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.git("init", "-q")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Fixture")
        self.contract = "reviewed execution"
        for path in DOCUMENT_PATHS - {"GOVERNANCE.md", "docs/src/openssf-scorecard.md"}:
            self.write(path, "old document " + path)
        for path in ("core/src/lib.rs", "macros/src/lib.rs", "core/fuzz/Cargo.lock",
                     "core/fuzz/fuzz_targets/parser.rs", "Cargo.toml", "Cargo.lock",
                     ".cargo/config.toml", "rust-toolchain.toml", "core/build.rs"):
            self.write(path, "reviewed source " + path)
        self.before_sha = self.commit()
        self.before = self.files()
        for path in DOCUMENT_PATHS:
            self.write(path, "reviewed document " + path)
        self.after_sha = self.commit()
        self.after = self.files()
        self.review = {"schema_version": 1, "release_branch": "v12",
                       "before_commit": self.before_sha, "after_commit": self.after_sha,
                       "context_sha256": context_digest(self.before, self.contract, "v12"),
                       "documents": {path: {"before": list(self.before[path]) if path in self.before else None,
                                             "after": list(self.after[path])}
                                     for path in DOCUMENT_PATHS}}

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, stderr=subprocess.PIPE)

    def write(self, path, contents):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(contents)

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-qm", "test(fuzz): update context fixture")
        return self.git("rev-parse", "HEAD").decode().strip()

    def files(self):
        files = {}
        for entry in self.git("ls-tree", "-rz", "HEAD").split(b"\0"):
            if entry:
                meta, path = entry.split(b"\t")
                mode, kind, oid = meta.decode().split()
                self.assertEqual(kind, "blob")
                files[path.decode()] = (mode, oid)
        return files

    def normalize(self, files, contract=None, branch="v12", review=None):
        return normalized_files(files, contract or self.contract, branch, review or self.review)

    def test_exact_transition_preserves_identity_without_mutating_source_discovery(self):
        self.assertEqual(self.normalize(self.before)[0], self.before)
        raw = copy.deepcopy(self.after)
        normalized, receipt = self.normalize(self.after)
        self.assertEqual(normalized, self.before)
        self.assertEqual(receipt, self.review["context_sha256"])
        self.assertEqual(self.after, raw)
        self.assertIn("GOVERNANCE.md", self.after)
        self.assertNotIn("GOVERNANCE.md", normalized)

    def test_unreviewed_document_content_mode_deletion_and_mixed_state_are_not_normalized(self):
        for path in sorted(DOCUMENT_PATHS):
            for operation in ("new-content", "mode", "delete", "old-content"):
                with self.subTest(path=path, operation=operation):
                    self.git("reset", "--hard", self.after_sha)
                    if operation == "new-content":
                        self.write(path, "unreviewed future edit")
                    elif operation == "mode":
                        (self.root / path).chmod(0o755)
                    elif operation == "delete" or path not in self.before:
                        (self.root / path).unlink()
                    else:
                        self.write(path, "old document " + path)
                    self.commit()
                    files = self.files()
                    self.assertEqual(self.normalize(files), (files, None))

    def test_any_changed_source_lock_manifest_toolchain_or_new_file_invalidates_context(self):
        for path in sorted(set(self.before) - DOCUMENT_PATHS | {"new-consumer.rs", "docs/new.md"}):
            with self.subTest(path=path):
                self.git("reset", "--hard", self.after_sha)
                # New files from earlier cases must not affect this case.
                for new in ("new-consumer.rs", "docs/new.md"):
                    (self.root / new).unlink(missing_ok=True)
                self.write(path, 'include_str!("README.md")')
                self.commit()
                files = self.files()
                self.assertEqual(self.normalize(files), (files, None))

    def test_new_consumer_in_both_campaigns_does_not_inherit_old_document_review(self):
        self.write("core/src/lib.rs", 'include_str!("../../README.md")')
        self.commit()
        changed_after = self.files()
        self.assertIsNone(self.normalize(changed_after)[1])
        for path in DOCUMENT_PATHS:
            if path in self.before:
                self.write(path, "old document " + path)
            else:
                (self.root / path).unlink()
        self.commit()
        changed_before = self.files()
        self.assertIsNone(self.normalize(changed_before)[1])
        self.assertNotEqual(self.normalize(changed_before)[0], self.normalize(changed_after)[0])

    def test_other_branch_or_execution_contract_never_inherits_context_review(self):
        for branch in ("main", "v13"):
            self.assertEqual(self.normalize(self.after, branch=branch), (self.after, None))
        self.assertEqual(self.normalize(self.after, contract="shorter or different campaign"),
                         (self.after, None))

    def test_development_needs_its_own_reviewed_source_and_document_states(self):
        paths = document_paths("main")
        before = {p: e for p, e in self.before.items() if p not in DOCUMENT_PATHS - paths}
        after = {p: e for p, e in self.after.items() if p not in DOCUMENT_PATHS - paths}
        review = copy.deepcopy(self.review)
        review["release_branch"] = "main"
        review["documents"] = {p: e for p, e in review["documents"].items() if p in paths}
        review["context_sha256"] = context_digest(before, self.contract, "main")
        self.assertEqual(self.normalize(after, branch="main", review=review)[0], before)
        self.assertIsNone(self.normalize(after, branch="v12", review=review)[1])

    def test_only_explicit_control_paths_are_ignored_by_context(self):
        for path in CONTROL_PATHS:
            self.write(path, "changed control-plane input")
        self.commit()
        self.assertEqual(self.normalize(self.files())[1], self.review["context_sha256"])
        self.write(".github/unknown.py", "unreviewed helper")
        self.commit()
        self.assertIsNone(self.normalize(self.files())[1])

    def test_malformed_review_cannot_grant_credit(self):
        cases = []
        for key, value in (("schema_version", True), ("schema_version", 2),
                           ("release_branch", "feature"), ("before_commit", "short"),
                           ("after_commit", self.before_sha), ("context_sha256", "short"),
                           ("documents", [])):
            review = copy.deepcopy(self.review)
            review[key] = value
            cases.append(review)
        for mutation in ("unknown", "missing", "mode", "oid", "null", "extra-state"):
            review = copy.deepcopy(self.review)
            documents = review["documents"]
            if mutation == "unknown": documents["core/src/lib.rs"] = documents["README.md"]
            elif mutation == "missing": del documents["README.md"]
            elif mutation == "mode": documents["README.md"]["after"][0] = "100755"
            elif mutation == "oid": documents["README.md"]["after"][1] = "short"
            elif mutation == "null": documents["README.md"]["before"] = None
            else: documents["README.md"]["future"] = None
            cases.append(review)
        for review in cases:
            with self.subTest(review=review):
                with self.assertRaises(ValueError):
                    self.normalize(self.after, review=review)

    def test_receipt_identity_includes_review_engine_table_and_tests(self):
        from fuzz_evidence import plan
        candidate = SimpleNamespace(sha="a" * 40, release_branch="v12", inventory=[],
                                    contract="execution", global_hash="inputs")
        github = SimpleNamespace(branch="v12", repository="Rullst/Rullst")
        original = plan(candidate, github, force_full=True)["policy_sha256"]
        read_text = Path.read_text
        for changed in (REVIEW, ".github/fuzz_reviewed_context.py",
                        ".github/test-fuzz-reviewed-context.py"):
            with self.subTest(changed=changed):
                def changed_text(path, *args, **kwargs):
                    content = read_text(path, *args, **kwargs)
                    return content + "changed policy" if path == ROOT / changed else content
                with patch.object(Path, "read_text", changed_text):
                    updated = plan(candidate, github, force_full=True)["policy_sha256"]
                self.assertNotEqual(original, updated)

    def test_snapshot_integration_for_reviewed_history_when_available(self):
        from fuzz_evidence_inputs import Snapshot
        review = json.loads((ROOT / REVIEW).read_text())
        for key in ("before_commit", "after_commit"):
            result = subprocess.run(["git", "cat-file", "-e", review[key] + "^{commit}"],
                                    cwd=ROOT, capture_output=True, check=False)
            if result.returncode:
                self.skipTest("historical integration requires full checkout (provided by fuzz workflow)")
        before, after = [Snapshot(review[key]) for key in ("before_commit", "after_commit")]
        self.assertEqual(before.context_review, review["context_sha256"])
        self.assertEqual(after.context_review, review["context_sha256"])
        for path, states in review["documents"].items():
            self.assertEqual(list(after.files[path]), states["after"])
            self.assertEqual(list(before.files[path]) if path in before.files else None, states["before"])
        self.assertTrue(before.ancestor_of(after))
        self.assertEqual(before.contract, after.contract)
        self.assertEqual(before.inventory, after.inventory)
        for directory in before.directories:
            self.assertEqual(before.fingerprint(directory), after.fingerprint(directory))

    def test_shipped_review_has_exact_schema_and_safe_path_set(self):
        validate(json.loads((ROOT / REVIEW).read_text()))
        self.assertFalse(DOCUMENT_PATHS & CONTROL_PATHS)


if __name__ == "__main__":
    unittest.main()
