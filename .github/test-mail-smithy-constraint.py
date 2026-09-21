#!/usr/bin/env python3
"""Keep the temporary SES resolver constraint optional and consistent."""
import pathlib
import tomllib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class MailSmithyConstraint(unittest.TestCase):
    def test_native_ses_uses_the_verified_types_without_default_activation(self):
        manifest = tomllib.loads((ROOT / "rullst-mail/Cargo.toml").read_text())
        constraint = manifest["dependencies"]["aws-smithy-types"]
        self.assertEqual(constraint["version"], "=1.6.4")
        self.assertIs(constraint["optional"], True)
        self.assertIs(constraint["default-features"], False)
        self.assertIn("dep:aws-smithy-types", manifest["features"]["aws-ses"])
        self.assertEqual(manifest["features"]["default"], [])
        self.assertIn("aws-smithy-types", manifest["package"]["metadata"]["cargo-machete"]["ignored"])

    def test_published_baseline_keeps_its_source_and_comparison(self):
        script = (ROOT / ".github/check-semver.sh").read_text()
        self.assertIn('"$package" == "rullst-mail" && "$baseline" == "12.0.0"', script)
        self.assertIn('[dependencies.aws-smithy-types]', script)
        self.assertIn('version = "=1.6.3"', script)
        self.assertIn('cargo semver-checks check-release', script)
        self.assertNotIn('--no-default-features', script)
        self.assertNotIn('--exclude-features', script)


if __name__ == "__main__":
    unittest.main()
