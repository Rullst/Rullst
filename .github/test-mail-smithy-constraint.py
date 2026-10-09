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


class CapitalPkcs1Constraint(unittest.TestCase):
    def test_current_nfse_pins_the_compatible_pkcs1_release(self):
        manifest = tomllib.loads((ROOT / "rullst-capital/Cargo.toml").read_text())
        self.assertEqual(manifest["dependencies"]["pkcs1"]["version"], "=0.8.0-rc.4")
        self.assertIn("dep:pkcs1", manifest["features"]["nfse"])

    def test_older_baselines_get_the_same_resolver_only_constraint(self):
        script = (ROOT / ".github/check-semver.sh").read_text()
        self.assertIn('"$package" == "rullst-capital" || "$package" == "rullst"', script)
        self.assertIn("[dependencies.pkcs1]", script)
        self.assertIn('version = "=0.8.0-rc.4"', script)


if __name__ == "__main__":
    unittest.main()
