#!/usr/bin/env python3
"""Check the Brazilian Portuguese translations of the first-hour pages, Scope and Limits and the classroom guide.

Every page under docs/src/pt-BR/ translates an English source named in
TRANSLATIONS: a whole page, or one section of README.md. The check fails when

- a page under docs/src/pt-BR/ is not in TRANSLATIONS, or a listed one is missing;
- its fenced code blocks differ from the English source's, byte for byte and
  in order, so the English doctests and first-hour checks cover them too;
- the note under its title does not link the English source and name the full
  commit it was translated from;
- the English source no longer links to the translation, or SUMMARY.md does
  not list it.

With the noted commit in the local history (not in a shallow clone), it also
prints a notice, without failing, for each translation whose English source
changed since that commit: its prose may be out of date.

Usage: check-translations.py [--root DIR]
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
TRANSLATED = Path("docs/src/pt-BR")
BOOK = "https://rullst.github.io/Rullst/book/"
# Translation -> (English source, the source's section heading or None for the
# whole page, the link to the translation the English source carries).
TRANSLATIONS: dict[str, tuple[str, str | None, str]] = {
    "docs/src/pt-BR/index.md": ("README.md", "## ⚡ Rullst in 30 seconds", BOOK + "pt-BR/index.html"),
    "docs/src/pt-BR/1-getting-started.md": ("docs/src/1-getting-started.md", None, "pt-BR/1-getting-started.md"),
    "docs/src/pt-BR/scope-and-limits.md": ("docs/src/scope-and-limits.md", None, "pt-BR/scope-and-limits.md"),
    "docs/src/pt-BR/iot-classroom.md": ("docs/src/iot-classroom.md", None, "pt-BR/iot-classroom.md"),
    **{
        f"docs/src/pt-BR/tutorials/{name}.md": (f"docs/src/tutorials/{name}.md", None, f"../pt-BR/tutorials/{name}.md")
        for name in ("01-hello-world", "rest-api-quickstart", "02-cli-generators", "03-active-record-crud",
                     "05-migrations-and-seeds", "11-authentication-system")
    },
}
FENCE = re.compile(r"^(?P<indent> *)(?P<fence>`{3,}|~{3,})")
NOTE = re.compile(r"^> \*\*Tradução\*\* .*\]\((?P<source>[^)\s]+)\).*\[`(?P<short>[0-9a-f]{7,40})`\]"
                  r"\(https://github\.com/Rullst/Rullst/commit/(?P<commit>[0-9a-f]{40})\)")


class Failure(Exception):
    """A translation does not match its English source."""


def code_blocks(text: str) -> list[list[str]]:
    """The fenced blocks of a Markdown text, each with its fence lines, as written."""
    lines, blocks, index = text.split("\n"), [], 0
    while index < len(lines):
        opening = FENCE.match(lines[index])
        if not opening:
            index += 1
            continue
        fence, end = opening["fence"], index + 1
        while end < len(lines) and not (lines[end].strip().startswith(fence)
                                         and set(lines[end].strip()) == {fence[0]}):
            end += 1
        blocks.append(lines[index:end + 1])
        index = end + 1
    return blocks


def section(text: str, heading: str | None) -> str:
    """The whole text, or the lines from `heading` to the next heading of its level or above."""
    if heading is None:
        return text
    lines = text.split("\n")
    if heading not in lines:
        raise Failure(f"the English source has no line `{heading}`")
    start = lines.index(heading)
    level = len(heading) - len(heading.lstrip("#"))
    end = next((index for index in range(start + 1, len(lines))
                if re.match(rf"^#{{1,{level}}} ", lines[index])), len(lines))
    return "\n".join(lines[start:end])


def first_difference(english: list[list[str]], translated: list[list[str]]) -> str | None:
    for number, (left, right) in enumerate(zip(english, translated), start=1):
        if left != right:
            line = next(index for index, pair in enumerate(zip(left + [""], right + [""])) if pair[0] != pair[1])
            return (f"code block {number} differs at its line {line + 1}: English "
                    f"{(left + [''])[line]!r}, translation {(right + [''])[line]!r}")
    if len(english) != len(translated):
        return f"{len(translated)} code blocks, the English source has {len(english)}"
    return None


def check_page(root: Path, page: str, summary: str) -> tuple[str, str] | None:
    """Fails on a mismatch; returns the English source and the noted commit."""
    source, heading, link = TRANSLATIONS[page]
    text = (root / page).read_text(encoding="utf-8")
    english = (root / source).read_text(encoding="utf-8")
    difference = first_difference(code_blocks(section(english, heading)), code_blocks(text))
    if difference:
        raise Failure(f"{page}: {difference}; copy the English blocks unchanged")
    lines = text.split("\n")
    note = NOTE.match(lines[2]) if len(lines) > 2 and lines[0].startswith("# ") and not lines[1] else None
    expected = os.path.relpath(root / source, (root / page).parent).replace(os.sep, "/")
    if not note or note["source"].split("#")[0] != expected or not note["commit"].startswith(note["short"]):
        raise Failure(f"{page}: the line under the title must be the translation note linking "
                      f"`{expected}` and the commit it was translated from (`[`<sha>`](…/commit/<full sha>)`)")
    if f"]({link})" not in english:
        raise Failure(f"{source} no longer links its translation `{link}`")
    if f"]({Path(page).relative_to('docs/src').as_posix()})" not in summary:
        raise Failure(f"docs/src/SUMMARY.md does not list {page}")
    return source, note["commit"]


def stale_notice(root: Path, page: str, source: str, commit: str) -> str | None:
    """A notice when the English source (its section) changed since `commit`,
    apart from the link to the translation, if git knows that commit."""
    shown = subprocess.run(["git", "show", f"{commit}:{source}"], cwd=root, capture_output=True, text=True)
    if shown.returncode != 0:
        return None
    _, heading, link = TRANSLATIONS[page]

    def content(text: str) -> str:
        try:
            lines = section(text, heading).split("\n")
        except Failure:
            return ""
        kept = "\n".join(line for line in lines if f"]({link})" not in line)
        return re.sub(r"\n{3,}", "\n\n", kept)

    if content(shown.stdout) == content((root / source).read_text(encoding="utf-8")):
        return None
    return (f"::notice title=Translation may be out of date::{page}: {source} changed since "
            f"{commit[:9]} (git diff {commit[:9]} -- {source})")


def check(root: Path) -> list[str]:
    """Fails on the first mismatch; returns the notices of possibly stale translations."""
    present = {path.relative_to(root).as_posix() for path in (root / TRANSLATED).rglob("*.md")}
    unlisted, missing = sorted(present - set(TRANSLATIONS)), sorted(set(TRANSLATIONS) - present)
    if unlisted or missing:
        raise Failure(f"translations not in TRANSLATIONS: {unlisted}; listed but missing: {missing}")
    summary = (root / "docs/src/SUMMARY.md").read_text(encoding="utf-8")
    notices = []
    for page in sorted(TRANSLATIONS):
        source, commit = check_page(root, page, summary)
        notice = stale_notice(root, page, source, commit)
        if notice:
            notices.append(notice)
    return notices


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args(argv)
    try:
        notices = check(args.root.resolve())
    except Failure as error:
        print(f"::error title=Brazilian Portuguese translations::{error}")
        return 1
    for notice in notices:
        print(notice)
    print(f"{len(TRANSLATIONS)} Brazilian Portuguese pages: code blocks identical to the English sources, "
          f"translation notes and links present")
    return 0


if __name__ == "__main__":
    sys.exit(main())
