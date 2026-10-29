#!/usr/bin/env python3
"""Offline fixtures for the pre-admission release checks."""

import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("release-preflight.py")
SPEC = importlib.util.spec_from_file_location("release_preflight", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot import {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

VERSION = "13.0.0-alpha.2"


def registry(owners: dict, published: set[str]):
    """Fake crates.io: `owners` maps a crate to its logins; absent means 404."""

    def fetch(url: str):
        name, _, rest = url.removeprefix("https://crates.io/api/v1/crates/").partition("/")
        if name not in owners:
            return 404, None
        if rest == "owners":
            return 200, {"users": [{"login": login} for login in owners[name]]}
        return (200, {"version": {}}) if name in published else (404, None)

    return fetch


class RegistryTests(unittest.TestCase):
    def test_registered_owned_unpublished_packages_pass(self):
        fetch = registry({"rullst": ["venelouis", "stevi001"]}, set())
        self.assertEqual(
            MODULE.registry_findings(["rullst"], VERSION, "venelouis", fetch), ([], [])
        )

    def test_unregistered_name_outside_the_allowlist_is_refused(self):
        fetch = registry({"rullst": ["venelouis"]}, set())
        failures, first = MODULE.registry_findings(
            ["rullst", "rullst-new"], VERSION, "venelouis", fetch, frozenset({"rullst-extras"})
        )
        self.assertEqual(first, [])
        self.assertEqual(len(failures), 1)
        self.assertIn("rullst-new is not registered", failures[0])
        self.assertIn("not in the reviewed bootstrap allowlist", failures[0])

    def test_allowlisted_unregistered_name_is_a_reviewed_first_publication(self):
        fetch = registry({"rullst": ["venelouis"]}, set())
        failures, first = MODULE.registry_findings(
            ["rullst", "rullst-new"], VERSION, "venelouis", fetch, frozenset({"rullst-new"})
        )
        self.assertEqual(failures, [])
        self.assertEqual(len(first), 1)
        self.assertTrue(first[0].startswith("rullst-new: first publication; "), first[0])
        self.assertIn("needs CRATES_IO_BOOTSTRAP_TOKEN in the crates-io environment", first[0])

    def test_allowlist_does_not_admit_a_foreign_owner(self):
        fetch = registry({"rullst-new": ["someone-else"]}, set())
        failures, first = MODULE.registry_findings(
            ["rullst-new"], VERSION, "venelouis", fetch, frozenset({"rullst-new"})
        )
        self.assertEqual(first, [])
        self.assertEqual(failures, ["rullst-new is not owned by the reviewed owner venelouis"])

    def test_allowlist_does_not_admit_registry_errors(self):
        failures, first = MODULE.registry_findings(
            ["rullst-new"], VERSION, "venelouis", lambda _: (503, None), frozenset({"rullst-new"})
        )
        self.assertEqual(first, [])
        self.assertEqual(failures, ["crates.io owner lookup for rullst-new returned HTTP 503"])

    def test_other_owner_is_refused(self):
        fetch = registry({"rullst": ["someone-else"]}, set())
        failures, first = MODULE.registry_findings(["rullst"], VERSION, "venelouis", fetch)
        self.assertEqual(first, [])
        self.assertEqual(failures, ["rullst is not owned by the reviewed owner venelouis"])

    def test_already_published_version_is_refused(self):
        fetch = registry({"rullst": ["venelouis"]}, {"rullst"})
        failures, _ = MODULE.registry_findings(["rullst"], VERSION, "venelouis", fetch)
        self.assertEqual(failures, [f"rullst {VERSION} is already published; choose a new version"])

    def test_registry_errors_are_refused(self):
        failures, _ = MODULE.registry_findings(
            ["rullst"], VERSION, "venelouis", lambda _: (503, None)
        )
        self.assertEqual(failures, ["crates.io owner lookup for rullst returned HTTP 503"])


class PolicyTests(unittest.TestCase):
    def test_bootstrap_allowlist_is_read_from_the_policy(self):
        policy = {"expected_owner": "venelouis", "bootstrap_unregistered": ["rullst-new"]}
        self.assertEqual(MODULE.bootstrap_names(policy), frozenset({"rullst-new"}))
        self.assertEqual(MODULE.bootstrap_names({"expected_owner": "venelouis"}), frozenset())

    def test_malformed_bootstrap_allowlist_is_refused(self):
        for names in ("rullst-new", [1], {"rullst-new": True}):
            with self.subTest(names=names), self.assertRaises(ValueError):
                MODULE.bootstrap_names({"bootstrap_unregistered": names})

    def test_checked_out_policy_allowlists_only_release_packages(self):
        root = Path(__file__).resolve().parent
        policy = json.loads((root / "crates-ownership-policy.json").read_text())
        order = json.loads((root / "release-order.json").read_text())
        self.assertLessEqual(MODULE.bootstrap_names(policy), frozenset(order))


class SourceTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)
        self.origin = self.base / "origin.git"
        self.root = self.base / "work"
        self.git(self.base, "init", "--quiet", "--bare", "--initial-branch=main", str(self.origin))
        self.git(self.base, "init", "--quiet", "--initial-branch=main", str(self.root))
        for key, value in (("user.name", "Preflight fixture"), ("user.email", "fixture@example.invalid"),
                           ("commit.gpgsign", "false"), ("tag.gpgsign", "false"),
                           ("core.hooksPath", "/dev/null")):
            self.git(self.root, "config", key, value)
        self.git(self.root, "remote", "add", "origin", str(self.origin))
        (self.root / "CHANGELOG.md").write_text(
            f"# Changelog\n\n## [{VERSION}] - 2026-10-20\n\nSecond prerelease.\n"
        )
        self.git(self.root, "add", ".")
        self.git(self.root, "commit", "--quiet", "-m", "test(release): preflight fixture")
        self.git(self.root, "push", "--quiet", "origin", "main")

    @staticmethod
    def git(cwd: Path, *args: str) -> None:
        subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True)

    def test_branch_head_without_tag_passes(self):
        self.assertEqual(MODULE.git_failures(self.root, f"v{VERSION}", "main"), [])

    def test_unpushed_head_is_refused(self):
        self.git(self.root, "commit", "--quiet", "--allow-empty", "-m", "test(release): local only")
        failures = MODULE.git_failures(self.root, f"v{VERSION}", "main")
        self.assertEqual(len(failures), 1)
        self.assertIn("is not the head of origin/main", failures[0])

    def test_existing_tag_is_refused(self):
        self.git(self.root, "tag", f"v{VERSION}")
        self.git(self.root, "push", "--quiet", "origin", f"v{VERSION}")
        self.assertEqual(
            MODULE.git_failures(self.root, f"v{VERSION}", "main"),
            [f"tag v{VERSION} already exists on origin"],
        )

    def test_changelog_section_must_render(self):
        self.assertEqual(MODULE.notes_failures(self.root, VERSION), [])
        failures = MODULE.notes_failures(self.root, "13.0.0-alpha.3")
        self.assertEqual(len(failures), 1)
        self.assertIn("expected exactly one dated CHANGELOG section", failures[0])


if __name__ == "__main__":
    unittest.main()
