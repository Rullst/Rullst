"""Explicit release-major/branch binding, including the immutable v12 policy."""

from __future__ import annotations

import re


BRANCHES = {12: "main", 13: "v13"}
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
    if (schema != 3 or type(major) is not int or major not in BRANCHES
            or branch != BRANCHES[major]):
        raise ValueError("release policy must bind major 12 to main or major 13 to v13")
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
