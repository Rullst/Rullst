#!/usr/bin/env python3
"""Exercise the SemVer job's locked baseline and its feature selection."""

from __future__ import annotations

import json
import re
import subprocess
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("check-semver.sh").read_text()
MATCH = re.search(r"^heuristic_features_filter='\n(.*?)^'$", SCRIPT, re.DOTALL | re.MULTILINE)
if MATCH is None:
    raise RuntimeError("Cannot locate the SemVer feature filter.")
FEATURE_FILTER = MATCH.group(1)


def package(name: str, features: dict[str, list[str]], dependencies: list[dict]) -> dict:
    return {"name": name, "features": features, "dependencies": dependencies}


def selected_features(metadata: dict, name: str) -> list[str]:
    result = subprocess.run(
        ["jq", "-r", "--arg", "package", name, FEATURE_FILTER],
        input=json.dumps(metadata), capture_output=True, text=True, check=True, timeout=30,
    )
    output = result.stdout.strip()
    return output.split(",") if output else []


class FeatureSelectionTests(unittest.TestCase):
    def test_every_public_feature_and_implicit_optional_dependency_is_selected(self) -> None:
        metadata = {"packages": [
            package("other", {"ignored": []}, []),
            package(
                "subject",
                {
                    "default": ["std"],
                    "std": [],
                    "duckdb": ["dep:duckdb"],
                    "unstable": [],
                    "unstable-api": [],
                    "unstable_api": [],
                    "nightly": [],
                    "bench": [],
                    "no_std": [],
                    "_internal": [],
                },
                [
                    {"name": "duckdb", "optional": True},
                    {"name": "redis", "optional": True},
                    {"name": "serde", "optional": False},
                    {"name": "tokio-postgres", "rename": "postgres", "optional": True},
                    {"name": "redis", "optional": True, "target": "cfg(unix)"},
                ],
            ),
        ]}
        self.assertEqual(
            selected_features(metadata, "subject"),
            ["default", "duckdb", "postgres", "redis", "std"],
        )

    def test_a_package_without_features_selects_none(self) -> None:
        metadata = {"packages": [package("subject", {}, [])]}
        self.assertEqual(selected_features(metadata, "subject"), [])


class LockedBaselineTests(unittest.TestCase):
    def test_both_sides_are_documented_from_their_committed_lockfiles(self) -> None:
        self.assertIn('.cargo_vcs_info.json', SCRIPT)
        self.assertIn('git archive --format=tar "$source_commit"', SCRIPT)
        self.assertIn('--locked', SCRIPT)
        self.assertIn('--baseline-rustdoc "$baseline_json"', SCRIPT)
        self.assertIn('--current-rustdoc "$current_json"', SCRIPT)
        self.assertIn('cargo semver-checks check-release', SCRIPT)

    def test_the_comparison_keeps_every_feature(self) -> None:
        self.assertNotIn('--no-default-features', SCRIPT)
        self.assertNotIn('--exclude-features', SCRIPT)
        self.assertNotIn('--only-explicit-features', SCRIPT)


if __name__ == "__main__":
    unittest.main()
