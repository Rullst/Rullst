#!/usr/bin/env python3
"""Regression tests for the cached Rust target policy."""

from __future__ import annotations

import importlib.util
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path


def load(module: str, script: str):
    spec = importlib.util.spec_from_file_location(module, Path(__file__).with_name(script))
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot import {script}")
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


POLICY = load("check_rust_cache_policy", "check-rust-cache-policy.py")
PIN = "a188e68d0fd2bd36ad51dccb2b07ba6e75105f30"


def workflow(options: str, *steps: str, header: str = "    steps:\n      - uses: actions/checkout@" + "0" * 40 + "\n") -> str:
    toolchain = f"      - name: Set up Rust\n        uses: actions-rust-lang/setup-rust-toolchain@{PIN} # v2.0.1\n"
    if options:
        toolchain += "        with:\n" + "".join(f"          {line}\n" for line in options.splitlines())
    return ("name: Example\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n"
            + header + toolchain + "".join(steps))


ALL_FEATURES = "      - run: cargo check --workspace --all-features\n"


class RustCachePolicyTests(unittest.TestCase):
    def errors(self, text: str) -> list[str]:
        return POLICY.check_text("example.yml", text)

    def test_uncached_toolchain_needs_nothing(self) -> None:
        self.assertEqual(self.errors(workflow("cache: false", ALL_FEATURES)), [])

    def test_cached_toolchain_must_not_save_failed_runs(self) -> None:
        for options in ("", 'rustflags: ""', "cache-on-failure: true", "cache: true"):
            with self.subTest(options=options):
                errors = self.errors(workflow(options))
                self.assertEqual(len(errors), 1)
                self.assertIn("cache-on-failure: false", errors[0])
        for options in ("cache-on-failure: false", "cache-on-failure: 'false'",
                        'cache-on-failure: "False" # reviewed'):
            with self.subTest(options=options):
                self.assertEqual(self.errors(workflow(options)), [])

    def test_reads_steps_listed_at_the_key_indentation(self) -> None:
        text = workflow("", ALL_FEATURES, header="    steps:\n").replace("\n      ", "\n    ")
        errors = self.errors(text)
        self.assertEqual(len(errors), 1)
        self.assertIn("cache-on-failure: false", errors[0])

    def test_fails_closed_on_unreadable_steps(self) -> None:
        errors = self.errors(
            f"jobs:\n  build:\n    uses: actions-rust-lang/setup-rust-toolchain@{PIN}\n")
        self.assertEqual(len(errors), 1)
        self.assertIn("review the cache policy by hand", errors[0])

    def test_repository_workflows_hold(self) -> None:
        output = StringIO()
        with redirect_stdout(output):
            self.assertEqual(POLICY.main(), 0)
        self.assertIn("Rust cache policy holds", output.getvalue())


if __name__ == "__main__":
    unittest.main()
