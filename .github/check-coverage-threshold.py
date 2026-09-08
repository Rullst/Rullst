#!/usr/bin/env python3
"""Enforce exact whole-repository and framework-library line coverage floors."""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
from typing import Any


FRAMEWORK_SOURCE = re.compile(
    r"(?:^|/)rullst(?:-(?:ai|auth|capital|connect|core|iot|mail|messaging|"
    r"nexus|orm|security|studio))?/src/"
)


def fail(message: str) -> None:
    print(f"coverage threshold check failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def line_totals(value: Any, label: str) -> tuple[int, int]:
    if not isinstance(value, dict):
        fail(f"{label} line totals are missing")
    count = value.get("count")
    covered = value.get("covered")
    if not isinstance(count, int) or not isinstance(covered, int):
        fail(f"{label} line totals must be integers")
    if count <= 0 or covered < 0 or covered > count:
        fail(f"{label} line totals are outside their valid range")
    return count, covered


def percentage(count: int, covered: int) -> str:
    return f"{covered * 100 / count:.4f}% ({covered}/{count})"


def require_floor(label: str, count: int, covered: int, minimum: int) -> None:
    print(f"{label}: {percentage(count, covered)}; required: >= {minimum}%")
    if covered * 100 < count * minimum:
        fail(f"{label} is below {minimum}%")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=pathlib.Path)
    parser.add_argument("--minimum", type=int, default=90)
    args = parser.parse_args()
    if not 1 <= args.minimum <= 100:
        fail("--minimum must be between 1 and 100")

    try:
        report = json.loads(args.report.read_text(encoding="utf-8"))
        data = report["data"]
        root = data[0]
        files = root["files"]
        totals = root["totals"]
    except (OSError, json.JSONDecodeError, KeyError, IndexError, TypeError) as error:
        fail(f"cannot read cargo-llvm-cov summary {args.report}: {error}")
    if not isinstance(data, list) or len(data) != 1 or not isinstance(files, list):
        fail("cargo-llvm-cov summary must contain one data entry and a file list")

    whole_count, whole_covered = line_totals(totals.get("lines"), "whole repository")

    framework_count = 0
    framework_covered = 0
    framework_files = 0
    for entry in files:
        if not isinstance(entry, dict) or not isinstance(entry.get("filename"), str):
            fail("coverage file entries must contain a filename")
        if FRAMEWORK_SOURCE.search(entry["filename"]) is None:
            continue
        summary = entry.get("summary")
        if not isinstance(summary, dict):
            fail(f"coverage summary is missing for {entry['filename']}")
        count, covered = line_totals(summary.get("lines"), entry["filename"])
        framework_count += count
        framework_covered += covered
        framework_files += 1

    if framework_files == 0:
        fail("no framework library source files matched the governed path set")

    require_floor("Whole repository", whole_count, whole_covered, args.minimum)
    require_floor(
        f"Framework libraries ({framework_files} files)",
        framework_count,
        framework_covered,
        args.minimum,
    )


if __name__ == "__main__":
    main()
