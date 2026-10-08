#!/usr/bin/env python3
"""Regression tests for .github/render-release-notes.py."""

from __future__ import annotations

import importlib.util
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "render-release-notes.py"
spec = importlib.util.spec_from_file_location("render_release_notes", SCRIPT)
assert spec is not None and spec.loader is not None
notes_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notes_module)

CHANGELOG = """# Changelog

## [Unreleased]

## [13.0.0-alpha.1] - 2026-10-10

First preview of the v13 line. See the migration guide.

### Known limitations

- Experimental crates may change between alphas.

### Capital

- A long entry that belongs only in the full changelog.

## [12.3.0] - 2026-10-08

### Deprecated

- Only subsections, no introduction.
"""


def expect_failure(call) -> str:
    try:
        call()
    except SystemExit as error:
        return str(error)
    raise AssertionError("expected the renderer to refuse")


def main() -> None:
    summary = notes_module.render(CHANGELOG, "13.0.0-alpha.1", "Rullst/Rullst")
    assert summary.startswith("## [13.0.0-alpha.1] - 2026-10-10\n\nFirst preview"), summary
    assert "### Known limitations" in summary and "may change between alphas" in summary
    assert "A long entry" not in summary, "subsections other than limitations stay in CHANGELOG"
    assert (
        "https://github.com/Rullst/Rullst/blob/v13.0.0-alpha.1/CHANGELOG.md"
        "#1300-alpha1---2026-10-10" in summary
    ), summary

    full = notes_module.render(CHANGELOG, "12.3.0", "Rullst/Rullst")
    assert full.startswith("## [12.3.0] - 2026-10-08") and "Only subsections" in full
    assert "Full changelog" not in full, "a section without introduction is rendered whole"

    huge = CHANGELOG.replace("- Only subsections", "- " + "x" * 130_000)
    assert "125 KiB" in expect_failure(lambda: notes_module.render(huge, "12.3.0", "Rullst/Rullst"))

    duplicated = CHANGELOG + "\n## [12.3.0] - 2026-10-09\n"
    assert "exactly one" in expect_failure(
        lambda: notes_module.render(duplicated, "12.3.0", "Rullst/Rullst")
    )

    assert notes_module.github_anchor("## [12.3.0] - 2026-10-08") == "1230---2026-10-08"
    print("render-release-notes: 5 checks passed")


if __name__ == "__main__":
    main()
