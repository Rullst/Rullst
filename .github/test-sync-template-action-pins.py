#!/usr/bin/env python3
"""Tests for sync-template-action-pins.py."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPT = Path(__file__).with_name("sync-template-action-pins.py")
SPEC = importlib.util.spec_from_file_location("sync_template_action_pins", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)

OLD = "3d3c42e5aac5ba805825da76410c181273ba90b1"
NEW = "1111111111111111111111111111111111111111"
NODE = "949feb2413d6458794dcd2491c4babbbce0c15c1"
HOG_OLD = "db665f0c24c63a536220e0e4a3ec9b9dc250e6be"
HOG_NEW = "2222222222222222222222222222222222222222"

TEMPLATE = f"""name: CI
on: push
jobs:
  test:
    steps:
      - name: Check out
        uses: actions/checkout@{OLD} # v7.0.1
        with:
          persist-credentials: false
      - uses: trufflesecurity/trufflehog@{HOG_OLD} # main @ 2026-11-15
      - run: echo "uses: is only text here"
"""


class Repository:
    """Rullst's own workflows and one template in a temporary directory."""

    def __init__(self, root: Path, workflows: dict[str, str], template: str = TEMPLATE) -> None:
        self.workflows = root / "workflows"
        self.workflows.mkdir()
        for name, text in workflows.items():
            (self.workflows / name).write_text(text)
        self.template = root / "ci.yml"
        self.template.write_text(template)

    def run(self, *flags: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with mock.patch.object(MODULE, "WORKFLOWS", self.workflows), \
                mock.patch.object(MODULE, "TEMPLATES", (self.template,)), \
                contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            status = MODULE.main(list(flags))
        return status, out.getvalue(), err.getvalue()


def workflow(*pins: str) -> str:
    steps = "\n".join(f"      - uses: {pin}" for pin in pins)
    return f"jobs:\n  a:\n    steps:\n{steps}\n      - uses: ./.github/workflows/reusable.yml\n"


class Sync(unittest.TestCase):
    def test_a_bumped_pin_is_copied_with_its_comment(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Repository(Path(directory), {
                "a.yml": workflow(f"actions/checkout@{NEW} # v7.1.0", f"trufflesecurity/trufflehog@{HOG_OLD} # main @ 2026-11-15"),
            })
            status, out, _ = repository.run()
            self.assertEqual(status, 0)
            self.assertIn(f"ci.yml:7: actions/checkout@{OLD} # v7.0.1 -> {NEW} # v7.1.0", out)
            text = repository.template.read_text()
            self.assertIn(f"        uses: actions/checkout@{NEW} # v7.1.0\n        with:\n", text)
            self.assertEqual(text, TEMPLATE.replace(f"{OLD} # v7.0.1", f"{NEW} # v7.1.0"))
            # A second run changes nothing.
            self.assertEqual(repository.run("--check")[0], 0)

    def test_check_reports_without_writing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Repository(Path(directory), {
                "a.yml": workflow(f"actions/checkout@{NEW} # v7.1.0"),
                "b.yml": workflow(f"trufflesecurity/trufflehog@{HOG_OLD} # main @ 2026-11-15"),
            })
            status, out, err = repository.run("--check")
            self.assertEqual(status, 1)
            self.assertIn("-> " + NEW, out)
            self.assertIn("run `python3 .github/sync-template-action-pins.py` and commit the result", err)
            self.assertEqual(repository.template.read_text(), TEMPLATE)

    def test_the_newest_of_several_pins_wins(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Repository(Path(directory), {
                # A partially applied update: one workflow still has the old pin.
                "a.yml": workflow(f"actions/checkout@{OLD} # v7.0.1", f"trufflesecurity/trufflehog@{HOG_OLD} # main @ 2026-11-15"),
                "b.yml": workflow(f"actions/checkout@{NEW} # v7.10.0", f"trufflesecurity/trufflehog@{HOG_NEW} # main @ 2026-12-01"),
                "c.yml": workflow(f"actions/checkout@{NODE} # v7.9.3"),
            })
            self.assertEqual(repository.run()[0], 0)
            text = repository.template.read_text()
            self.assertIn(f"actions/checkout@{NEW} # v7.10.0", text)
            self.assertIn(f"trufflesecurity/trufflehog@{HOG_NEW} # main @ 2026-12-01", text)

    def test_unordered_or_tied_pins_and_unknown_actions_stop_without_writing(self) -> None:
        cases = {
            "cannot order": {"a.yml": workflow(f"actions/checkout@{NEW} # v7.1.0", f"actions/checkout@{NODE} # main @ 2026-11-15")},
            "share the newest version": {"a.yml": workflow(f"actions/checkout@{NEW} # v7.1.0", f"actions/checkout@{NODE} # 7.1.0")},
            "no Rullst workflow uses trufflesecurity/trufflehog": {"a.yml": workflow(f"actions/checkout@{OLD} # v7.0.1")},
        }
        for message, workflows in cases.items():
            with self.subTest(message=message), tempfile.TemporaryDirectory() as directory:
                repository = Repository(Path(directory), workflows)
                status, _, err = repository.run()
                self.assertEqual(status, 2)
                self.assertIn(message, err)
                self.assertEqual(repository.template.read_text(), TEMPLATE)

    def test_a_template_line_that_is_not_a_plain_pin_is_an_error(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repository = Repository(
                Path(directory),
                {"a.yml": workflow(f"actions/checkout@{OLD} # v7.0.1")},
                template="steps:\n  - uses: ./local-action\n",
            )
            status, _, err = repository.run()
            self.assertEqual(status, 2)
            self.assertIn("ci.yml:2: not a plain `uses: owner/repo@sha # version` line", err)

    def test_own_pins_skip_unpinned_and_local_references(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "a.yml").write_text(workflow(
                f"actions/checkout@{OLD} # v7.0.1", "actions/cache@v4", f"actions/setup-node@{NODE}",
            ))
            self.assertEqual(MODULE.own_pins(root), {"actions/checkout": {(OLD, "v7.0.1")}})

    def test_the_repository_templates_are_in_sync(self) -> None:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            status = MODULE.main(["--check"])
        self.assertEqual(status, 0, out.getvalue() + err.getvalue())


if __name__ == "__main__":
    unittest.main()
