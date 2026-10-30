#!/usr/bin/env python3
"""Fail when a public dependency leaves the version line allowed for 13.x.

A public dependency is a third-party crate whose types or items the public API
of a release library exposes (see docs/src/compatibility-policy.md, "Public
dependencies"). `.github/public-dependencies.json` maps each one to its caret
line ("0.8" for axum 0.8.x, "1" for tokio 1.x). The check reads the release
packages' manifests (normal and target dependencies, with workspace
inheritance) and Cargo.lock, and fails when:

- a manifest requirement or a locked version lies outside the allowed line;
- a listed crate is no longer a dependency of any release library;
- the policy table in the documentation and the JSON file disagree.

    python3 .github/check-public-dependencies.py
"""

from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
POLICY_FILE = ".github/public-dependencies.json"
# The CLI's library is its implementation, not a framework API.
EXCLUDED = {"cargo-rullst"}
VERSION = re.compile(r"(\d+)(?:\.(\d+))?(?:\.(\d+))?(?:[-+][0-9A-Za-z.+-]*)?")
REQUIREMENT = re.compile(r"\s*([\^~=]?)\s*(" + VERSION.pattern + r")\s*")
TABLE_ROW = re.compile(r"\|\s*`([^`]+)`\s*\|\s*`([^`]+)`\s*\|")


def line(version: str) -> tuple[int, ...]:
    """The caret line of a version: its components up to the first non-zero one."""
    match = VERSION.fullmatch(version.strip())
    if match is None:
        raise ValueError(f"not a version: {version!r}")
    parts = [int(part) for part in match.groups() if part is not None]
    for index, part in enumerate(parts):
        if part != 0:
            return tuple(parts[: index + 1])
    return tuple(parts)


def requirement_line(requirement: str) -> tuple[int, ...] | None:
    """The caret line a single `^`, `~`, `=` or bare requirement stays within, if any."""
    match = REQUIREMENT.fullmatch(requirement)
    return line(match.group(2)) if match else None


def show(value: tuple[int, ...] | None) -> str:
    return ".".join(map(str, value)) if value else "unbounded"


def dependency_tables(manifest: dict) -> list[dict]:
    """Normal dependencies, including target-specific ones; dev and build ones are private."""
    tables = [manifest.get("dependencies", {})]
    tables += [target.get("dependencies", {}) for target in manifest.get("target", {}).values()]
    return tables


def declared(root: Path, package: str, workspace: dict) -> dict[str, list[str]]:
    """Crate name -> the version requirements one release package declares for it."""
    manifest = tomllib.loads((root / package / "Cargo.toml").read_text())
    result: dict[str, list[str]] = {}
    for table in dependency_tables(manifest):
        for key, spec in table.items():
            if isinstance(spec, dict) and spec.get("workspace"):
                spec = {**_as_table(workspace.get(key, {})), **{k: v for k, v in spec.items() if k != "workspace"}}
            spec = _as_table(spec)
            if "path" in spec and "version" not in spec:
                continue
            result.setdefault(spec.get("package", key), []).append(spec.get("version", "*"))
    return result


def _as_table(spec) -> dict:
    return {"version": spec} if isinstance(spec, str) else dict(spec)


def locked(lock: dict) -> tuple[dict[str, list[str]], dict[str, list[tuple[str, str | None]]]]:
    """Versions of every locked crate, and the dependency entries of local packages."""
    versions: dict[str, list[str]] = {}
    local: dict[str, list[tuple[str, str | None]]] = {}
    for package in lock.get("package", []):
        versions.setdefault(package["name"], []).append(package["version"])
        if "source" not in package:
            entries = []
            for entry in package.get("dependencies", []):
                fields = entry.split()
                entries.append((fields[0], fields[1] if len(fields) > 1 else None))
            local[package["name"]] = entries
    return versions, local


def documented(root: Path, policy: dict) -> dict[str, str]:
    """The rows of the policy table: crate -> line."""
    document, _, anchor = policy["policy"].partition("#")
    heading = "## " + anchor.replace("-", " ").capitalize()
    text = (root / document).read_text()
    if heading not in text:
        raise ValueError(f"{document} has no `{heading}` section")
    section = text.split(heading, 1)[1].split("\n## ", 1)[0]
    return {match[1]: match[2] for match in TABLE_ROW.finditer(section)}


def check(root: Path = ROOT) -> list[str]:
    policy = json.loads((root / POLICY_FILE).read_text())
    allowed = {name: line(value) for name, value in policy["dependencies"].items()}
    workspace = tomllib.loads((root / "Cargo.toml").read_text()).get("workspace", {}).get("dependencies", {})
    versions, local = locked(tomllib.loads((root / "Cargo.lock").read_text()))
    order = json.loads((root / ".github/release-order.json").read_text())
    errors: list[str] = []
    used: set[str] = set()
    for package in order:
        if package in EXCLUDED:
            continue
        requirements = declared(root, package, workspace)
        for name, wanted in allowed.items():
            if name not in requirements:
                continue
            used.add(name)
            for requirement in requirements[name]:
                if requirement_line(requirement) != wanted:
                    errors.append(f"{name}: {package} requires `{requirement}` (line "
                                  f"{show(requirement_line(requirement))}), outside the 13.x line {show(wanted)}")
            entries = [version or (versions[dep][0] if len(versions.get(dep, [])) == 1 else None)
                       for dep, version in local.get(package, []) if dep == name]
            in_line = [version for version in entries if version and line(version) == wanted]
            if not in_line:
                found = ", ".join(sorted({version or "?" for version in entries})) or "none"
                errors.append(f"{name}: Cargo.lock resolves {package} to {found}, outside the 13.x line {show(wanted)}")
    for name in sorted(set(allowed) - used):
        errors.append(f"{name}: listed in {POLICY_FILE} but no release library depends on it; "
                      "remove it there and from the policy table")
    table = documented(root, policy)
    if table != policy["dependencies"]:
        errors.append(f"the public dependency table of {policy['policy']} and {POLICY_FILE} differ: "
                      f"only in the table {sorted(set(table.items()) - set(policy['dependencies'].items()))}, "
                      f"only in the file {sorted(set(policy['dependencies'].items()) - set(table.items()))}")
    return errors


def main() -> int:
    errors = check()
    if not errors:
        print(f"Every public dependency stays within its 13.x line ({POLICY_FILE}).")
        return 0
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    print("\nA semver-incompatible update of a public dependency ships only in a Rullst major "
          "release; see docs/src/compatibility-policy.md#public-dependencies. A security exception "
          f"must change {POLICY_FILE} and the policy table explicitly in the same pull request, and "
          "the CHANGELOG must mark it as a security exception with migration notes.", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
