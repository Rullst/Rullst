#!/usr/bin/env python3
"""Select a measured ancestor without changing an immutable release tag."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path


def resolve_source(mode: str, requested: str, workflow: str, repo: Path) -> str:
    if mode not in {"full", "targeted", "resume", "finalize"}:
        raise ValueError("Unsupported mutation mode.")
    if requested and mode != "full":
        raise ValueError("An explicit source_sha is supported only in full mode.")
    selected = requested or workflow
    for name, sha in (("source", selected), ("workflow", workflow)):
        if not re.fullmatch(r"[0-9a-f]{40}", sha):
            raise ValueError(f"The {name} must be a full lowercase commit SHA.")
        result = subprocess.run(
            ["git", "cat-file", "-t", sha], cwd=repo, text=True,
            capture_output=True, check=False,
        )
        if result.returncode != 0 or result.stdout.strip() != "commit":
            raise ValueError(f"The {name} is not an available commit.")
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", selected, workflow],
        cwd=repo, capture_output=True, check=False,
    )
    if result.returncode != 0:
        raise ValueError("The measured source must be an ancestor of the workflow commit.")
    return selected


def main() -> int:
    if len(sys.argv) != 4:
        print("Usage: resolve-mutation-source.py MODE REQUESTED_SHA WORKFLOW_SHA", file=sys.stderr)
        return 2
    try:
        print(resolve_source(*sys.argv[1:], Path.cwd()))
    except (OSError, ValueError) as error:
        print(error, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
