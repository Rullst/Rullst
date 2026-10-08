#!/usr/bin/env python3
"""Render the GitHub release notes for one exact CHANGELOG version section.

The release page carries a summary, not the whole section: the section's
introduction (the text before its first `###` heading), its "Known limitations"
subsection when present, and a link to the complete section at the release tag.
This keeps release pages readable and far below GitHub's 125,000-character
limit. A section without an introduction falls back to the complete text, which
must then fit the limit.
"""

from __future__ import annotations

import os
import re
import sys
from pathlib import Path
from typing import NoReturn

LIMIT_BYTES = 125_000
LIMITATIONS = re.compile(r"^### Known limitations\s*$", re.MULTILINE)


def fail(message: str) -> NoReturn:
    raise SystemExit(f"release notes: {message}")


def github_anchor(heading: str) -> str:
    """GitHub's fragment for a Markdown heading (lowercase, punctuation dropped)."""
    text = heading.lstrip("#").strip().lower()
    text = re.sub(r"[^\w\- ]", "", text)
    return text.replace(" ", "-")


def subsection(body: str, start: re.Match[str]) -> str:
    rest = body[start.end() :]
    following = re.search(r"^###? ", rest, re.MULTILINE)
    end = start.end() + following.start() if following is not None else len(body)
    return body[start.start() : end].strip()


def render(changelog: str, version: str, repository: str) -> str:
    heading = re.compile(
        rf"^## \[{re.escape(version)}\] - [0-9]{{4}}-[0-9]{{2}}-[0-9]{{2}}(?: .*)?$",
        re.MULTILINE,
    )
    matches = list(heading.finditer(changelog))
    if len(matches) != 1:
        fail(
            f"expected exactly one dated CHANGELOG section for {version}; "
            f"found {len(matches)}"
        )

    match = matches[0]
    next_section = re.search(r"^## \[[^\]]+\]", changelog[match.end() :], re.MULTILINE)
    end = match.end() + next_section.start() if next_section is not None else len(changelog)
    section = changelog[match.start() : end].strip()
    body = changelog[match.end() : end]

    first_subsection = re.search(r"^### ", body, re.MULTILINE)
    introduction = (body[: first_subsection.start()] if first_subsection else body).strip()
    if not introduction:
        notes = section
    else:
        parts = [match.group(0).strip(), introduction]
        limitations = LIMITATIONS.search(body)
        if limitations is not None:
            parts.append(subsection(body, limitations))
        link = (
            f"https://github.com/{repository}/blob/v{version}/CHANGELOG.md"
            f"#{github_anchor(match.group(0))}"
        )
        parts.append(f"**Full changelog:** [every change in {version}]({link})")
        notes = "\n\n".join(parts)

    if len(notes.encode("utf-8")) > LIMIT_BYTES:
        fail(
            f"release notes for {version} exceed the 125 KiB limit; add an "
            "introduction before the section's first ### heading so the release "
            "page links to the full section instead"
        )
    return notes


def main() -> None:
    if len(sys.argv) != 3:
        fail("usage: render-release-notes.py VERSION OUTPUT")

    version, output_name = sys.argv[1:]
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?", version):
        fail(f"invalid semantic version: {version}")

    repository_root = Path(__file__).resolve().parent.parent
    changelog = (repository_root / "CHANGELOG.md").read_text(encoding="utf-8")
    repository = os.environ.get("GITHUB_REPOSITORY") or "Rullst/Rullst"
    notes = render(changelog, version, repository)

    output = Path(output_name)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(notes + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
