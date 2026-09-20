#!/usr/bin/env python3
"""Real Git fixtures for version, tag, branch and package admission."""

import importlib.util
import json
import re
from pathlib import Path
import subprocess
import tempfile
import unittest

from release_line import policy_line, tagged_version

SCRIPT = Path(__file__).with_name("check-release-source.py")
SPEC = importlib.util.spec_from_file_location("check_release_source", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot import {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ReleaseSourceTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.git("init", "--quiet")
        self.git("config", "user.name", "Release fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "core.hooksPath", "/dev/null")
        self.policy = {"schema_version": 3, "required_major": 13, "required_branch": "v13"}
        (self.root / ".github").mkdir()
        (self.root / "rullst").mkdir()
        (self.root / ".github/release-required-workflows.json").write_text(json.dumps(self.policy))
        (self.root / ".github/release-order.json").write_text('["rullst"]')
        self.manifest = self.root / "rullst/Cargo.toml"
        self.manifest.write_text('[package]\nname="rullst"\nversion="13.0.0"\n')
        self.git("add", ".")
        self.git("commit", "--quiet", "-m", "test(release): source fixture")
        self.sha = self.git("rev-parse", "HEAD")
        self.git("tag", "v13.0.0")
        self.state = {"name": "v13", "protected": True, "commit": {"sha": self.sha}}

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, stderr=subprocess.PIPE, text=True).strip()

    def validate(self, state=None, tag="v13.0.0"):
        MODULE.validate_source(self.root, tag, self.sha, self.state if state is None else state)

    def test_matching_protected_source_and_packages_are_accepted(self):
        self.validate()

    def test_declared_push_workflows_cover_main_and_v13(self):
        directory = SCRIPT.parent
        policy = json.loads((directory / "release-required-workflows.json").read_text())
        for requirement in policy["workflows"]:
            if requirement["event"] != "push":
                continue
            source = (directory / "workflows" / requirement["workflow"]).read_text()
            filters = re.findall(r"(?m)^    branches: \[([^\]]+)\]$", source)
            with self.subTest(workflow=requirement["workflow"]):
                self.assertEqual(len(filters), 2, "review push and PR trigger layout")
                for value in filters:
                    self.assertEqual({part.strip(' \"\'') for part in value.split(',')}, {"main", "v13"})

    def test_wrong_branch_unprotected_or_stale_head_are_rejected(self):
        for state in ({**self.state, "name": "main"}, {**self.state, "protected": False},
                      {**self.state, "protected": "true"},
                      {**self.state, "commit": {"sha": "a" * 40}}):
            with self.subTest(state=state), self.assertRaises(ValueError):
                self.validate(state)

    def test_checked_out_source_must_be_the_admitted_commit(self):
        self.git("commit", "--allow-empty", "--quiet", "-m", "test(release): other head")
        with self.assertRaises(ValueError):
            self.validate()

    def test_tag_must_resolve_to_the_admitted_commit(self):
        self.git("commit", "--allow-empty", "--quiet", "-m", "test(release): other tag")
        self.git("tag", "--force", "v13.0.0")
        self.git("checkout", "--quiet", "--detach", self.sha)
        with self.assertRaises(ValueError):
            self.validate()

    def test_every_package_requires_the_exact_published_version(self):
        for extra in ('version="12.1.0"', 'version="13.0.0-alpha.1"',
                      'version="13.0.0"\npublish=false', 'version="13.0.0"\npublish=[]'):
            self.manifest.write_text('[package]\nname="rullst"\n' + extra + '\n')
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                self.validate()

    def test_canonical_tags_and_prereleases_match_only_their_major(self):
        self.assertEqual(tagged_version("v13.0.0-rc.1", self.policy), "13.0.0-rc.1")
        for tag in ("13.0.0", "v12.1.0", "v14.0.0", "v013.0.0", "v13.01.0",
                    "v13.0.0-rc.01", "v13.0.0-", "v13.0.0+build", "v13.0.0\n"):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                self.validate(tag=tag)

    def test_legacy_policy_is_only_v12_main(self):
        self.assertEqual(policy_line({"schema_version": 2, "required_branch": "main"}), (12, "main"))
        for policy in ({"schema_version": 2, "required_branch": "v13"},
                       {**self.policy, "required_major": 12},
                       {**self.policy, "required_branch": "main"},
                       {**self.policy, "required_major": True},
                       {**self.policy, "required_major": 14}):
            with self.subTest(policy=policy), self.assertRaises(ValueError):
                policy_line(policy)


if __name__ == "__main__":
    unittest.main()
