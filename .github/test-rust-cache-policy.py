#!/usr/bin/env python3
"""Regression tests for the cached Rust target and DuckDB source repair."""

from __future__ import annotations

import errno
import importlib.util
import os
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path
from unittest import mock


def load(module: str, script: str):
    spec = importlib.util.spec_from_file_location(module, Path(__file__).with_name(script))
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot import {script}")
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


REPAIR = load("repair_duckdb_sources", "repair-duckdb-sources.py")
POLICY = load("check_rust_cache_policy", "check-rust-cache-policy.py")
PIN = "a188e68d0fd2bd36ad51dccb2b07ba6e75105f30"
KEY = "0123456789abcdef"


def stripped_tree(root: Path) -> Path:
    """A source tree as Swatinem/rust-cache saves it: directories, no files."""
    for directory in ("duckdb/src/include", "duckdb/third_party/fmt", "duckdb/extension/loader"):
        (root / directory).mkdir(parents=True)
    return root


def complete_tree(root: Path) -> Path:
    (root / "duckdb/src/include").mkdir(parents=True)
    (root / "duckdb/src/include/duckdb.hpp").write_text("// header\n")
    (root / ".complete").write_text("")
    return root


class RepairDuckdbSourcesTests(unittest.TestCase):
    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.target = Path(self.scratch.name) / "target"
        self.target.mkdir()

    def tearDown(self) -> None:
        self.scratch.cleanup()

    def test_stripped_tree_blocks_publish_until_repaired(self) -> None:
        # libduckdb-sys publishes a staged tree with rename(2), which cannot
        # replace a non-empty directory: the CI failure, reproduced.
        sources = self.target / "duckdb-sources"
        stripped_tree(sources / KEY)
        staging = complete_tree(sources / ".duckdb-unpack-2723-1")
        with self.assertRaises(OSError) as raised:
            os.rename(staging, sources / KEY)
        self.assertIn(raised.exception.errno, {errno.ENOTEMPTY, errno.EEXIST})

        removed = REPAIR.repair(self.target)

        self.assertEqual(len(removed), 2)
        self.assertIn("no .complete marker", removed[1])
        self.assertEqual(list(sources.iterdir()), [])
        staging = complete_tree(sources / ".duckdb-unpack-2723-2")
        os.rename(staging, sources / KEY)
        self.assertTrue((sources / KEY / ".complete").is_file())

    def test_keeps_complete_trees_and_the_rest_of_the_target(self) -> None:
        sources = self.target / "duckdb-sources"
        complete = complete_tree(sources / "fedcba9876543210")
        stripped_tree(sources / KEY)
        (sources / ".duckdb-unpack-99-0" / "duckdb").mkdir(parents=True)
        (sources / "stray-file").write_text("")
        (sources / "marker-is-a-directory" / ".complete").mkdir(parents=True)
        triple = stripped_tree(self.target / "x86_64-unknown-linux-gnu/duckdb-sources" / KEY)
        out = self.target / "debug/build/libduckdb-sys-011a274bdf9aa8b8/out"
        out.mkdir(parents=True)
        (out / "libduckdb.a").write_text("archive")

        removed = REPAIR.repair(self.target)

        self.assertEqual(len(removed), 5)
        self.assertEqual(sorted(path.name for path in sources.iterdir()), [complete.name])
        self.assertTrue((complete / "duckdb/src/include/duckdb.hpp").is_file())
        self.assertFalse(triple.exists())
        self.assertTrue((out / "libduckdb.a").is_file())

    def test_does_not_follow_symbolic_links(self) -> None:
        outside = complete_tree(Path(self.scratch.name) / "outside")
        (outside / ".complete").unlink()
        sources = self.target / "duckdb-sources"
        sources.mkdir()
        (sources / KEY).symlink_to(outside, target_is_directory=True)

        self.assertEqual(len(REPAIR.repair(self.target)), 1)

        self.assertFalse((sources / KEY).exists())
        self.assertTrue((outside / "duckdb/src/include/duckdb.hpp").is_file())

    def test_missing_or_clean_targets_are_left_alone(self) -> None:
        self.assertEqual(REPAIR.repair(Path(self.scratch.name) / "absent"), [])
        self.assertEqual(REPAIR.repair(self.target), [])
        stripped_tree(self.target / "duckdb-sources" / KEY)
        output = StringIO()
        with mock.patch.dict(os.environ, {"CARGO_TARGET_DIR": str(self.target)}), \
                redirect_stdout(output):
            self.assertEqual(REPAIR.main(["repair-duckdb-sources.py"]), 0)
        self.assertIn("Removed incomplete DuckDB source tree", output.getvalue())
        self.assertFalse((self.target / "duckdb-sources" / KEY).exists())
        with redirect_stderr(StringIO()):
            self.assertEqual(REPAIR.main(["repair-duckdb-sources.py", "a", "b"]), 2)


def workflow(options: str, *steps: str, header: str = "    steps:\n      - uses: actions/checkout@" + "0" * 40 + "\n") -> str:
    toolchain = f"      - name: Set up Rust\n        uses: actions-rust-lang/setup-rust-toolchain@{PIN} # v2.0.1\n"
    if options:
        toolchain += "        with:\n" + "".join(f"          {line}\n" for line in options.splitlines())
    return ("name: Example\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n"
            + header + toolchain + "".join(steps))


REPAIR_STEP = "      - run: python3 .github/repair-duckdb-sources.py\n"
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

    def test_duckdb_builds_need_the_repair_first(self) -> None:
        options = "cache-on-failure: false"
        self.assertEqual(self.errors(workflow(options, REPAIR_STEP, ALL_FEATURES)), [])
        self.assertEqual(self.errors(workflow(
            options, "      - run: |\n          python3 .github/repair-duckdb-sources.py\n"
                     "          cargo test --all-features\n")), [])
        for steps in (
            (ALL_FEATURES,),
            (ALL_FEATURES, REPAIR_STEP),
            ("      - run: cargo test -p app --features orm-duckdb\n", REPAIR_STEP),
            ("      - name: Clippy\n        run: |\n          cargo clippy \\\n"
             "            -p rullst-orm \\\n            --lib --all-features\n",),
        ):
            with self.subTest(steps=steps):
                errors = self.errors(workflow(options, *steps))
                self.assertEqual(len(errors), 1)
                self.assertIn("repair-duckdb-sources.py", errors[0])

    def test_other_cargo_commands_and_comments_need_no_repair(self) -> None:
        self.assertEqual(self.errors(workflow(
            "cache-on-failure: false",
            "      - run: cargo build -p rullst-iot --no-default-features # not duckdb\n",
            "      - run: echo --all-features\n",
        )), [])

    def test_reads_steps_listed_at_the_key_indentation(self) -> None:
        text = workflow("", ALL_FEATURES, header="    steps:\n").replace("\n      ", "\n    ")
        self.assertEqual(len(self.errors(text)), 2)

    def test_fails_closed_on_unreadable_steps(self) -> None:
        errors = self.errors(
            f"jobs:\n  build:\n    uses: actions-rust-lang/setup-rust-toolchain@{PIN}\n")
        self.assertEqual(len(errors), 1)
        self.assertIn("review the cache policy by hand", errors[0])

    def test_repository_workflows_hold(self) -> None:
        output = StringIO()
        with redirect_stdout(output):
            self.assertEqual(POLICY.main(["check-rust-cache-policy.py"]), 0)
        self.assertIn("Rust cache policy holds", output.getvalue())


if __name__ == "__main__":
    unittest.main()
