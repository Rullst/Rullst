#!/usr/bin/env python3
"""Real Git fixtures for immutable-source mutation campaigns."""

from __future__ import annotations

import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("resolve-mutation-source.py")
SPEC = importlib.util.spec_from_file_location("resolve_mutation_source", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot import {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class MutationSourceTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.repo = Path(temporary.name)
        self.git("init", "--quiet")
        self.git("config", "user.name", "Mutation fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "core.hooksPath", "/dev/null")
        self.git("commit", "--allow-empty", "--quiet", "-m", "test(ci): release fixture")
        self.release = self.git("rev-parse", "HEAD")
        self.git("tag", "v12.1.0")
        self.git("commit", "--allow-empty", "--quiet", "-m", "fix(ci): controller fixture")
        self.workflow = self.git("rev-parse", "HEAD")

    def git(self, *args: str) -> str:
        return subprocess.run(
            ["git", *args], cwd=self.repo, text=True, capture_output=True, check=True,
        ).stdout.strip()

    def resolve(self, requested: str, mode: str = "full") -> str:
        return MODULE.resolve_source(mode, requested, self.workflow, self.repo)

    def test_default_measures_workflow_commit(self) -> None:
        self.assertEqual(self.resolve(""), self.workflow)

    def test_ancestor_measurement_preserves_release_tag(self) -> None:
        self.assertEqual(self.resolve(self.release), self.release)
        self.assertEqual(self.git("rev-parse", "v12.1.0"), self.release)
        self.assertEqual(self.git("rev-parse", "HEAD"), self.workflow)

    def test_explicit_source_cannot_override_targeted_or_recovery_policy(self) -> None:
        for mode in ("targeted", "resume", "finalize"):
            with self.subTest(mode=mode), self.assertRaises(ValueError):
                self.resolve(self.release, mode)
            self.assertEqual(self.resolve("", mode), self.workflow)

    def test_rejects_symbolic_malformed_and_missing_sources(self) -> None:
        for source in ("HEAD", "v12.1.0", self.release[:8], "--help", "f" * 40,
                       self.release + "\n", self.release.upper()):
            with self.subTest(source=source), self.assertRaises(ValueError):
                self.resolve(source)

    def test_rejects_non_commit_object(self) -> None:
        with self.assertRaises(ValueError):
            self.resolve(self.git("rev-parse", "HEAD^{tree}"))

    def test_rejects_non_ancestor_commit(self) -> None:
        self.git("checkout", "--quiet", "--detach", self.release)
        self.git("commit", "--allow-empty", "--quiet", "-m", "test(ci): unrelated branch")
        with self.assertRaises(ValueError):
            self.resolve(self.git("rev-parse", "HEAD"))

    def test_rejects_invalid_controller_and_mode(self) -> None:
        with self.assertRaises(ValueError):
            MODULE.resolve_source("full", self.release, "HEAD", self.repo)
        with self.assertRaises(ValueError):
            self.resolve("", "unknown")


if __name__ == "__main__":
    unittest.main()
