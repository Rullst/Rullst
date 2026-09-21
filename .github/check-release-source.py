#!/usr/bin/env python3
"""Reject a release before builds unless tag, packages and protected head agree."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib
import urllib.request

from release_line import policy_line, tagged_version


def validate_source(root: Path, tag: str, sha: str, branch_state: dict) -> None:
    policy = json.loads((root / ".github/release-required-workflows.json").read_text())
    _, branch = policy_line(policy)
    version = tagged_version(tag, policy)
    if re.fullmatch(r"[0-9a-f]{40}", sha) is None:
        raise ValueError("release source must be a complete lowercase commit SHA")
    if (not isinstance(branch_state, dict) or branch_state.get("name") != branch
            or branch_state.get("protected") is not True
            or branch_state.get("commit", {}).get("sha") != sha):
        raise ValueError("release must match the current head of its protected release branch")
    for revision in ("HEAD", f"refs/tags/{tag}^{{commit}}"):
        actual = subprocess.check_output(
            ["git", "rev-parse", "--verify", "--end-of-options", revision],
            cwd=root, stderr=subprocess.PIPE, text=True,
        ).strip()
        if actual != sha:
            raise ValueError("checked-out source and tag must both match the admitted SHA")
    packages = json.loads((root / ".github/release-order.json").read_text())
    if (not isinstance(packages, list) or not packages
            or any(not isinstance(name, str) or re.fullmatch(r"[a-z][a-z0-9-]*", name) is None
                   for name in packages) or len(set(packages)) != len(packages)):
        raise ValueError("invalid release package inventory")
    for name in packages:
        package = tomllib.loads((root / name / "Cargo.toml").read_text())["package"]
        publish = package.get("publish")
        if (package.get("name") != name or package.get("version") != version or publish is False
                or isinstance(publish, list) and "crates-io" not in publish):
            raise ValueError(f"{name} must be publishable at the exact tagged version {version}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--tag", default=os.environ.get("RELEASE_TAG", ""))
    parser.add_argument("--sha", default=os.environ.get("GITHUB_SHA", ""))
    args = parser.parse_args()
    try:
        policy = json.loads((args.root / ".github/release-required-workflows.json").read_text())
        _, branch = policy_line(policy)
        tagged_version(args.tag, policy)
        if os.environ.get("GITHUB_REPOSITORY") != "Rullst/Rullst":
            raise ValueError("release source admission is bound to Rullst/Rullst")
        token = os.environ.get("GITHUB_TOKEN", "")
        if not token:
            raise ValueError("GITHUB_TOKEN is required to verify branch protection")
        request = urllib.request.Request(
            f"https://api.github.com/repos/Rullst/Rullst/branches/{branch}",
            headers={"Accept": "application/vnd.github+json", "Authorization": f"Bearer {token}",
                     "X-GitHub-Api-Version": "2022-11-28"},
        )
        with urllib.request.urlopen(request, timeout=30) as response:
            state = json.load(response)
        validate_source(args.root, args.tag, args.sha, state)
        print(f"Release source admitted: {args.tag}, protected {branch}, {args.sha}")
    except (OSError, ValueError, KeyError, TypeError, AttributeError, subprocess.CalledProcessError) as error:
        print(f"release source admission: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
