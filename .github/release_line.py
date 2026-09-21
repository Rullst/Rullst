"""Versioned release-major/branch bindings; historical policies stay immutable."""

from __future__ import annotations

import re


BRANCHES = {12: "v12", 13: "main"}
LEGACY_BRANCHES = {12: "main", 13: "v13"}
EVIDENCE_BRANCHES = frozenset((*BRANCHES.values(), *LEGACY_BRANCHES.values()))
# Reviewed release surfaces. Expanding one line never reinterprets old tags.
FUZZ_TARGET_COUNTS = {12: 40, 13: 42}
VERSION = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?")


def policy_line(policy: dict) -> tuple[int, str]:
    if not isinstance(policy, dict):
        raise ValueError("release policy must be an object")
    schema = policy.get("schema_version")
    branch = policy.get("required_branch")
    if type(schema) is not int:
        raise ValueError("release policy schema must be an integer")
    if schema == 2 and branch == "main":
        # Existing v12 policy remains readable without reinterpreting old tags.
        return 12, "main"
    major = policy.get("required_major")
    branches = LEGACY_BRANCHES if schema == 3 else BRANCHES
    if (schema not in (3, 4) or type(major) is not int or major not in branches
            or branch != branches[major]):
        raise ValueError("release policy must match its versioned major/branch binding")
    return major, branch


def tagged_version(tag: str, policy: dict) -> str:
    major, _ = policy_line(policy)
    match = VERSION.fullmatch(tag.removeprefix("v")) if tag.startswith("v") else None
    if match is None or int(match[1]) != major:
        raise ValueError("release tag must be a canonical SemVer tag for the policy's major")
    if match[4] and any(part.isdigit() and len(part) > 1 and part.startswith("0")
                        for part in match[4].split(".")):
        raise ValueError("numeric prerelease identifiers cannot have leading zeros")
    return tag[1:]
