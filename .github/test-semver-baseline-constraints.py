#!/usr/bin/env python3
"""Exercise the actual baseline guard with Cargo's offline dependency resolver."""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("check-semver.sh").read_text()
MATCH = re.search(r"      if ! grep -Eq [^\n]*tinyvec[^\n]*\n.*?      fi", SCRIPT, re.DOTALL)
if MATCH is None:
    raise RuntimeError("Cannot locate the tinyvec baseline guard.")
GUARD = textwrap.dedent(MATCH.group())


class BaselineConstraintTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.vendor = self.root / "vendor"
        self.vendor.mkdir()
        config = self.root / ".cargo"
        config.mkdir()
        (config / "config.toml").write_text(
            '[source.crates-io]\nreplace-with = "fixture"\n'
            f'[source.fixture]\ndirectory = {json.dumps(str(self.vendor))}\n'
        )
        self.manifest = self.root / "Cargo.toml"
        self.manifest.write_text(
            '[package]\nname = "baseline-fixture"\nversion = "0.0.0"\nedition = "2024"\n'
            '[lib]\npath = "lib.rs"\n'
        )
        (self.root / "lib.rs").write_text("pub fn original_api() {}\n")

    def vendor_tinyvec(self, version: str) -> None:
        package = self.vendor / f"tinyvec-{version}"
        package.mkdir()
        (package / "Cargo.toml").write_text(
            f'[package]\nname = "tinyvec"\nversion = "{version}"\n'
            '[lib]\npath = "lib.rs"\n'
        )
        (package / "lib.rs").write_text("")
        (package / ".cargo-checksum.json").write_text('{"files":{},"package":null}')

    def apply_guard(self) -> None:
        subprocess.run(
            ["bash", "-euo", "pipefail", "-c", GUARD],
            env=dict(os.environ, baseline_root=str(self.root)), check=True,
            capture_output=True, text=True, timeout=30,
        )
        self.assertEqual((self.root / "lib.rs").read_text(), "pub fn original_api() {}\n")

    def resolve(self) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["cargo", "metadata", "--offline", "--format-version", "1"],
            cwd=self.root, capture_output=True, text=True, check=False, timeout=30,
        )

    def test_published_orm_patch_pin_resolves_without_a_second_exact_pin(self) -> None:
        for version in ("1.13.2", "1.13.3"):
            self.vendor_tinyvec(version)
        orm = self.root / "orm"
        orm.mkdir()
        (orm / "Cargo.toml").write_text(
            '[package]\nname = "rullst-orm"\nversion = "12.1.0"\n'
            '[lib]\npath = "lib.rs"\n[dependencies.tinyvec]\nversion = "=1.13.3"\n'
        )
        (orm / "lib.rs").write_text("")
        with self.manifest.open("a") as manifest:
            manifest.write('[dependencies.rullst-orm]\npath = "orm"\n')
        self.apply_guard()
        result = self.resolve()
        self.assertEqual(result.returncode, 0, result.stderr)
        versions = [p["version"] for p in json.loads(result.stdout)["packages"] if p["name"] == "tinyvec"]
        self.assertEqual(versions, ["1.13.3"])

    def test_existing_published_constraint_is_not_rewritten(self) -> None:
        self.vendor_tinyvec("1.13.2")
        with self.manifest.open("a") as manifest:
            manifest.write('[dependencies.tinyvec]\nversion = "=1.13.2"\n')
        original = self.manifest.read_bytes()
        self.apply_guard()
        self.assertEqual(self.manifest.read_bytes(), original)
        result = self.resolve()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_guard_still_excludes_the_known_broken_release(self) -> None:
        self.vendor_tinyvec("1.13.0")
        self.apply_guard()
        result = self.resolve()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("tinyvec", result.stderr)


if __name__ == "__main__":
    unittest.main()
