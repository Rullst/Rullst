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
        self.assertEqual(MODULE.registry_failures(["rullst"], VERSION, "venelouis", fetch), [])

    def test_unregistered_name_is_refused_before_admission(self):
        fetch = registry({"rullst": ["venelouis"]}, set())
        failures = MODULE.registry_failures(["rullst", "rullst-new"], VERSION, "venelouis", fetch)
        self.assertEqual(len(failures), 1)
        self.assertIn("rullst-new is not registered", failures[0])
        self.assertIn("bootstrap", failures[0])

    def test_other_owner_is_refused(self):
        fetch = registry({"rullst": ["someone-else"]}, set())
        failures = MODULE.registry_failures(["rullst"], VERSION, "venelouis", fetch)
        self.assertEqual(failures, ["rullst is not owned by the reviewed owner venelouis"])

    def test_already_published_version_is_refused(self):
        fetch = registry({"rullst": ["venelouis"]}, {"rullst"})
        failures = MODULE.registry_failures(["rullst"], VERSION, "venelouis", fetch)
        self.assertEqual(failures, [f"rullst {VERSION} is already published; choose a new version"])

    def test_registry_errors_are_refused(self):
        failures = MODULE.registry_failures(["rullst"], VERSION, "venelouis", lambda _: (503, None))
        self.assertEqual(failures, ["crates.io owner lookup for rullst returned HTTP 503"])


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
    def git(cwd: Path, *args: str) -> str:
        return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True,
                              text=True).stdout.strip()

    def push_tag(self, *options: str, name: str = f"v{VERSION}") -> str:
        self.git(self.root, "tag", *options, name)
        self.git(self.root, "push", "--quiet", "origin", name)
        return self.git(self.root, "rev-parse", "HEAD")

    def tag_failures(self, fetch, packages=("rullst-macros", "rullst")):
        return MODULE.existing_tag_failures(self.root, f"v{VERSION}", "main", VERSION, list(packages), fetch)

    def test_branch_head_without_tag_passes(self):
        self.assertEqual(MODULE.git_failures(self.root, f"v{VERSION}", "main"), [])

        def offline(url):
            raise AssertionError(f"no crates.io lookup without a tag: {url}")

        self.assertEqual(self.tag_failures(offline), [])
        # Another prerelease whose name starts with this one is not this tag.
        self.push_tag(name=f"v{VERSION}0")
        self.assertEqual(self.tag_failures(offline), [])

    def test_unpushed_head_is_refused(self):
        self.git(self.root, "commit", "--quiet", "--allow-empty", "-m", "test(release): local only")
        failures = MODULE.git_failures(self.root, f"v{VERSION}", "main")
        self.assertEqual(len(failures), 1)
        self.assertIn("is not the head of origin/main", failures[0])

    def test_tag_of_a_release_that_published_nothing_prints_the_recovery_commands(self):
        commit = self.push_tag()
        lookups = []
        unpublished = registry({"rullst-macros": ["venelouis"], "rullst": ["venelouis"]}, set())

        def fetch(url):
            lookups.append(url)
            return unpublished(url)

        [failure] = self.tag_failures(fetch)
        self.assertEqual(lookups, [f"https://crates.io/api/v1/crates/{name}/{VERSION}"
                                   for name in ("rullst-macros", "rullst")])
        tag = f"v{VERSION}"
        self.assertTrue(failure.startswith(
            f"tag {tag} already exists on origin (commit {commit[:12]}), and no release package has "
            f"{VERSION} on crates.io: that release published nothing."), failure)
        self.assertIn("MAINTAINING.md §7.5: such a tag may be deleted and recreated only with the "
                      "maintainer's explicit approval. This preflight never deletes it.", failure)
        lines = [line.strip() for line in failure.splitlines()]
        commands = [line for line in lines if line.startswith(("git ", "gh "))]
        self.assertEqual(commands, [
            f"gh run list --workflow release.yml --branch {tag}",
            f"git push origin --delete refs/tags/{tag}",
            f"git tag --delete {tag}",
            "git switch main",
            "git pull --ff-only origin main",
            f"git tag {tag}",
            f"git push origin {tag}",
        ])
        # Printing the commands deletes nothing.
        self.assertIn(f"refs/tags/{tag}", self.git(self.root, "ls-remote", "--tags", "origin"))
        self.assertEqual(self.git(self.root, "tag", "--list", tag), tag)

    def test_annotated_tag_reports_its_commit(self):
        commit = self.push_tag("-a", "-m", "release")
        self.assertNotEqual(self.git(self.root, "rev-parse", f"v{VERSION}"), commit)
        [failure] = self.tag_failures(registry({}, set()))
        self.assertIn(f"(commit {commit[:12]})", failure)

    def test_tag_of_a_published_version_is_never_recreated(self):
        self.push_tag()
        owners = {"rullst-macros": ["venelouis"], "rullst": ["venelouis"]}
        [failure] = self.tag_failures(registry(owners, {"rullst-macros"}))
        self.assertEqual(failure, (
            f"tag v{VERSION} already exists on origin and 1 of 2 packages have {VERSION} on "
            "crates.io (rullst-macros). A published version is never reused (MAINTAINING.md §7.5): "
            "release the next version, or resume an interrupted publication from the same tag "
            "(docs/src/release-recovery.md)."))
        self.assertNotIn("git push origin --delete", failure)

    def test_tag_with_an_unconfirmed_registry_state_gives_no_commands(self):
        self.push_tag()
        statuses = {"rullst-macros": 404, "rullst": 503}

        def fetch(url):
            return statuses[url.split("/")[-2]], None

        [failure] = self.tag_failures(fetch)
        self.assertEqual(failure, (
            f"tag v{VERSION} already exists on origin, and crates.io could not confirm that {VERSION} "
            "is unpublished: rullst (HTTP 503). Rerun the preflight before deciding anything about the tag."))
        self.assertNotIn("git push origin --delete", failure)

    def test_changelog_section_must_render(self):
        self.assertEqual(MODULE.notes_failures(self.root, VERSION), [])
        failures = MODULE.notes_failures(self.root, "13.0.0-alpha.3")
        self.assertEqual(len(failures), 1)
        self.assertIn("expected exactly one dated CHANGELOG section", failures[0])

    def test_unmerged_changelog_fragments_are_refused(self):
        self.assertEqual(MODULE.fragment_failures(self.root), [])
        fragments = self.root / "changelog.d"
        fragments.mkdir()
        (fragments / "README.md").write_text("# Changelog fragments\n")
        self.assertEqual(MODULE.fragment_failures(self.root), [])
        (fragments / "b.md").write_text("### Fixed\n\n- B.\n")
        (fragments / "a.md").write_text("### Added\n\n- A.\n")
        failures = MODULE.fragment_failures(self.root)
        self.assertEqual(len(failures), 1)
        self.assertIn("changelog.d/ has 2 unmerged fragment(s) (a.md, b.md)", failures[0])
        self.assertIn("assemble-changelog.py --release", failures[0])


if __name__ == "__main__":
    unittest.main()
