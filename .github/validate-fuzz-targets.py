#!/usr/bin/env python3
"""Keep the shared fuzz workflow matrix aligned with every fuzz manifest."""

from __future__ import annotations

import json
import pathlib
import sys
import tomllib


ROOT = pathlib.Path(__file__).resolve().parent.parent
MATRIX_PATH = ROOT / ".github" / "fuzz-targets.json"


def fail(message: str) -> None:
    print(f"fuzz target validation failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_declared() -> list[tuple[str, str]]:
    try:
        raw = json.loads(MATRIX_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {MATRIX_PATH.relative_to(ROOT)}: {error}")

    if not isinstance(raw, list):
        fail(".github/fuzz-targets.json must contain a JSON array")

    declared: list[tuple[str, str]] = []
    for index, item in enumerate(raw):
        if not isinstance(item, dict) or set(item) != {"dir", "target"}:
            fail(f"entry {index} must contain only string dir and target keys")
        directory = item.get("dir")
        target = item.get("target")
        if not isinstance(directory, str) or not isinstance(target, str):
            fail(f"entry {index} must contain only string dir and target values")
        declared.append((directory, target))
    return declared


def discover_manifests() -> set[tuple[str, str]]:
    discovered: set[tuple[str, str]] = set()
    manifests = sorted(ROOT.glob("*/fuzz/Cargo.toml"))
    if not manifests:
        fail("no */fuzz/Cargo.toml manifests were found")

    for manifest in manifests:
        try:
            cargo = tomllib.loads(manifest.read_text(encoding="utf-8-sig"))
        except (OSError, tomllib.TOMLDecodeError) as error:
            fail(f"cannot parse {manifest.relative_to(ROOT)}: {error}")

        directory = manifest.parent.relative_to(ROOT).as_posix()
        bins = cargo.get("bin", [])
        if not isinstance(bins, list) or not bins:
            fail(f"{manifest.relative_to(ROOT)} declares no [[bin]] fuzz targets")

        for item in bins:
            target = item.get("name") if isinstance(item, dict) else None
            if not isinstance(target, str) or not target:
                fail(f"{manifest.relative_to(ROOT)} has a [[bin]] without a name")
            source = manifest.parent / "fuzz_targets" / f"{target}.rs"
            if not source.is_file():
                fail(f"missing fuzz source {source.relative_to(ROOT)}")
            discovered.add((directory, target))
    return discovered


def main() -> None:
    declared_list = load_declared()
    declared = set(declared_list)
    if len(declared) != len(declared_list):
        fail(".github/fuzz-targets.json contains duplicate entries")

    discovered = discover_manifests()
    missing = sorted(discovered - declared)
    stale = sorted(declared - discovered)
    if missing or stale:
        if missing:
            print(f"missing matrix entries: {missing}", file=sys.stderr)
        if stale:
            print(f"stale matrix entries: {stale}", file=sys.stderr)
        raise SystemExit(1)

    print(
        f"validated {len(declared)} fuzz targets across "
        f"{len({directory for directory, _ in declared})} manifests"
    )


if __name__ == "__main__":
    main()
