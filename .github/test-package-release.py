#!/usr/bin/env python3
"""An unadmitted workspace member must never enter the release archives."""
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class PackageInventoryTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        (self.root / ".github").mkdir()
        self.inventory = self.root / ".github/release-order.json"
        self.inventory.write_text('["rullst-macros","rullst-core"]')
        self.receipt = self.root / "arguments.json"
        self.cargo = self.root / "cargo-probe"
        self.cargo.write_text("#!/usr/bin/env python3\nimport json,os,sys\nfrom pathlib import Path\nPath(os.environ['PACKAGE_RECEIPT']).write_text(json.dumps(sys.argv[1:]))\n")
        self.cargo.chmod(0o700)
        self.env = dict(os.environ, CARGO=str(self.cargo), PACKAGE_RECEIPT=str(self.receipt))

    def package(self, *args):
        return subprocess.run(["bash", str(ROOT / ".github/package-release.sh"), *args], cwd=self.root, env=self.env, capture_output=True, text=True, check=False)

    def test_only_admitted_packages_are_selected_and_verification_is_explicit(self):
        # The presence of another crate has no effect on the release inventory.
        (self.root / "rullst-supervision").mkdir()
        (self.root / "rullst-supervision/Cargo.toml").write_text('[package]\nname="rullst-supervision"\nversion="13.0.0-alpha.1"\npublish=false\n')
        expected = ["package", "--package", "rullst-macros", "--package", "rullst-core", "--all-features", "--locked"]
        self.assertEqual(self.package().returncode, 0)
        self.assertEqual(json.loads(self.receipt.read_text()), expected)
        self.assertEqual(self.package("--no-verify").returncode, 0)
        self.assertEqual(json.loads(self.receipt.read_text()), expected + ["--no-verify"])
        self.assertNotIn("--workspace", expected)

    def test_invalid_or_empty_inventory_never_starts_cargo(self):
        for inventory in [[], ["rullst", "rullst"], [1], ["bad name"], ["$(false)"], {"crate": "rullst"}]:
            with self.subTest(inventory=inventory):
                self.inventory.write_text(json.dumps(inventory))
                self.assertNotEqual(self.package().returncode, 0)
                self.assertFalse(self.receipt.exists())

    def test_callers_cannot_override_the_selected_packages(self):
        for flag in ["--workspace", "--package", "--exclude", "--manifest-path"]:
            self.assertNotEqual(self.package(flag).returncode, 0)
            self.assertFalse(self.receipt.exists())

    def test_candidate_audit_is_explicit_and_never_default_release_admission(self):
        self.inventory.write_text('["rullst-core"]')
        license_text = (ROOT / "LICENSE").read_bytes()
        (self.root / "LICENSE").write_bytes(license_text)
        archives = self.root / "packages"
        archives.mkdir()
        for name in ["rullst-core", "rullst-supervision"]:
            with tarfile.open(archives / f"{name}-13.0.0-alpha.1.crate", "w:gz") as archive:
                for path, content in {"LICENSE": license_text, "Cargo.toml": b"[package]\n", "README.md": b"Fixture\n", "src/lib.rs": b"// fixture\n"}.items():
                    info = tarfile.TarInfo(f"{name}-13.0.0-alpha.1/{path}")
                    info.size = len(content)
                    archive.addfile(info, io.BytesIO(content))
        args = ["bash", str(ROOT / ".github/audit-packages.sh"), "13.0.0-alpha.1", str(archives)]
        def audit(extra):
            return subprocess.run(args + extra, cwd=self.root, capture_output=True, text=True, check=False).returncode
        self.assertNotEqual(audit([]), 0)
        self.assertEqual(audit(["--supervision-candidate"]), 0)
        self.assertNotEqual(audit(["--unknown"]), 0)
        self.inventory.write_text('["rullst-core","rullst-supervision"]')
        self.assertNotEqual(audit(["--supervision-candidate"]), 0)
        self.assertEqual(audit([]), 0)


if __name__ == "__main__":
    unittest.main()
