#!/usr/bin/env python3
"""Extract one exact version section from CHANGELOG.md for a GitHub release."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from typing import NoReturn


def fail(message: str) -> NoReturn:
    raise SystemExit(f"release notes: {message}")


def main() -> None:
    if len(sys.argv) != 3:
        fail("usage: render-release-notes.py VERSION OUTPUT")

    version, output_name = sys.argv[1:]
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?", version):
        fail(f"invalid semantic version: {version}")

    repository = Path(__file__).resolve().parent.parent
    changelog_path = repository / "CHANGELOG.md"
    changelog = changelog_path.read_text(encoding="utf-8")
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

    start = matches[0].start()
    next_section = re.search(r"^## \[[^\]]+\]", changelog[matches[0].end() :], re.MULTILINE)
    end = (
        matches[0].end() + next_section.start()
        if next_section is not None
        else len(changelog)
    )
    notes = changelog[start:end].strip()
    if len(notes.encode("utf-8")) > 125_000:
        fail(f"CHANGELOG section for {version} exceeds the 125 KiB release-note limit")

    output = Path(output_name)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(notes + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
