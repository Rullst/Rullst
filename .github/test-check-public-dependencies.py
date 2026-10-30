#!/usr/bin/env python3
"""Tests for check-public-dependencies.py."""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("check-public-dependencies.py")
SPEC = importlib.util.spec_from_file_location("check_public_dependencies", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

POLICY = "docs/src/compatibility-policy.md#public-dependencies"
DOC = """# Policy

## Public dependencies

| Crate | 13.x line | Where |
| :--- | :--- | :--- |
| `axum` | `0.8` | Core |
| `tokio` | `1` | Core |

## Next
"""


def lock_package(name: str, version: str, dependencies: list[str] | None = None, local: bool = False) -> str:
    text = f'[[package]]\nname = "{name}"\nversion = "{version}"\n'
    if not local:
        text += 'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
    if dependencies:
        text += "dependencies = [" + ", ".join(f'"{entry}"' for entry in dependencies) + "]\n"
    return text + "\n"


class Workspace:
    """A minimal repository: one release library, its manifest and Cargo.lock."""

    def __init__(self, root: Path, *, axum: str = "0.8.4", locked_axum: str = "0.8.9",
                 dev_axum: str | None = None, table: str = DOC,
                 dependencies: dict[str, str] | None = None) -> None:
        (root / ".github").mkdir()
        (root / "docs/src").mkdir(parents=True)
        (root / "core").mkdir()
        (root / ".github/release-order.json").write_text(json.dumps(["core", "cargo-rullst"]))
        (root / ".github/public-dependencies.json").write_text(json.dumps({
            "schema_version": 1, "major": 13, "policy": POLICY,
            "dependencies": dependencies or {"axum": "0.8", "tokio": "1"},
        }))
        (root / "docs/src/compatibility-policy.md").write_text(table)
        (root / "Cargo.toml").write_text('[workspace]\nmembers = ["core"]\n\n'
                                         '[workspace.dependencies]\nrt = { package = "tokio", version = "1.40" }\n')
        manifest = ('[package]\nname = "core"\nversion = "13.0.0"\n\n[dependencies]\n'
                    f'axum = "{axum}"\nrt = {{ workspace = true, features = ["rt"] }}\n')
        if dev_axum:
            manifest += f'\n[dev-dependencies]\naxum = "{dev_axum}"\n'
        (root / "core/Cargo.toml").write_text(manifest)
        core_dependencies = [f"axum {locked_axum}", "tokio"]
        packages = [lock_package("axum", locked_axum), lock_package("tokio", "1.54.1")]
        if dev_axum:
            core_dependencies.append("axum 0.9.0")
            packages.append(lock_package("axum", "0.9.0"))
        packages.append(lock_package("core", "13.0.0", core_dependencies, local=True))
        (root / "Cargo.lock").write_text("version = 4\n\n" + "".join(packages))


class CheckTests(unittest.TestCase):
    def run_check(self, **options) -> list[str]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            Workspace(root, **options)
            return MODULE.check(root)

    def test_lines_follow_caret_semantics(self) -> None:
        self.assertEqual(MODULE.line("0.8.9"), (0, 8))
        self.assertEqual(MODULE.line("1.54.1"), (1,))
        self.assertEqual(MODULE.line("0.0.3"), (0, 0, 3))
        self.assertEqual(MODULE.line("13.0.0-alpha.4"), (13,))
        self.assertEqual(MODULE.requirement_line("^0.8"), (0, 8))
        self.assertEqual(MODULE.requirement_line("~0.8.1"), (0, 8))
        self.assertEqual(MODULE.requirement_line("=1.2.3"), (1,))
        self.assertIsNone(MODULE.requirement_line(">=0.8"))
        self.assertIsNone(MODULE.requirement_line("*"))

    def test_a_workspace_within_its_lines_passes(self) -> None:
        self.assertEqual(self.run_check(), [])

    def test_a_requirement_outside_the_line_fails(self) -> None:
        errors = self.run_check(axum="0.9", locked_axum="0.9.1")
        self.assertTrue(any("core requires `0.9`" in error for error in errors), errors)
        self.assertTrue(any("resolves core to 0.9.1" in error for error in errors), errors)

    def test_an_unbounded_requirement_fails(self) -> None:
        errors = self.run_check(axum=">=0.8")
        self.assertTrue(any("line unbounded" in error for error in errors), errors)

    def test_a_dev_dependency_on_another_line_is_private(self) -> None:
        self.assertEqual(self.run_check(dev_axum="0.9"), [])

    def test_a_stale_entry_fails(self) -> None:
        errors = self.run_check(dependencies={"axum": "0.8", "tokio": "1", "url": "2"},
                                table=DOC.replace("| `tokio`", "| `url` | `2` | AI |\n| `tokio`"))
        self.assertEqual(len(errors), 1, errors)
        self.assertIn("url: listed in", errors[0])

    def test_the_documented_table_must_match_the_file(self) -> None:
        errors = self.run_check(table=DOC.replace("`0.8`", "`0.9`"))
        self.assertEqual(len(errors), 1, errors)
        self.assertIn("differ", errors[0])

    def test_the_repository_passes(self) -> None:
        self.assertEqual(MODULE.check(), [])


if __name__ == "__main__":
    unittest.main()
