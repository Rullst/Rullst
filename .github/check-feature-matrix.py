#!/usr/bin/env python3
"""Check the umbrella table of docs/src/feature-matrix.md against rullst/Cargo.toml.

Every public umbrella feature needs exactly one row, no row may name a feature
the manifest does not define, and both the Default column and the sentence
naming the default features must match `default`.
"""

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "rullst" / "Cargo.toml"
MATRIX = ROOT / "docs" / "src" / "feature-matrix.md"
SECTION = "## Umbrella crate: `rullst`"
ROW = re.compile(r"^\| `([a-z0-9-]+)` \| (yes|no) \|")
DEFAULT_SENTENCE = re.compile(r"The default `rullst` dependency enables (.*?):", re.DOTALL)


def umbrella_section(text: str) -> str:
    if SECTION not in text:
        raise SystemExit(f"{MATRIX.relative_to(ROOT)}: missing section {SECTION!r}")
    return text.split(SECTION, 1)[1].split("\n## ", 1)[0]


def umbrella_rows(section: str) -> tuple[dict[str, bool], list[str]]:
    rows: dict[str, bool] = {}
    duplicates = []
    for line in section.splitlines():
        match = ROW.match(line)
        if match:
            if match.group(1) in rows:
                duplicates.append(match.group(1))
            rows[match.group(1)] = match.group(2) == "yes"
    return rows, duplicates


def main() -> int:
    features = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))["features"]
    defaults = set(features.get("default", []))
    public = set(features) - {"default"}
    section = umbrella_section(MATRIX.read_text(encoding="utf-8"))
    rows, duplicates = umbrella_rows(section)

    errors = [f"missing umbrella row for `{name}`" for name in sorted(public - rows.keys())]
    errors += [f"row for `{name}`, which rullst/Cargo.toml does not define" for name in sorted(rows.keys() - public)]
    errors += [f"duplicate umbrella row for `{name}`" for name in duplicates]
    for name in sorted(public & rows.keys()):
        expected = name in defaults
        if rows[name] != expected:
            errors.append(f"Default column for `{name}` must be {'yes' if expected else 'no'}")
    sentence = DEFAULT_SENTENCE.search(section)
    named = set(re.findall(r"`([a-z0-9-]+)`", sentence.group(1))) if sentence else None
    if named != defaults:
        errors.append(f"the default-features sentence must name exactly {sorted(defaults)}")

    for error in errors:
        print(f"{MATRIX.relative_to(ROOT)}: {error}", file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
