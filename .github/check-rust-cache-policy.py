#!/usr/bin/env python3
"""Keep the Cargo `target/` cache of setup-rust-toolchain safe to restore.

actions-rust-lang/setup-rust-toolchain archives `target/` with
Swatinem/rust-cache unless its step sets `cache: false`. Every such step must
set `cache-on-failure: false`, so a failed or cancelled run (an interrupted
build script, a half-written output directory) never saves a cache.

This reads the repository's block-style workflow layout, not arbitrary YAML,
and fails closed on a setup step it cannot attribute to a job's steps. It
checks every `.github/workflows/*.yml` file and takes no arguments.
"""

from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOOLCHAIN = "actions-rust-lang/setup-rust-toolchain@"
KEY = re.compile(r"(?P<key>[A-Za-z0-9_-]+) *:(?P<value>.*)")
USES = re.compile(r"\s*(?:- +)?uses *:")

_SPEC = importlib.util.spec_from_file_location(
    "validate_action_pins", Path(__file__).with_name("validate-action-pins.py"))
if _SPEC is None or _SPEC.loader is None:
    raise RuntimeError("cannot import validate-action-pins.py")
_PINS = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_PINS)

Lines = list[tuple[int, str]]


def indentation(line: str) -> int:
    return len(line) - len(line.lstrip(" "))


def scalar(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"":
        value = value[1:-1]
    return value.strip().lower()


def key_at(line: str, indent: int) -> re.Match[str] | None:
    """The `key: value` pair written at exactly `indent`, if any."""
    if indentation(line) != indent:
        return None
    return KEY.fullmatch(line.strip())


def code_lines(text: str) -> Lines:
    """Numbered lines without comments or blank lines."""
    lines = []
    for number, line in enumerate(text.splitlines(), 1):
        line = _PINS.strip_yaml_comment(line)
        if line.strip():
            lines.append((number, line))
    return lines


def block_end(lines: Lines, start: int, parent: int) -> int:
    """Index after the lines indented deeper than `parent` from `start`."""
    end = start
    while end < len(lines) and indentation(lines[end][1]) > parent:
        end += 1
    return end


def options(lines: Lines) -> dict[str, str]:
    """The top-level entries of a `with:` mapping body."""
    indent = indentation(lines[0][1]) if lines else 0
    entries = (key_at(line, indent) for _, line in lines)
    return {entry["key"]: scalar(entry["value"]) for entry in entries if entry is not None}


def parse_step(lines: Lines) -> dict:
    number, first = lines[0]
    dash = indentation(first)
    lines = [(number, first[:dash] + "  " + first[dash + 2:]), *lines[1:]]
    key_indent = indentation(lines[0][1])
    step = {"line": number, "uses": "", "with": {}}
    index = 0
    while index < len(lines):
        end = max(block_end(lines, index + 1, key_indent), index + 1)
        match = key_at(lines[index][1], key_indent)
        key = match["key"] if match is not None else ""
        if key == "uses":
            step["uses"] = scalar(match["value"])
        elif key == "with":
            step["with"] = options(lines[index + 1:end])
        index = end
    return step


def step_items(lines: Lines, steps_key: int) -> Lines:
    """The list below a `steps:` key, whose dashes may share its indentation."""
    parent = indentation(lines[steps_key][1])
    end = steps_key + 1
    while end < len(lines):
        line = lines[end][1]
        if indentation(line) < parent or (
                indentation(line) == parent and not line.lstrip().startswith("- ")):
            break
        end += 1
    return lines[steps_key + 1:end]


def job_steps(lines: Lines) -> list[dict]:
    keys = [index for index, (_, line) in enumerate(lines) if line.strip() == "steps:"]
    items = step_items(lines, keys[0]) if keys else []
    if not items or not items[0][1].lstrip().startswith("- "):
        return []
    dash = indentation(items[0][1])
    starts = [position for position, (_, text) in enumerate(items)
              if indentation(text) == dash and text.lstrip().startswith("- ")]
    return [parse_step(items[start:stop]) for start, stop in zip(starts, [*starts[1:], len(items)])]


def jobs(lines: Lines) -> list[tuple[str, Lines]]:
    starts = [index for index, (_, line) in enumerate(lines) if line == "jobs:"]
    body = lines[starts[0] + 1:block_end(lines, starts[0] + 1, 0)] if starts else []
    if not body:
        return []
    job_indent = indentation(body[0][1])
    heads = [index for index, (_, line) in enumerate(body) if indentation(line) == job_indent]
    return [(body[head][1].strip().rstrip(":"), body[head + 1:stop])
            for head, stop in zip(heads, [*heads[1:], len(body)])]


def check_toolchain(name: str, job: str, step: dict) -> list[str]:
    if step["with"].get("cache", "true") == "false":
        return []
    if step["with"].get("cache-on-failure") != "false":
        return [f"{name}:{step['line']}: job {job}: setup-rust-toolchain caches target/ "
                "but would save it from a failed or cancelled run; set "
                "`cache-on-failure: false`"]
    return []


def check_text(name: str, text: str) -> list[str]:
    lines = code_lines(text)
    errors = []
    found = 0
    for job, job_lines in jobs(lines):
        for step in job_steps(job_lines):
            if step["uses"].startswith(TOOLCHAIN):
                found += 1
                errors += check_toolchain(name, job, step)
    expected = sum(1 for _, line in lines if USES.match(line) and TOOLCHAIN in line)
    if found != expected:
        errors.append(f"{name}: {expected - found} setup-rust-toolchain step(s) outside a "
                      "readable job `steps:` list; review the cache policy by hand")
    return errors


def main() -> int:
    paths = sorted((ROOT / ".github" / "workflows").glob("*.yml"))
    errors = []
    for path in paths:
        errors += check_text(str(path.relative_to(ROOT)), path.read_text(encoding="utf-8"))
    for error in errors:
        print(error, file=sys.stderr)
    if not errors:
        print(f"Rust cache policy holds for {len(paths)} workflow(s).")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
