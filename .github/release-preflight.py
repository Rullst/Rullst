#!/usr/bin/env python3
"""Refuse a release candidate before its admission campaign, not hours into it.

Run on the release branch head before dispatching the manual admission
workflows:

    python3 .github/release-preflight.py v13.0.0-alpha.2

It only reads Git, the source tree and public crates.io endpoints. Each check
mirrors a refusal that release.yml would otherwise reach after the full
admission and two hours of packaging:
- every release package is publishable at the tagged version;
- CHANGELOG.md renders release notes for that version, and changelog.d/ has
  no fragment left to merge;
- the tag does not exist yet and HEAD is the release branch head;
- every package name is registered and owned by the reviewed owner
  (release.yml refuses first publications);
- no package already has that version on crates.io.

When the tag already exists, it checks crates.io for that version. If no
package has it, the release published nothing, and the refusal prints the
commands that delete and recreate the tag, which MAINTAINING.md section 7.5
allows only with the maintainer's approval. It never deletes a tag itself.
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
TAG_RULE = "MAINTAINING.md §7.5"

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
    return failures


def remote_tag_commit(root: Path, tag: str) -> str | None:
    """The commit `tag` names on origin (peeled when annotated), or None."""
    listing = git(root, "ls-remote", "--tags", "origin", f"refs/tags/{tag}", f"refs/tags/{tag}^{{}}")
    refs = {}
    for line in listing.splitlines():
        sha, _, name = line.partition("\t")
        refs[name] = sha
    return refs.get(f"refs/tags/{tag}^{{}}") or refs.get(f"refs/tags/{tag}")


def publication_status(
    packages: list[str], version: str, fetch: Fetch, pause: float = 0.0
) -> tuple[list[str], list[str]]:
    """The packages that have `version` on crates.io, and the lookups that failed."""
    published: list[str] = []
    unknown: list[str] = []
    for name in packages:
        status, _ = fetch(f"https://crates.io/api/v1/crates/{name}/{version}")
        if status == 200:
            published.append(name)
        elif status != 404:
            unknown.append(f"{name} (HTTP {status})")
        if pause:
            time.sleep(pause)
    return published, unknown


def existing_tag_failures(
    root: Path, tag: str, branch: str, version: str, packages: list[str], fetch: Fetch,
    pause: float = 0.0,
) -> list[str]:
    """Refuse an existing tag and say what MAINTAINING.md §7.5 allows for it.

    Only reads: deleting a tag needs the maintainer's approval every time.
    """
    commit = remote_tag_commit(root, tag)
    if commit is None:
        return []
    published, unknown = publication_status(packages, version, fetch, pause)
    if published:
        return [
            f"tag {tag} already exists on origin and {len(published)} of {len(packages)} packages "
            f"have {version} on crates.io ({', '.join(published)}). A published version is never "
            f"reused ({TAG_RULE}): release the next version, or resume an interrupted publication "
            "from the same tag (docs/src/release-recovery.md)."
        ]
    if unknown:
        return [
            f"tag {tag} already exists on origin, and crates.io could not confirm that {version} is "
            f"unpublished: {', '.join(unknown)}. Rerun the preflight before deciding anything about the tag."
        ]
    return ["\n".join((
        f"tag {tag} already exists on origin (commit {commit[:12]}), and no release package has "
        f"{version} on crates.io: that release published nothing.",
        f"  {TAG_RULE}: such a tag may be deleted and recreated only with the maintainer's "
        "explicit approval. This preflight never deletes it.",
        f"  First check that no release.yml run for {tag} is still running or waiting for the "
        "crates-io approval:",
        f"    gh run list --workflow release.yml --branch {tag}",
        "  With the maintainer's approval, delete the tag on origin and any local copy:",
        f"    git push origin --delete refs/tags/{tag}",
        f"    git tag --delete {tag}",
        f"  Then rerun this preflight on the corrected head of origin/{branch}, admit that commit",
        "  and tag it again (RELEASE_GUIDE.md, Phase 3):",
        f"    git switch {branch}",
        f"    git pull --ff-only origin {branch}",
        f"    git tag {tag}",
        f"    git push origin {tag}",
    ))]


def notes_failures(root: Path, version: str) -> list[str]:
    try:
        changelog = (root / "CHANGELOG.md").read_text(encoding="utf-8")
        load("render-release-notes").render(changelog, version, "Rullst/Rullst")
    except (OSError, SystemExit) as error:
        return [str(error)]
    return []


def fragment_failures(root: Path) -> list[str]:
    try:
        load("check-release-source").require_merged_changelog(root)
    except (OSError, ValueError) as error:
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

    failures = notes_failures(root, version) + fragment_failures(root)
    try:
        failures += git_failures(root, args.tag, branch)
        failures += existing_tag_failures(root, args.tag, branch, version, packages, fetch_json,
                                          pause=0.2)
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
