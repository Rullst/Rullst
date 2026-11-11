#!/usr/bin/env python3
"""Keep the Cargo `target/` cache of setup-rust-toolchain safe to restore.

actions-rust-lang/setup-rust-toolchain archives `target/` with
Swatinem/rust-cache unless its step sets `cache: false`. Every such step must:

1. set `cache-on-failure: false`, so a failed or cancelled run (an interrupted
   build script, a half-written output directory) never saves a cache;
2. be followed by a step running `.github/repair-duckdb-sources.py` before any
   later step that runs Cargo with `--all-features` or a DuckDB feature: the
   archive cannot hold a usable libduckdb-sys source tree (see that script).

This reads the repository's block-style workflow layout, not arbitrary YAML,
and fails closed on a setup step it cannot attribute to a job's steps.

Usage: check-rust-cache-policy.py [WORKFLOW ...]
(default: every .github/workflows/*.yml)
"""

from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOOLCHAIN = "actions-rust-lang/setup-rust-toolchain@"
REPAIR = ".github/repair-duckdb-sources.py"
CARGO = re.compile(r"(?<![\w./-])cargo(?![\w-])")
DUCKDB_BUILD = re.compile(r"--all-features|duckdb")
KEY = re.compile(r"^(?P<key>[A-Za-z0-9_-]+)[ ]*:(?:[ ]+(?P<value>.*))?$")

_SPEC = importlib.util.spec_from_file_location(
    "validate_action_pins", Path(__file__).with_name("validate-action-pins.py"))
if _SPEC is None or _SPEC.loader is None:
    raise RuntimeError("cannot import validate-action-pins.py")
_PINS = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_PINS)


def indentation(line: str) -> int:
    return len(line) - len(line.lstrip(" "))


def scalar(value: str | None) -> str:
    value = (value or "").strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"":
        value = value[1:-1]
    return value.strip().lower()


def code_lines(text: str) -> list[tuple[int, str]]:
    """Numbered lines without comments or blank lines."""
    lines = []
    for number, line in enumerate(text.splitlines(), 1):
        line = _PINS.strip_yaml_comment(line)
        if line.strip():
            lines.append((number, line))
    return lines


def block(lines: list[tuple[int, str]], start: int, parent: int) -> int:
    """Index after the lines indented deeper than `parent` from `start`."""
    end = start
    while end < len(lines) and indentation(lines[end][1]) > parent:
        end += 1
    return end


def parse_step(lines: list[tuple[int, str]]) -> dict:
    first_number, first = lines[0]
    dash = indentation(first)
    lines = [(first_number, first[:dash] + "  " + first[dash + 2:]), *lines[1:]]
    key_indent = indentation(lines[0][1])
    step = {"line": first_number, "uses": "", "run": "", "with": {}}
    index = 0
    while index < len(lines):
        _, line = lines[index]
        end = block(lines, index + 1, key_indent)
        match = KEY.match(line.strip()) if indentation(line) == key_indent else None
        if match is not None and match["key"] == "uses":
            step["uses"] = scalar(match["value"])
        elif match is not None and match["key"] == "run":
            step["run"] = "\n".join([match["value"] or "", *(text for _, text in lines[index + 1:end])])
        elif match is not None and match["key"] == "with" and end > index + 1:
            option_indent = indentation(lines[index + 1][1])
            for _, option in lines[index + 1:end]:
                entry = KEY.match(option.strip()) if indentation(option) == option_indent else None
                if entry is not None:
                    step["with"][entry["key"]] = scalar(entry["value"])
        index = max(end, index + 1)
    return step


def job_steps(lines: list[tuple[int, str]]) -> list[dict]:
    for index, (_, line) in enumerate(lines):
        if line.strip() != "steps:":
            continue
        parent = indentation(line)
        end = index + 1
        while end < len(lines) and (indentation(lines[end][1]) > parent or (
                indentation(lines[end][1]) == parent and lines[end][1].lstrip().startswith("- "))):
            end += 1
        items = lines[index + 1:end]
        if not items or not items[0][1].lstrip().startswith("- "):
            return []
        dash = indentation(items[0][1])
        starts = [position for position, (_, text) in enumerate(items)
                  if indentation(text) == dash and text.lstrip().startswith("- ")]
        return [parse_step(items[start:stop])
                for start, stop in zip(starts, [*starts[1:], len(items)])]
    return []


def jobs(lines: list[tuple[int, str]]) -> list[tuple[str, list[tuple[int, str]]]]:
    starts = [index for index, (_, line) in enumerate(lines) if line == "jobs:"]
    if not starts:
        return []
    body = lines[starts[0] + 1:block(lines, starts[0] + 1, 0)]
    if not body:
        return []
    job_indent = indentation(body[0][1])
    heads = [index for index, (_, line) in enumerate(body) if indentation(line) == job_indent]
    return [(body[head][1].strip().rstrip(":"), body[head + 1:stop])
            for head, stop in zip(heads, [*heads[1:], len(body)])]


def check_text(name: str, text: str) -> list[str]:
    lines = code_lines(text)
    errors = []
    found = 0
    for job, job_lines in jobs(lines):
        steps = job_steps(job_lines)
        for index, step in enumerate(steps):
            if not step["uses"].startswith(TOOLCHAIN):
                continue
            found += 1
            where = f"{name}:{step['line']}: job {job}"
            options = step["with"]
            if options.get("cache", "true") == "false":
                continue
            if options.get("cache-on-failure") != "false":
                errors.append(f"{where}: setup-rust-toolchain caches target/ but would save it "
                              "from a failed or cancelled run; set `cache-on-failure: false`")
            repaired = False
            for later in steps[index + 1:]:
                run = later["run"]
                cargo = CARGO.search(run)
                repair = run.find(REPAIR)
                if repair != -1 and (cargo is None or repair < cargo.start()):
                    repaired = True
                if cargo is not None and DUCKDB_BUILD.search(run) and not repaired:
                    errors.append(f"{name}:{later['line']}: job {job}: run `python3 {REPAIR}` "
                                  "after the cached setup-rust-toolchain step and before Cargo "
                                  "builds with --all-features or a DuckDB feature")
                    break
    expected = sum(1 for _, line in lines
                   if re.match(r"^\s*(?:-\s+)?uses\s*:", line) and TOOLCHAIN in line)
    if found != expected:
        errors.append(f"{name}: {expected - found} setup-rust-toolchain step(s) outside a "
                      "readable job `steps:` list; review the cache policy by hand")
    return errors


def main(argv: list[str]) -> int:
    paths = [Path(arg) for arg in argv[1:]] or sorted((ROOT / ".github" / "workflows").glob("*.yml"))
    errors = []
    for path in paths:
        name = path.relative_to(ROOT) if path.is_relative_to(ROOT) else path
        errors += check_text(str(name), path.read_text(encoding="utf-8"))
    for error in errors:
        print(error, file=sys.stderr)
    if not errors:
        print(f"Rust cache policy holds for {len(paths)} workflow(s).")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
