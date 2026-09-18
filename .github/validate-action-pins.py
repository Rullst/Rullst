#!/usr/bin/env python3
"""Enforce a fail-closed, canonical syntax for GitHub Actions `uses` values."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


CANONICAL_USES = re.compile(
    r"^(?P<indent>[ ]*)(?:-[ ]+)?uses[ ]*:[ ]*(?P<value>.*?)[ ]*$"
)
OTHER_USES_KEY = re.compile(
    r"(?:^|[{,])[ ]*(?:-[ ]*)?[\"']?uses[\"']?[ ]*:"
)
EXPLICIT_MAPPING_KEY = re.compile(r"^[ ]*(?:-[ ]+)?\?[ ]")
MERGE_MAPPING_KEY = re.compile(r"(?:^|[{,])[ ]*<<[ ]*:")
YAML_REFERENCE = re.compile(
    r"(?:^|[\s:\[,{])(?:&|\*)[A-Za-z0-9_-]+(?:$|[\s,\]}])"
)
YAML_TAG = re.compile(r"(?:^|[:\[,{])[ ]*![A-Za-z0-9_!:/.-]+")
BLOCK_SCALAR = re.compile(
    r"^[ ]*(?:-[ ]+)?[^:#][^:]*:[ ]*[>|](?:[1-9][+-]?|[+-][1-9]?|[+-]?)[ ]*$"
)
REMOTE_ACTION = re.compile(
    r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*@[0-9A-Fa-f]{40}$"
)
LOCAL_ACTION = re.compile(r"^\./[A-Za-z0-9_.@/-]+$")


def strip_yaml_comment(line: str) -> str:
    """Remove a YAML comment while preserving hashes inside quoted scalars."""

    quote: str | None = None
    escaped = False
    index = 0
    while index < len(line):
        character = line[index]
        if quote == '"':
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == quote:
                quote = None
        elif quote == "'":
            if character == quote:
                if index + 1 < len(line) and line[index + 1] == quote:
                    index += 1
                else:
                    quote = None
        elif character in {'"', "'"}:
            quote = character
        elif character == "#" and (index == 0 or line[index - 1].isspace()):
            return line[:index].rstrip()
        index += 1
    return line.rstrip()


def decode_scalar(value: str) -> str | None:
    """Decode the simple scalar forms allowed by this repository policy."""

    if not value:
        return None
    if value.startswith('"'):
        try:
            decoded = json.loads(value)
        except (json.JSONDecodeError, TypeError):
            return None
        return decoded if isinstance(decoded, str) else None
    if value.startswith("'"):
        if len(value) < 2 or not value.endswith("'"):
            return None
        return value[1:-1].replace("''", "'")
    if value[-1:] in {'"', "'"}:
        return None
    return value


def has_quoted_mapping_key(line: str) -> bool:
    """Reject quoted keys, including YAML escape spellings of `uses`."""

    starts = [0]
    starts.extend(index + 1 for index, character in enumerate(line) if character in "{,")
    for start in starts:
        candidate = line[start:].lstrip()
        if candidate.startswith("- "):
            candidate = candidate[2:].lstrip()
        if not candidate or candidate[0] not in {'"', "'"}:
            continue

        quote = candidate[0]
        index = 1
        while index < len(candidate):
            character = candidate[index]
            if quote == '"' and character == "\\":
                index += 2
                continue
            if character == quote:
                if quote == "'" and index + 1 < len(candidate) and candidate[index + 1] == quote:
                    index += 2
                    continue
                return candidate[index + 1 :].lstrip().startswith(":")
            index += 1
    return False


def validate_text(source: str, display_name: str = "<memory>") -> list[str]:
    errors: list[str] = []
    block_indent: int | None = None
    codeql_pins: set[str] = set()

    for line_number, raw_line in enumerate(source.splitlines(), start=1):
        if "\t" in raw_line[: len(raw_line) - len(raw_line.lstrip())]:
            errors.append(f"{display_name}:{line_number}: tabs are not valid indentation")
            continue

        indentation = len(raw_line) - len(raw_line.lstrip(" "))
        if block_indent is not None:
            if not raw_line.strip() or indentation > block_indent:
                continue
            block_indent = None

        code = strip_yaml_comment(raw_line)
        if not code.strip():
            continue

        if has_quoted_mapping_key(code):
            errors.append(
                f"{display_name}:{line_number}: quoted YAML mapping keys are rejected"
            )
            continue
        if EXPLICIT_MAPPING_KEY.match(code) is not None:
            errors.append(
                f"{display_name}:{line_number}: explicit YAML mapping keys are rejected"
            )
            continue
        if MERGE_MAPPING_KEY.search(code) is not None:
            errors.append(
                f"{display_name}:{line_number}: YAML merge keys are rejected"
            )
            continue
        if YAML_REFERENCE.search(code) is not None or YAML_TAG.search(code) is not None:
            errors.append(
                f"{display_name}:{line_number}: YAML anchors, aliases, and tags are rejected"
            )
            continue

        match = CANONICAL_USES.fullmatch(code)
        if match is not None:
            scalar = decode_scalar(match.group("value"))
            if scalar is None:
                errors.append(
                    f"{display_name}:{line_number}: uses must be one non-empty scalar"
                )
            elif LOCAL_ACTION.fullmatch(scalar) is not None:
                pass
            elif REMOTE_ACTION.fullmatch(scalar) is None:
                errors.append(
                    f"{display_name}:{line_number}: remote uses value must end in "
                    "exactly one 40-character commit SHA"
                )
            elif scalar.lower().startswith("github/codeql-action/"):
                codeql_pins.add(scalar.rsplit("@", 1)[1].lower())
            continue

        if OTHER_USES_KEY.search(code) is not None:
            errors.append(
                f"{display_name}:{line_number}: uses must use a standalone canonical "
                "mapping line; flow mappings and quoted keys are rejected"
            )
            continue

        if BLOCK_SCALAR.fullmatch(code) is not None:
            block_indent = indentation

    if len(codeql_pins) > 1:
        errors.append(
            f"{display_name}: CodeQL sub-actions must use the same commit SHA"
        )
    return errors


def workflow_files(root: Path) -> list[Path]:
    return sorted(
        path
        for path in root.rglob("*")
        if path.is_file() and path.suffix.lower() in {".yml", ".yaml"}
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "root",
        nargs="?",
        type=Path,
        default=Path(".github/workflows"),
        help="workflow file or directory (default: .github/workflows)",
    )
    args = parser.parse_args()

    paths = [args.root] if args.root.is_file() else workflow_files(args.root)
    if not paths:
        print(f"action pins: no workflow files found under {args.root}", file=sys.stderr)
        return 1

    errors: list[str] = []
    for path in paths:
        try:
            source = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            errors.append(f"{path}: cannot read workflow: {error}")
            continue
        errors.extend(validate_text(source, str(path)))

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1

    print(f"validated immutable Action pins in {len(paths)} workflow files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
