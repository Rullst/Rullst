#!/usr/bin/env python3
"""Tests for check-unsafe-allowlist.py."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("check-unsafe-allowlist.py")
SPEC = importlib.util.spec_from_file_location("check_unsafe_allowlist", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class RustSourceTests(unittest.TestCase):
    def test_every_form_that_lowers_the_lint_is_found(self) -> None:
        for source in [
            "#[allow(unsafe_code)]\nfn f() {}",
            "#![allow(unsafe_code)]",
            "#[allow(dead_code, unsafe_code)]\nfn f() {}",
            "#[allow(unsafe_code, reason = \"FFI\")]",
            "#[expect(unsafe_code)]\nfn f() {}",
            "#[warn(unsafe_code)]\nfn f() {}",
            "#[cfg_attr(windows, allow(unsafe_code))]\nfn f() {}",
            "#[cfg_attr(all(), expect(unsafe_code))]",
            "#[allow(\n    dead_code,\n    unsafe_code,\n)]\nfn f() {}",
            "# [ allow ( unsafe_code ) ]",
            "#![\n  allow(unsafe_code)\n]",
            # A lint passed to a macro that writes the attribute.
            "lint_level!(unsafe_code);",
            "#[deny(unsafe_code)]\n#[allow(unsafe_code)]\nfn f() {}",
            "#[allow(unsafe-code)]",
        ]:
            with self.subTest(source=source):
                self.assertTrue(MODULE.rust_lowers_lint(source))

    def test_forbid_deny_and_other_identifiers_are_not_findings(self) -> None:
        for source in [
            "#![forbid(unsafe_code)]",
            "#![deny(unsafe_code)]",
            "#[deny(unsafe_code, missing_docs)]\nmod m {}",
            "# ! [ forbid ( unsafe_code ) ]",
            "pub fn scan_unsafe_code() {}",
            "fn unsafe_code_is_found() {}",
            "#[allow(dead_code)]\nfn f() {}",
            "",
        ]:
            with self.subTest(source=source):
                self.assertFalse(MODULE.rust_lowers_lint(source))


class ManifestTests(unittest.TestCase):
    def test_manifest_levels_below_deny_are_found(self) -> None:
        for manifest in [
            '[lints.rust]\nunsafe_code = "allow"\n',
            "[workspace.lints.rust]\nunsafe_code = 'warn'\n",
            '[lints.rust]\nunsafe_code = { level = "allow", priority = 1 }\n',
            '[build]\nrustflags = ["-A", "unsafe-code"]\n',
            '[build]\nrustflags = ["-Aunsafe-code"]\n',
        ]:
            with self.subTest(manifest=manifest):
                self.assertTrue(MODULE.toml_lowers_lint(manifest))
        for manifest in [
            '[lints.rust]\nunsafe_code = "forbid"\n',
            '[lints.rust]\nunsafe_code = { level = "deny", priority = 1 }\n',
            '[package]\nname = "x"\n',
        ]:
            with self.subTest(manifest=manifest):
                self.assertFalse(MODULE.toml_lowers_lint(manifest))


class WorkspaceTests(unittest.TestCase):
    def test_members_are_scanned_but_fuzz_and_target_are_not(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["core", "examples/app"]\n', encoding="utf-8"
            )
            files = {
                "core/src/lib.rs": "#![forbid(unsafe_code)]\n",
                "core/src/ffi.rs": "#[allow(dead_code, unsafe_code)]\nfn f() {}\n",
                "core/Cargo.toml": '[lints.rust]\nunsafe_code = "warn"\n',
                "core/fuzz/fuzz_targets/a.rs": "#![allow(unsafe_code)]\n",
                "core/target/debug/build/out.rs": "#![allow(unsafe_code)]\n",
                "examples/app/src/main.rs": "#[expect(unsafe_code)]\nfn main() {}\n",
                "outside/src/lib.rs": "#![allow(unsafe_code)]\n",
                ".cargo/config.toml": '[build]\nrustflags = ["-Aunsafe-code"]\n',
            }
            for name, content in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding="utf-8")
            self.assertEqual(
                MODULE.lowering_files(root),
                [
                    ".cargo/config.toml",
                    "core/Cargo.toml",
                    "core/src/ffi.rs",
                    "examples/app/src/main.rs",
                ],
            )

    def test_the_repository_lowers_the_lint_only_in_reviewed_files(self) -> None:
        self.assertEqual(MODULE.lowering_files(MODULE.ROOT), sorted(MODULE.ALLOWED))
        self.assertEqual(MODULE.main([]), 0)


if __name__ == "__main__":
    unittest.main()
