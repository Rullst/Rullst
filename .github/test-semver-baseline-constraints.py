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
SELECTION = re.search(r"^baseline_selection_filter='\n(.*?)^'$", SCRIPT, re.DOTALL | re.MULTILINE)
if SELECTION is None:
    raise RuntimeError("Cannot locate the SemVer baseline selection.")
BASELINE_FILTER = SELECTION.group(1)


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


def select(current: str, versions: list[str], yanked: tuple[str, ...] = ()) -> dict:
    response = {"versions": [{"num": num, "yanked": num in yanked} for num in versions]}
    result = subprocess.run(
        ["jq", "-c", "--arg", "current", current, BASELINE_FILTER],
        input=json.dumps(response), capture_output=True, text=True, check=True, timeout=30,
    )
    return json.loads(result.stdout)


V12 = ["12.3.0", "12.2.0", "12.1.2", "12.0.0"]
V13_PRERELEASES = ["13.0.0-alpha.3", "13.0.0-alpha.2", "13.0.0-alpha.1"]


class BaselineSelectionTests(unittest.TestCase):
    def test_v13_prereleases_have_no_baseline_until_13_0_0(self) -> None:
        self.assertEqual(select("13.0.0-alpha.4", V13_PRERELEASES + V12), {"baseline": None})
        self.assertEqual(select("13.0.0", V13_PRERELEASES + V12), {"baseline": None})
        # A newer major's prerelease is not a baseline either.
        self.assertEqual(select("14.0.0-alpha.1", ["13.2.0", "13.0.0"]), {"baseline": None})

    def test_the_check_switches_on_at_13_0_0(self) -> None:
        published = ["13.0.0", *V13_PRERELEASES, *V12]
        self.assertEqual(select("13.0.0", published), {"baseline": "13.0.0", "release_type": "minor"})
        self.assertEqual(select("13.0.1", published), {"baseline": "13.0.0", "release_type": None})
        self.assertEqual(select("13.1.0-alpha.1", published), {"baseline": "13.0.0", "release_type": None})
        self.assertEqual(select("13.0.0-alpha.4", published), {"baseline": "13.0.0", "release_type": "older"})

    def test_latest_stable_release_is_chosen_by_version_not_by_order(self) -> None:
        published = ["13.0.9", "13.0.10", "13.1.0-rc.1", "13.0.2+build.7", "12.9.0"]
        self.assertEqual(select("13.1.0", published), {"baseline": "13.0.10", "release_type": None})
        self.assertEqual(select("13.0.10", published), {"baseline": "13.0.10", "release_type": "minor"})
        self.assertEqual(select("13.0.11", published, yanked=("13.0.10",)),
                         {"baseline": "13.0.9", "release_type": None})
        self.assertEqual(select("13.0.3", ["13.0.2+build.7"]), {"baseline": "13.0.2+build.7", "release_type": None})

    def test_v12_compares_with_its_own_line_not_the_newer_13_prereleases(self) -> None:
        self.assertEqual(select("12.3.0", V13_PRERELEASES + V12), {"baseline": "12.3.0", "release_type": "minor"})
        self.assertEqual(select("12.3.1", V13_PRERELEASES + V12), {"baseline": "12.3.0", "release_type": None})

    def test_the_script_applies_the_selection(self) -> None:
        self.assertIn('--arg current "$current_version" "$baseline_selection_filter"', SCRIPT)
        self.assertIn('${release_type:+--release-type "$release_type"}', SCRIPT)
        self.assertIn("has no stable ${current_version%%.*}.x release on crates.io yet", SCRIPT)
        self.assertIn('if [[ "$release_type" == older ]]; then', SCRIPT)
        self.assertNotIn("select(.yanked == false)][0].num", SCRIPT)


class ScopeTests(unittest.TestCase):
    def test_the_cli_library_is_reported_as_excluded_before_any_comparison(self) -> None:
        self.assertIn("not_public_api_packages=(cargo-rullst)", SCRIPT)
        notice = SCRIPT.index("::notice title=SemVer scope::$package is excluded")
        self.assertLess(notice, SCRIPT.index('"https://crates.io/api/v1/crates/$package"'))
        self.assertLess(notice, SCRIPT.index("cargo semver-checks check-release"))


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
