#!/usr/bin/env python3
"""Refuse a release candidate before its admission campaign, not hours into it.

Run on the release branch head before dispatching the manual admission
workflows:

    python3 .github/release-preflight.py v13.0.0-alpha.2

It only reads Git, the source tree and public crates.io endpoints. Each check
mirrors a refusal that release.yml would otherwise reach after the full
admission and two hours of packaging:
- every release package is publishable at the tagged version;
- CHANGELOG.md renders release notes for that version;
- the tag does not exist yet and HEAD is the release branch head;
- every package name is registered and owned by the reviewed owner
  (release.yml refuses first publications);
- no package already has that version on crates.io.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import time
from typing import Any, Callable
import urllib.error
import urllib.request

from release_line import policy_line, tagged_version

HERE = Path(__file__).resolve().parent
RECOVERY = "docs/src/release-recovery.md#v13-package-registration"

Fetch = Callable[[str], tuple[int, Any]]


def load(script: str):
    spec = importlib.util.spec_from_file_location(script.replace("-", "_"), HERE / f"{script}.py")
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot import {script}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def fetch_json(url: str) -> tuple[int, Any]:
    request = urllib.request.Request(
        url, headers={"User-Agent": "rullst-release-preflight/1"}
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return response.status, json.load(response)
    except urllib.error.HTTPError as error:
        return error.code, None


def registry_failures(
    packages: list[str], version: str, owner: str, fetch: Fetch, pause: float = 0.0
) -> list[str]:
    failures: list[str] = []
    for name in packages:
        status, owners = fetch(f"https://crates.io/api/v1/crates/{name}/owners")
        if status == 404:
            failures.append(
                f"{name} is not registered on crates.io; release.yml refuses first "
                f"publications. Merge a reviewed bootstrap change before the admission ({RECOVERY})."
            )
            continue
        if status != 200 or not isinstance(owners, dict):
            failures.append(f"crates.io owner lookup for {name} returned HTTP {status}")
            continue
        logins = [
            entry.get("login")
            for key in ("users", "teams")
            for entry in owners.get(key) or []
            if isinstance(entry, dict)
        ]
        if owner not in logins:
            failures.append(f"{name} is not owned by the reviewed owner {owner}")
        status, _ = fetch(f"https://crates.io/api/v1/crates/{name}/{version}")
        if status == 200:
            failures.append(f"{name} {version} is already published; choose a new version")
        elif status != 404:
            failures.append(f"crates.io version lookup for {name} {version} returned HTTP {status}")
        if pause:
            time.sleep(pause)
    return failures


def git(root: Path, *args: str) -> str:
    return subprocess.check_output(
        ["git", *args], cwd=root, stderr=subprocess.PIPE, text=True
    ).strip()


def git_failures(root: Path, tag: str, branch: str) -> list[str]:
    failures: list[str] = []
    head = git(root, "rev-parse", "HEAD")
    remote = git(root, "ls-remote", "origin", f"refs/heads/{branch}").split()
    if not remote or remote[0] != head:
        failures.append(
            f"HEAD {head[:12]} is not the head of origin/{branch}; the admission and tag must use it"
        )
    if git(root, "ls-remote", "--tags", "origin", f"refs/tags/{tag}"):
        failures.append(f"tag {tag} already exists on origin")
    return failures


def notes_failures(root: Path, version: str) -> list[str]:
    try:
        changelog = (root / "CHANGELOG.md").read_text(encoding="utf-8")
        load("render-release-notes").render(changelog, version, "Rullst/Rullst")
    except (OSError, SystemExit) as error:
        return [str(error)]
    return []


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("tag", help="the tag to create, for example v13.0.0-alpha.2")
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args()
    root = args.root
    try:
        policy = json.loads((root / ".github/release-required-workflows.json").read_text())
        _, branch = policy_line(policy)
        version = tagged_version(args.tag, policy)
        packages = load("check-release-source").validate_packages(root, version)
        owner = json.loads((root / ".github/crates-ownership-policy.json").read_text())[
            "expected_owner"
        ]
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"release preflight: {error}", file=sys.stderr)
        return 1

    failures = notes_failures(root, version)
    try:
        failures += git_failures(root, args.tag, branch)
    except subprocess.CalledProcessError as error:
        failures.append(f"git failed: {(error.stderr or '').strip()}")
    failures += registry_failures(packages, version, owner, fetch_json, pause=0.2)

    if failures:
        print(f"release preflight refused {args.tag}:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print(f"release preflight passed: {args.tag}, {len(packages)} packages, origin/{branch}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
