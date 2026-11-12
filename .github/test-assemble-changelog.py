#!/usr/bin/env python3
"""Fixtures for changelog fragment validation, release assembly and the PR policy."""

import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("assemble-changelog.py")
SPEC = importlib.util.spec_from_file_location("assemble_changelog", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot import {SCRIPT}")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

NOTES = Path(__file__).with_name("render-release-notes.py")
NOTES_SPEC = importlib.util.spec_from_file_location("render_release_notes", NOTES)
if NOTES_SPEC is None or NOTES_SPEC.loader is None:
    raise RuntimeError(f"cannot import {NOTES}")
NOTES_MODULE = importlib.util.module_from_spec(NOTES_SPEC)
NOTES_SPEC.loader.exec_module(NOTES_MODULE)

CHANGELOG = """# Changelog

Intro.

## [Unreleased]

Publication status is recorded elsewhere.

### Known limitations

- Hand-written limitation.

### Added

- Existing addition.

### Security

- Existing security note.

### Dependencies

- Existing dependency note.

## [13.0.0-alpha.4] - 2026-10-31

Fourth prerelease.

### Fixed

- Old fix.
"""

ASSEMBLED = """# Changelog

Intro.

## [Unreleased]

Publication status is recorded elsewhere.

## [13.0.0-rc.1] - 2026-11-20

### Known limitations

- Hand-written limitation.

### Added

- Existing addition.
- Second addition, in
  two lines.
- Third addition.

### Changed

- **Breaking:** a change.

### Fixed

- A fix with code:

  ```rust
  # fn main() {}
  ```

### Security

- Existing security note.

### Dependencies

- Existing dependency note.

## [13.0.0-alpha.4] - 2026-10-31

Fourth prerelease.

### Fixed

- Old fix.
"""


class FixtureTest(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.fragments = self.root / "changelog.d"
        self.fragments.mkdir()
        (self.fragments / "README.md").write_text("# Changelog fragments\n\n## Format\n")
        (self.root / "CHANGELOG.md").write_text(CHANGELOG)

    def fragment(self, name: str, text: str) -> Path:
        path = self.fragments / name
        path.write_text(text)
        return path

    def assert_refused(self, text: str, message: str, name: str = "entry.md"):
        path = self.fragment(name, text)
        with self.assertRaises(MODULE.ChangelogError) as refused:
            MODULE.load_fragments(self.root)
        self.assertIn(message, str(refused.exception))
        path.unlink()


class FragmentTests(FixtureTest):
    def test_valid_fragments_are_sorted_by_file_name_and_readme_is_skipped(self):
        self.fragment("b-fix.md", "### Fixed\n\n- B.\n")
        self.fragment("a-add.md", "### Added\n- A,\n  continued.\n  - nested.\n\n")
        fragments = MODULE.load_fragments(self.root)
        self.assertEqual([fragment.path.name for fragment in fragments], ["a-add.md", "b-fix.md"])
        self.assertEqual(fragments[0].category, "Added")
        self.assertEqual(fragments[0].lines, ("- A,", "  continued.", "  - nested."))
        self.assertEqual(fragments[1].lines, ("- B.",))

    def test_no_directory_means_no_fragments(self):
        for path in self.fragments.iterdir():
            path.unlink()
        self.fragments.rmdir()
        self.assertEqual(MODULE.load_fragments(self.root), [])

    def test_every_category_is_accepted(self):
        for category in ("Added", "Changed", "Deprecated", "Removed", "Fixed", "Security"):
            with self.subTest(category=category):
                path = self.fragment("entry.md", f"### {category}\n\n- Entry.\n")
                self.assertEqual(MODULE.load_fragments(self.root)[0].category, category)
                path.unlink()

    def test_unknown_or_misplaced_category_is_refused(self):
        for text in ("### Known limitations\n\n- X.\n", "### fixed\n\n- X.\n",
                     "## Fixed\n\n- X.\n", "### Fixed x\n\n- X.\n",
                     "\n### Fixed\n\n- X.\n", "- X.\n", "Fixed\n\n- X.\n"):
            with self.subTest(text=text):
                self.assert_refused(text, "entry.md:1: the first line must be one of")

    def test_empty_fragment_or_heading_alone_is_refused(self):
        self.assert_refused("", "entry.md: is empty")
        self.assert_refused("\n \n", "entry.md: is empty")
        self.assert_refused("### Fixed\n\n\n", "entry.md: has no entry below its heading")

    def test_stray_headings_are_refused_outside_code_fences(self):
        self.assert_refused("### Fixed\n\n- X.\n\n### Added\n\n- Y.\n",
                            "entry.md:5: only the first line may be a heading")
        self.assert_refused("### Fixed\n\n- X.\n  ## Details\n",
                            "entry.md:4: only the first line may be a heading")
        self.fragment("entry.md", "### Fixed\n\n- X:\n\n  ```bash\n  # comment\n  ```\n")
        self.assertEqual(len(MODULE.load_fragments(self.root)), 1)

    def test_entries_are_bullets_with_indented_continuations(self):
        self.assert_refused("### Fixed\n\nText.\n", "the entry must start with a `- ` bullet")
        self.assert_refused("### Fixed\n\n- X,\nnot indented.\n",
                            "entry.md:4: start each entry with `- `")
        self.assert_refused("### Fixed\n\n-X.\n", "the entry must start with a `- ` bullet")
        self.assert_refused("### Fixed\n\n- X:\n\n  ```\n  code\n", "a code fence is not closed")

    def test_only_slug_named_markdown_files_are_fragments(self):
        for name in ("Fix.md", "fix.txt", "fix_orm.md", "-fix.md", ".fix.md", "fix.md.orig"):
            with self.subTest(name=name):
                self.assert_refused("### Fixed\n\n- X.\n", "a fragment is a regular file named",
                                    name=name)
        (self.fragments / "nested").mkdir()
        with self.assertRaises(MODULE.ChangelogError):
            MODULE.load_fragments(self.root)

    def test_all_problems_are_reported_together(self):
        self.fragment("a.md", "### Other\n\n- X.\n")
        self.fragment("b.md", "")
        with self.assertRaises(MODULE.ChangelogError) as refused:
            MODULE.load_fragments(self.root)
        self.assertEqual(str(refused.exception).splitlines(), [
            "changelog.d/a.md:1: the first line must be one of `### Added`, `### Changed`, "
            "`### Deprecated`, `### Removed`, `### Fixed`, `### Security`",
            "changelog.d/b.md: is empty",
        ])


class ReleaseTests(FixtureTest):
    def add_release_fragments(self):
        self.fragment("10-fix.md", "### Fixed\n\n- A fix with code:\n\n  ```rust\n  # fn main() {}\n  ```\n")
        self.fragment("02-add.md", "### Added\n\n- Third addition.\n")
        self.fragment("01-add.md", "### Added\n\n- Second addition, in\n  two lines.\n\n")
        self.fragment("05-change.md", "### Changed\n\n- **Breaking:** a change.\n")

    def test_release_merges_fragments_in_canonical_order_and_deletes_them(self):
        self.add_release_fragments()
        message = MODULE.release(self.root, "13.0.0-rc.1", "2026-11-20")
        self.assertEqual((self.root / "CHANGELOG.md").read_text(), ASSEMBLED)
        self.assertEqual(sorted(path.name for path in self.fragments.iterdir()), ["README.md"])
        self.assertIn("merged 4 fragment(s) into 13.0.0-rc.1 (Added 2, Changed 1, Fixed 1)", message)
        # The version heading is the one the release tooling renders.
        notes = NOTES_MODULE.render(ASSEMBLED, "13.0.0-rc.1", "Rullst/Rullst")
        self.assertTrue(notes.startswith("## [13.0.0-rc.1] - 2026-11-20"))
        self.assertNotIn("Old fix", notes)

    def test_fragments_join_the_last_subsection_of_their_category(self):
        changelog = "## [Unreleased]\n\n### Changed\n\n- One.\n\n### Changed\n\n- Two.\n\n### Fixed\n\n"
        fragments = [MODULE.Fragment(Path("x.md"), "Changed", ("- Three.",))]
        self.assertEqual(
            MODULE.assemble(changelog, fragments, "1.0.0", "2026-01-31"),
            "## [Unreleased]\n\n## [1.0.0] - 2026-01-31\n\n### Changed\n\n- One.\n\n"
            "### Changed\n\n- Two.\n- Three.\n",
        )

    def test_unreleased_without_preamble_or_later_sections(self):
        changelog = "# Changelog\n\n## [Unreleased]\n"
        fragments = [MODULE.Fragment(Path("x.md"), "Removed", ("- Gone.",))]
        self.assertEqual(
            MODULE.assemble(changelog, fragments, "1.0.0", "2026-01-31"),
            "# Changelog\n\n## [Unreleased]\n\n## [1.0.0] - 2026-01-31\n\n### Removed\n\n- Gone.\n",
        )

    def test_invalid_release_arguments_are_refused(self):
        for version, date, message in (
            ("v13.0.0", "2026-11-20", "invalid semantic version"),
            ("13.0", "2026-11-20", "invalid semantic version"),
            ("13.0.0-rc.1", "2026-02-30", "invalid release date"),
            ("13.0.0-rc.1", "2026-11-20T00:00", "invalid release date"),
            ("13.0.0-rc.1", "20261120", "invalid release date"),
            ("13.0.0-alpha.4", "2026-11-20", "already has a section for 13.0.0-alpha.4"),
        ):
            with self.subTest(version=version, date=date), \
                    self.assertRaises(MODULE.ChangelogError) as refused:
                MODULE.assemble(CHANGELOG, [], version, date)
            self.assertIn(message, str(refused.exception))

    def test_nothing_to_release_or_missing_unreleased_is_refused(self):
        for changelog, message in (
            ("## [Unreleased]\n\nPreamble only.\n\n## [1.0.0] - 2026-01-01\n", "nothing to release"),
            ("## [Unreleased]\n\n### Fixed\n\n## [1.0.0] - 2026-01-01\n", "nothing to release"),
            ("# Changelog\n", "exactly one `## [Unreleased]` heading; found 0"),
            ("## [Unreleased]\n\n## [Unreleased]\n", "exactly one `## [Unreleased]` heading; found 2"),
        ):
            with self.subTest(changelog=changelog), \
                    self.assertRaises(MODULE.ChangelogError) as refused:
                MODULE.assemble(changelog, [], "2.0.0", "2026-11-20")
            self.assertIn(message, str(refused.exception))

    def test_an_invalid_fragment_leaves_everything_unchanged(self):
        self.add_release_fragments()
        self.fragment("99-bad.md", "### Fixed\n\nNo bullet.\n")
        before = sorted(path.name for path in self.fragments.iterdir())
        with self.assertRaises(MODULE.ChangelogError):
            MODULE.release(self.root, "13.0.0-rc.1", "2026-11-20")
        self.assertEqual((self.root / "CHANGELOG.md").read_text(), CHANGELOG)
        self.assertEqual(sorted(path.name for path in self.fragments.iterdir()), before)

    def test_command_line(self):
        self.add_release_fragments()

        def run(*args):
            return subprocess.run([sys.executable, str(SCRIPT), "--root", str(self.root), *args],
                                  capture_output=True, text=True)

        self.assertEqual(run("--check").returncode, 0)
        for args in (("--release", "13.0.0-rc.1"), ("--check", "--date", "2026-11-20"),
                     ("--check", "--branch", "main"), (),
                     ("--release", "13.0.0-rc.1", "--date", "2026-11-20", "--base", "HEAD")):
            with self.subTest(args=args):
                self.assertEqual(run(*args).returncode, 2)
        done = run("--release", "13.0.0-rc.1", "--date", "2026-11-20")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual((self.root / "CHANGELOG.md").read_text(), ASSEMBLED)
        again = run("--release", "13.0.0-rc.1", "--date", "2026-11-20")
        self.assertEqual(again.returncode, 1)
        self.assertIn("already has a section for 13.0.0-rc.1", again.stderr)


class PolicyTests(FixtureTest):
    def setUp(self):
        super().setUp()
        self.git("init", "--quiet", "--initial-branch=main")
        for key, value in (("user.name", "Changelog fixture"), ("user.email", "fixture@example.invalid"),
                           ("commit.gpgsign", "false"), ("core.hooksPath", "/dev/null")):
            self.git("config", key, value)
        self.base = self.commit("test(changelog): base fixture")

    def git(self, *args: str) -> str:
        return subprocess.run(["git", *args], cwd=self.root, check=True,
                              capture_output=True, text=True).stdout.strip()

    def commit(self, message: str) -> str:
        self.git("add", "--all")
        self.git("commit", "--quiet", "--allow-empty", "-m", message)
        return self.git("rev-parse", "HEAD")

    def edit_changelog(self, old: str, new: str):
        path = self.root / "CHANGELOG.md"
        path.write_text(path.read_text().replace(old, new, 1))

    def test_lines_added_under_unreleased_are_refused(self):
        self.edit_changelog("- Existing addition.\n", "- Existing addition.\n- New entry.\n")
        self.edit_changelog("### Dependencies\n", "### Fixed\n\n- Another.\n\n### Dependencies\n")
        self.commit("docs(changelog): edit unreleased")
        self.assertEqual(MODULE.unreleased_additions(self.root, self.base), [
            "CHANGELOG.md:16: - New entry.",
            "CHANGELOG.md:22: ### Fixed",
            "CHANGELOG.md:24: - Another.",
        ])
        with self.assertRaises(MODULE.ChangelogError) as refused:
            MODULE.check(self.root, self.base, "fix/v13-something")
        self.assertIn("adds lines under `## [Unreleased]`", str(refused.exception))
        self.assertIn("CHANGELOG.md:16: - New entry.", str(refused.exception))
        # Release preparation may edit [Unreleased]; other `release` names may not.
        self.assertIn("release-preparation branch",
                      MODULE.check(self.root, self.base, "chore/v13-release-13.0.0-rc.1"))
        for branch in ("chore/v13-alpha4-release-notes", "release/13.0.0-rc.1",
                       "chore/release-13.0.0", "x/chore/v13-release-1"):
            with self.subTest(branch=branch), self.assertRaises(MODULE.ChangelogError):
                MODULE.check(self.root, self.base, branch)

    def test_fragments_removals_and_older_sections_are_allowed(self):
        self.fragment("new-entry.md", "### Fixed\n\n- New entry.\n")
        self.edit_changelog("- Existing security note.\n", "")
        self.edit_changelog("- Old fix.\n", "- Old fix, reworded.\n- Another old fix.\n")
        self.edit_changelog("### Added\n", "### Added\n\n")
        self.commit("docs(changelog): add a fragment")
        self.assertEqual(MODULE.unreleased_additions(self.root, self.base), [])
        self.assertIn("1 changelog fragment(s) valid; no lines added",
                      MODULE.check(self.root, self.base, ""))

    def test_release_preparation_passes_without_the_exemption(self):
        self.fragment("fix.md", "### Fixed\n\n- A fix.\n")
        self.base = self.commit("docs(changelog): add a fragment")
        MODULE.release(self.root, "13.0.0-rc.1", "2026-11-20")
        self.edit_changelog("## [13.0.0-rc.1] - 2026-11-20\n",
                            "## [13.0.0-rc.1] - 2026-11-20\n\nSummary.\n")
        self.commit("chore(release): prepare 13.0.0-rc.1")
        self.assertEqual(MODULE.unreleased_additions(self.root, self.base), [])

    def test_invalid_fragments_fail_even_on_release_branches(self):
        self.fragment("bad.md", "### Other\n\n- X.\n")
        self.commit("docs(changelog): bad fragment")
        with self.assertRaises(MODULE.ChangelogError):
            MODULE.check(self.root, self.base, "chore/v13-release-13.0.0-rc.1")

    def test_unknown_or_option_like_base_is_refused(self):
        with self.assertRaises(MODULE.ChangelogError) as refused:
            MODULE.check(self.root, "0" * 40, "")
        self.assertIn("is not a commit in this repository", str(refused.exception))
        for base in ("--output=/tmp/x", "-p", "", "main HEAD", "main\n"):
            with self.subTest(base=base), self.assertRaises(MODULE.ChangelogError) as refused:
                MODULE.check(self.root, base, "")
            self.assertIn("invalid --base revision", str(refused.exception))

    def test_base_may_be_a_branch_name_or_sha(self):
        self.edit_changelog("- Existing addition.\n", "- Existing addition.\n- New entry.\n")
        self.commit("docs(changelog): edit unreleased")
        for base in ("main~1", "HEAD^", self.base):
            with self.subTest(base=base):
                self.assertEqual(MODULE.unreleased_additions(self.root, base),
                                 ["CHANGELOG.md:16: - New entry."])


if __name__ == "__main__":
    unittest.main()
