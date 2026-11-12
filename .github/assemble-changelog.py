#!/usr/bin/env python3
"""Check changelog fragments and fold them into CHANGELOG.md at release preparation.

A pull request describes its change in its own file under `changelog.d/`
instead of editing the shared `## [Unreleased]` lines of CHANGELOG.md, so
parallel pull requests no longer conflict there. A fragment is
`changelog.d/<slug>.md` (lowercase letters, digits and hyphens); its first line
is one Keep a Changelog category heading and the rest are the entry's bullets,
written exactly as CHANGELOG.md entries (`changelog.d/README.md` has an
example).

    python3 .github/assemble-changelog.py --check [--base REV [--branch NAME]]
    python3 .github/assemble-changelog.py --release 13.0.0-rc.1 --date 2026-11-20

`--check` validates every fragment and the `## [Unreleased]` heading. With
`--base`, it also refuses lines that the commits since REV add under
`## [Unreleased]`, unless NAME is a release-preparation branch
(`chore/v<major>-release-<version>`).

`--release` validates everything before writing. It appends each category's
fragments, in file-name order, to that category's `[Unreleased]` subsection
(adding missing subsections in canonical order), opens the dated version
section below the `[Unreleased]` introduction, as release preparation did by
hand, and deletes the merged fragments. The summary and `### Known
limitations` are then written below the new heading.
"""

from __future__ import annotations

import argparse
import datetime
from pathlib import Path
import re
import subprocess
import sys
from typing import NamedTuple

CATEGORIES = ("Added", "Changed", "Deprecated", "Removed", "Fixed", "Security")
# Subsection order in a version section; any other title (Dependencies, ...) goes last.
RANK = {"Known limitations": 0, **{name: rank for rank, name in enumerate(CATEGORIES, 1)}}
OTHER_RANK = len(RANK)
FRAGMENTS = "changelog.d"
README = "README.md"
CHANGELOG = "CHANGELOG.md"
UNRELEASED = "## [Unreleased]"
NAME = re.compile(r"[0-9a-z][0-9a-z-]*\.md")
HEADING = re.compile(r"#{1,6}(?:[ \t]|$)")
FENCE = re.compile(r"[ \t]*(?:```|~~~)")
HUNK = re.compile(r"@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@", re.ASCII)
RELEASE_BRANCH = re.compile(r"chore/v\d+-release-.+", re.ASCII)
VERSION = re.compile(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?", re.ASCII)
DATE = re.compile(r"\d{4}-\d{2}-\d{2}", re.ASCII)
# A commit name or SHA for --base; never an option.
REVISION = re.compile(r"\w[\w.@{}~^/-]*", re.ASCII)
COMMIT = re.compile(r"[0-9a-f]{40}|[0-9a-f]{64}")


class ChangelogError(Exception):
    """A fragment, CHANGELOG.md or argument that the release process refuses."""


class Fragment(NamedTuple):
    path: Path
    category: str
    lines: tuple[str, ...]


class Unreleased(NamedTuple):
    start: int  # index of the `## [Unreleased]` line
    end: int  # index of the next `## ` heading, or the number of lines
    preamble: tuple[str, ...]
    sections: tuple[tuple[str, tuple[str, ...]], ...]


def trimmed(lines) -> list[str]:
    lines = list(lines)
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip():
        lines.pop()
    return lines


def parse_fragment(path: Path) -> Fragment:
    name = f"{FRAGMENTS}/{path.name}"
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise ChangelogError(f"{name}: cannot read: {error}") from error
    lines = [line.rstrip() for line in text.splitlines()]
    while lines and not lines[-1]:
        lines.pop()
    if not lines:
        raise ChangelogError(f"{name}: is empty")
    heading = re.fullmatch(r"### (.+)", lines[0])
    if heading is None or heading[1] not in CATEGORIES:
        allowed = ", ".join(f"`### {category}`" for category in CATEGORIES)
        raise ChangelogError(f"{name}:1: the first line must be one of {allowed}")
    body = trimmed(lines[1:])
    if not body:
        raise ChangelogError(f"{name}: has no entry below its heading")
    if not body[0].startswith("- "):
        raise ChangelogError(f"{name}: the entry must start with a `- ` bullet")
    check_entry_lines(name, body, len(lines) - len(body) + 1)
    return Fragment(path, heading[1], tuple(body))


def check_entry_lines(name: str, body: list[str], first_number: int) -> None:
    """Bullets with indented continuations; no heading outside a code fence."""
    fenced = False
    for number, line in enumerate(body, first_number):
        if not fenced and HEADING.match(line.lstrip()):
            raise ChangelogError(f"{name}:{number}: only the first line may be a heading")
        if line and not line.startswith(("- ", " ", "\t")):
            raise ChangelogError(
                f"{name}:{number}: start each entry with `- ` and indent its continuation lines"
            )
        if FENCE.match(line):
            fenced = not fenced
    if fenced:
        raise ChangelogError(f"{name}: a code fence is not closed")


def load_fragments(root: Path) -> list[Fragment]:
    """Every fragment, sorted by file name; all problems are reported together."""
    directory = root / FRAGMENTS
    if not directory.is_dir():
        return []
    fragments: list[Fragment] = []
    problems: list[str] = []
    for path in sorted(directory.iterdir(), key=lambda entry: entry.name):
        if path.name == README:
            continue
        if path.is_symlink() or not path.is_file() or NAME.fullmatch(path.name) is None:
            problems.append(
                f"{FRAGMENTS}/{path.name}: a fragment is a regular file named <slug>.md "
                "(lowercase letters, digits and hyphens)"
            )
            continue
        try:
            fragments.append(parse_fragment(path))
        except ChangelogError as error:
            problems.append(str(error))
    if problems:
        raise ChangelogError("\n".join(problems))
    return fragments


def parse_unreleased(lines: list[str]) -> Unreleased:
    starts = [index for index, line in enumerate(lines) if line.rstrip() == UNRELEASED]
    if len(starts) != 1:
        raise ChangelogError(
            f"{CHANGELOG} must have exactly one `{UNRELEASED}` heading; found {len(starts)}"
        )
    start = starts[0]
    end = next(
        (index for index in range(start + 1, len(lines)) if lines[index].startswith("## ")),
        len(lines),
    )
    preamble: list[str] = []
    sections: list[tuple[str, list[str]]] = []
    for line in lines[start + 1 : end]:
        if line.startswith("### "):
            sections.append((line[4:].strip(), []))
        elif sections:
            sections[-1][1].append(line)
        else:
            preamble.append(line)
    return Unreleased(
        start, end, tuple(preamble), tuple((title, tuple(body)) for title, body in sections)
    )


def validate_release(version: str, date: str) -> None:
    if VERSION.fullmatch(version) is None:
        raise ChangelogError(f"invalid semantic version: {version}")
    try:
        valid_date = DATE.fullmatch(date) is not None and datetime.date.fromisoformat(date)
    except ValueError:
        valid_date = False
    if not valid_date:
        raise ChangelogError(f"invalid release date (expected YYYY-MM-DD): {date}")


def assemble(changelog: str, fragments: list[Fragment], version: str, date: str) -> str:
    """CHANGELOG.md with the fragments merged and `[Unreleased]` opened as `version`."""
    validate_release(version, date)
    lines = changelog.splitlines()
    if any(line.startswith(f"## [{version}]") for line in lines):
        raise ChangelogError(f"{CHANGELOG} already has a section for {version}")
    unreleased = parse_unreleased(lines)
    sections = [
        (title, trimmed(body)) for title, body in unreleased.sections if trimmed(body)
    ]
    for category in CATEGORIES:
        entries = [line for fragment in fragments if fragment.category == category
                   for line in fragment.lines]
        if not entries:
            continue
        existing = [index for index, (title, _) in enumerate(sections) if title == category]
        if existing:
            sections[existing[-1]][1].extend(entries)
            continue
        position = next(
            (index for index, (title, _) in enumerate(sections)
             if RANK.get(title, OTHER_RANK) > RANK[category]),
            len(sections),
        )
        sections.insert(position, (category, entries))
    if not sections:
        raise ChangelogError(
            f"nothing to release: `{UNRELEASED}` has no entries and {FRAGMENTS}/ has no fragments"
        )
    output = lines[: unreleased.start + 1] + [""]
    preamble = trimmed(unreleased.preamble)
    if preamble:
        output += preamble + [""]
    output += [f"## [{version}] - {date}", ""]
    for title, body in sections:
        output += [f"### {title}", "", *body, ""]
    output += lines[unreleased.end :]
    return "\n".join(output).rstrip("\n") + "\n"


def release(root: Path, version: str, date: str) -> str:
    fragments = load_fragments(root)
    path = root / CHANGELOG
    try:
        changelog = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise ChangelogError(f"cannot read {CHANGELOG}: {error}") from error
    assembled = assemble(changelog, fragments, version, date)
    path.write_text(assembled, encoding="utf-8")
    for fragment in fragments:
        fragment.path.unlink()
    counts = ", ".join(
        f"{category} {sum(fragment.category == category for fragment in fragments)}"
        for category in CATEGORIES
        if any(fragment.category == category for fragment in fragments)
    )
    return (
        f"merged {len(fragments)} fragment(s) into {version}"
        + (f" ({counts})" if counts else "")
        + f"; write the summary and `### Known limitations` below `## [{version}] - {date}`"
    )


def git(root: Path, *args: str, stdin: str | None = None) -> str:
    try:
        return subprocess.run(
            ["git", *args], cwd=root, input=stdin, check=True, capture_output=True, text=True
        ).stdout
    except (OSError, subprocess.CalledProcessError) as error:
        detail = getattr(error, "stderr", "") or str(error)
        raise ChangelogError(f"git {' '.join(args)} failed: {detail.strip()}") from error


def unreleased_additions(root: Path, base: str) -> list[str]:
    """Non-blank lines that HEAD adds, relative to `base`, under `## [Unreleased]`."""
    if not REVISION.fullmatch(base):
        raise ChangelogError(f"invalid --base revision: {base!r}")
    # The revision reaches git on standard input, never as an argument.
    resolved = git(root, "cat-file", "--batch-check", stdin=f"{base}^{{commit}}\n").split()
    if len(resolved) != 3 or resolved[1] != "commit" or not COMMIT.fullmatch(resolved[0]):
        raise ChangelogError(f"--base {base} is not a commit in this repository")
    commit = resolved[0]
    diff = git(root, "diff", "--unified=0", "--no-color", "--no-ext-diff", commit, "HEAD",
               "--", CHANGELOG)
    added: set[int] = set()
    for line in diff.splitlines():
        hunk = HUNK.match(line)
        if hunk is not None:
            first = int(hunk[1])
            added.update(range(first, first + int(hunk[2] or 1)))
    if not added:
        return []
    lines = git(root, "show", f"HEAD:{CHANGELOG}").splitlines()
    unreleased = parse_unreleased(lines)
    return [
        f"{CHANGELOG}:{number}: {lines[number - 1]}"
        for number in sorted(added)
        if unreleased.start + 1 < number <= unreleased.end and lines[number - 1].strip()
    ]


def check(root: Path, base: str | None, branch: str) -> str:
    fragments = load_fragments(root)
    try:
        parse_unreleased((root / CHANGELOG).read_text(encoding="utf-8").splitlines())
    except (OSError, UnicodeError) as error:
        raise ChangelogError(f"cannot read {CHANGELOG}: {error}") from error
    summary = f"{len(fragments)} changelog fragment(s) valid"
    if base is None:
        return summary
    if RELEASE_BRANCH.fullmatch(branch):
        return f"{summary}; {branch} is a release-preparation branch and may edit `{UNRELEASED}`"
    additions = unreleased_additions(root, base)
    if additions:
        raise ChangelogError(
            f"this change adds lines under `{UNRELEASED}` in {CHANGELOG}:\n"
            + "\n".join(additions)
            + f"\nPut the entry in a fragment under {FRAGMENTS}/ instead "
            f"(see {FRAGMENTS}/{README}); only release preparation edits `{UNRELEASED}`."
        )
    return f"{summary}; no lines added under `{UNRELEASED}`"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true", help="validate without writing")
    mode.add_argument("--release", metavar="VERSION", help="merge the fragments into VERSION")
    parser.add_argument("--date", help="release date for --release, YYYY-MM-DD")
    parser.add_argument("--base", metavar="REV",
                        help="with --check: refuse lines added under [Unreleased] since REV")
    parser.add_argument("--branch", default="",
                        help="with --base: the head branch; release-preparation branches are exempt")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args(argv)
    if (args.release is None) != (args.date is None):
        parser.error("--release and --date go together")
    if args.base is None and args.branch:
        parser.error("--branch needs --base")
    if args.release is not None and args.base is not None:
        parser.error("--base goes with --check")
    try:
        if args.check:
            print(f"assemble-changelog: {check(args.root, args.base, args.branch)}")
        else:
            print(f"assemble-changelog: {release(args.root, args.release, args.date)}")
    except ChangelogError as error:
        print(f"assemble-changelog: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
