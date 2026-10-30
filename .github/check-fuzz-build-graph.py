#!/usr/bin/env python3
"""Cross-check the fuzz evidence input graph with Cargo's own resolution.

For every fuzz package, `cargo metadata --locked` lists the path packages its
build resolves and their target kinds. The evidence policy must reach each of
them; a build script must make the scope fail closed and a proc macro must be
known as one. A scope that already fails closed uses the whole repository.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

from fuzz_evidence_inputs import ROOT, Snapshot, git

FUZZ_BUILD_TARGET = "x86_64-unknown-linux-gnu"


def problems(snapshot: Snapshot, directory: str, metadata: dict, root: Path) -> list[str]:
    scope = snapshot.scopes[directory]
    if scope.reasons:
        return []
    resolved = {node["id"] for node in metadata["resolve"]["nodes"]}
    result = []
    for package in metadata["packages"]:
        if package["source"] is not None or package["id"] not in resolved:
            continue  # registry/git packages are pinned by the fuzz lockfile
        try:
            path = Path(package["manifest_path"]).parent.resolve().relative_to(root.resolve()).as_posix()
        except ValueError:
            result.append(f"{directory}: Cargo builds {package['manifest_path']} outside the repository")
            continue
        path = "" if path == "." else path
        kinds = {kind for target in package["targets"] for kind in target["kind"]}
        if path not in scope.packages:
            result.append(f"{directory}: Cargo builds {path or '.'}, which the scope does not reach")
        elif "custom-build" in kinds:
            result.append(f"{directory}: {path} has a build script but the scope did not fail closed")
        elif "proc-macro" in kinds and not snapshot.graph.package(path)["proc_macro"]:
            result.append(f"{directory}: {path} is a proc macro the scope does not check")
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sha", default=os.environ.get("GITHUB_SHA", ""))
    args = parser.parse_args()
    try:
        sha = args.sha or git("rev-parse", "HEAD").decode().strip()
        if git("rev-parse", "HEAD").decode().strip() != sha:
            raise ValueError("check out the commit being verified; Cargo reads the working tree")
        snapshot = Snapshot(sha)
        found = []
        for directory in sorted(snapshot.directories):
            output = subprocess.run(
                ["cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform",
                 FUZZ_BUILD_TARGET, "--manifest-path", str(ROOT / directory / "Cargo.toml")],
                cwd=ROOT, check=True, capture_output=True).stdout
            found += problems(snapshot, directory, json.loads(output), ROOT)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"fuzz build graph: {error}", file=sys.stderr)
        return 1
    for problem in found:
        print(f"fuzz build graph: {problem}", file=sys.stderr)
    if not found:
        print(f"Fuzz build graph: {len(snapshot.directories)} packages cover Cargo's path packages.")
    return 1 if found else 0


if __name__ == "__main__":
    raise SystemExit(main())
