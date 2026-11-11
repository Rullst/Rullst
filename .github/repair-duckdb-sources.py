#!/usr/bin/env python3
"""Remove DuckDB source trees that a restored Rust cache left incomplete.

libduckdb-sys (1.20000.1 and later) unpacks its bundled sources once into
`<target>/[<triple>/]duckdb-sources/<key>`: it extracts into a staging
directory, writes a `.complete` marker and renames the staging directory onto
`<key>`. A tree without that marker is never reused, but the rename cannot
replace a non-empty directory, so the build script fails with
`DirectoryNotEmpty` (errno 39 on Linux).

The `target/` archive of Swatinem/rust-cache (enabled by default in
actions-rust-lang/setup-rust-toolchain) produces exactly such trees: its
save-time cleanup does not recognise `duckdb-sources` as a Cargo profile, so
it deletes every file below it, the `.complete` marker included, and keeps
the directories. A later run that has to rerun the libduckdb-sys build script
(after a lockfile change to one of its build dependencies, for example) then
fails. Run this after the cache is restored and before Cargo builds: it
deletes every entry of a `duckdb-sources` directory that is not a complete
tree, interrupted staging directories included, and keeps complete trees.

It repairs the repository's `target/`, the directory the cache restores, and
takes no arguments.
"""

from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path

TARGET = Path(__file__).resolve().parents[1] / "target"
SOURCES = "duckdb-sources"
COMPLETE = ".complete"
STAGING = ".duckdb-unpack-"


def source_roots(target: Path) -> list[Path]:
    """`target/duckdb-sources` and `target/<triple>/duckdb-sources`."""
    candidates = [target / SOURCES]
    if target.is_dir() and not target.is_symlink():
        candidates += sorted(child / SOURCES for child in target.iterdir()
                             if child.is_dir() and not child.is_symlink())
    return [root for root in candidates if root.is_dir() and not root.is_symlink()]


def is_complete(entry: Path) -> bool:
    """A published tree; a staging directory is never reused, even if marked."""
    marker = entry / COMPLETE
    return (not entry.name.startswith(STAGING) and entry.is_dir() and not entry.is_symlink()
            and marker.is_file() and not marker.is_symlink())


def contents(entry: Path) -> str:
    if entry.is_symlink() or not entry.is_dir():
        return "not a directory"
    directories = files = 0
    for _, names, leaves in os.walk(entry):
        directories += len(names)
        files += len(leaves)
    state = "interrupted staging directory" if entry.name.startswith(STAGING) else f"no {COMPLETE} marker"
    return f"{directories} directories, {files} files, {state}"


def repair(target: Path) -> list[str]:
    """Delete incomplete entries below every DuckDB source root of `target`."""
    removed = []
    for root in source_roots(target):
        for entry in sorted(root.iterdir()):
            if is_complete(entry):
                continue
            removed.append(f"{entry} ({contents(entry)})")
            if entry.is_dir() and not entry.is_symlink():
                shutil.rmtree(entry)
            else:
                entry.unlink()
    return removed


def main() -> int:
    removed = repair(TARGET)
    for line in removed:
        print(f"Removed incomplete DuckDB source tree {line}")
    if not removed:
        print(f"No incomplete DuckDB source tree below {TARGET}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
