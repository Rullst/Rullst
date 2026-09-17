#!/usr/bin/env python3
"""Exercise artifact format integrity; checksums are not publisher authentication."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import subprocess

spec = importlib.util.spec_from_file_location("cli_artifacts", Path(__file__).with_name("stage-cli-artifacts.py"))
artifacts = importlib.util.module_from_spec(spec)
spec.loader.exec_module(artifacts)


class Artifacts(unittest.TestCase):
    def fixture(self, tag=None):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        source = Path(temp.name) / "source"
        source.mkdir()
        for name in artifacts.BINARIES:
            (source / name).write_bytes(b"fixture-native-content-" + name.encode())
        output = Path(temp.name) / "output"
        with patch.object(artifacts.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "rullst 12.1.0\n", "")):
            artifacts.stage(source, output, "12.1.0", "x86_64-unknown-linux-gnu", "a" * 40, "Rullst/Rullst", tag)
        return output

    def verify(self, directory, **kwargs):
        return artifacts.verify(directory, "12.1.0", "x86_64-unknown-linux-gnu", "a" * 40, "Rullst/Rullst", **kwargs)

    def test_diagnostic_and_tag_inventory_are_distinct(self):
        diagnostic = self.fixture()
        self.assertIsNone(self.verify(diagnostic)["release_tag"])
        with self.assertRaises(ValueError):
            self.verify(diagnostic, require_release=True)
        self.assertEqual(self.verify(self.fixture("v12.1.0"), require_release=True)["release_tag"], "v12.1.0")

    def test_substituted_truncated_empty_or_missing_executable_is_rejected(self):
        for replacement in (b"changed", b""):
            with self.subTest(replacement=replacement):
                directory = self.fixture()
                filename = self.verify(directory)["files"][0]["name"]
                (directory / filename).write_bytes(replacement)
                with self.assertRaises(ValueError):
                    self.verify(directory)
        directory = self.fixture()
        (directory / self.verify(directory)["files"][0]["name"]).unlink()
        with self.assertRaises(ValueError):
            self.verify(directory)

    def test_changed_source_version_platform_and_inventory_are_rejected(self):
        for field, value in (("version", "12.0.0"), ("source_commit", "b" * 40),
                             ("target", "x86_64-pc-windows-msvc"), ("repository", "other/repo"),
                             ("release_tag", "v12.0.0"), ("files", [])):
            with self.subTest(field=field):
                directory = self.fixture()
                path = directory / "cli-manifest-x86_64-unknown-linux-gnu.json"
                manifest = json.loads(path.read_text())
                manifest[field] = value
                path.write_text(json.dumps(manifest))
                with self.assertRaises(ValueError):
                    self.verify(directory)
        directory = self.fixture()
        (directory / "unexpected").write_bytes(b"extra")
        with self.assertRaises(ValueError):
            self.verify(directory)

    def test_manifest_and_checksum_inventory_are_both_bound(self):
        directory = self.fixture()
        manifest = directory / "cli-manifest-x86_64-unknown-linux-gnu.json"
        manifest.write_bytes(manifest.read_bytes() + b" ")
        with self.assertRaises(ValueError):
            self.verify(directory)
        directory = self.fixture()
        (directory / "cli-checksums-x86_64-unknown-linux-gnu.txt").write_text("bad")
        with self.assertRaises(ValueError):
            self.verify(directory)

    def test_links_and_oversized_files_are_rejected(self):
        directory = self.fixture()
        path = directory / self.verify(directory)["files"][0]["name"]
        with path.open("wb") as stream:
            stream.truncate(artifacts.MAX_BINARY + 1)
        with self.assertRaises(ValueError):
            self.verify(directory)
        path.unlink()
        try:
            path.symlink_to(directory / "cli-manifest-x86_64-unknown-linux-gnu.json")
        except OSError:
            return  # Windows without link privileges still exercises the size bound.
        with self.assertRaises(ValueError):
            self.verify(directory)

    def test_inventory_covers_unique_native_platforms_and_safe_names(self):
        self.assertEqual(len(artifacts.targets()), 4)
        for target in artifacts.targets():
            for name in artifacts.names("12.1.0", target):
                self.assertEqual(Path(name).name, name)
        for version in ("../12.1.0", "12.1.0/escape", "12.1.0\n", "latest"):
            with self.assertRaises(ValueError):
                artifacts.names(version, "x86_64-unknown-linux-gnu")

    def test_wrong_native_version_fails_before_staging(self):
        with tempfile.TemporaryDirectory() as temp:
            source = Path(temp) / "source"
            source.mkdir()
            for name in artifacts.BINARIES:
                (source / name).write_bytes(b"fixture")
            output = Path(temp) / "output"
            with patch.object(artifacts.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "rullst 12.0.0\n", "")):
                with self.assertRaises(ValueError):
                    artifacts.stage(source, output, "12.1.0", "x86_64-unknown-linux-gnu", "a" * 40, "Rullst/Rullst", None)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
