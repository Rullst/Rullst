#!/usr/bin/env python3
"""Observe Git change impact without executing project code or skipping checks."""

from __future__ import annotations

import argparse
import hashlib
import json
import posixpath
import re
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path


MAX_BYTES = 16 * 1024 * 1024
MAX_FILES = 10000
SHA = re.compile(r"[0-9a-f]{40}")
NAME = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]*")
SITE_FILES = {"docs/home_template.html", "docs/site.css", "docs/site.js"}
CRITICAL_PACKAGES = {
    "rullst", "rullst-core", "rullst-auth", "rullst-security", "rullst-connect",
    "rullst-capital", "rullst-orm", "rullst-macros", "rullst-orm-macros",
}


def safe_path(path: str) -> str:
    if (not path or len(path) > 2048 or path.startswith(("/", "-"))
            or "\\" in path or any(ord(c) < 32 or ord(c) == 127 for c in path)
            or any(part in {"", ".", ".."} for part in path.split("/"))):
        raise ValueError("invalid repository-relative path")
    return path


class Git:
    def __init__(self, root: Path) -> None:
        self.root = root

    def run(self, *args: str) -> bytes:
        # Spool stdout rather than buffering an attacker-sized blob in memory.
        with tempfile.TemporaryFile() as output:
            result = subprocess.run(
                ["git", "--no-replace-objects", "--literal-pathspecs", "-C",
                 str(self.root), *args], stdout=output, stderr=subprocess.DEVNULL,
                timeout=30, check=False,
            )
            if result.returncode or output.tell() > MAX_BYTES:
                raise ValueError("Git input unavailable or exceeds the observation limit")
            output.seek(0)
            return output.read(MAX_BYTES + 1)

    def commit(self, revision: str) -> str:
        if not revision or len(revision) > 256 or revision.startswith("-"):
            raise ValueError("invalid revision")
        value = self.run("rev-parse", "--verify", "--end-of-options",
                         revision + "^{commit}").decode().strip()
        if not SHA.fullmatch(value):
            raise ValueError("expected an available full SHA-1 commit")
        return value

    def tree(self, revision: str) -> dict[str, str]:
        records = self.run("ls-tree", "-r", "-z", "--full-tree", revision).split(b"\0")
        if records[-1] or len(records) > MAX_FILES + 1:
            raise ValueError("invalid or oversized Git tree")
        result = {}
        for record in records[:-1]:
            metadata, raw_path = record.split(b"\t", 1)
            path = safe_path(raw_path.decode("utf-8"))
            mode, _kind, _oid = metadata.decode().split()
            result[path] = mode
        return result

    def manifest(self, revision: str, path: str, tree: dict[str, str]) -> dict:
        if tree.get(path) != "100644":
            raise ValueError("manifest must be a regular non-executable tracked file")
        raw = self.run("show", f"{revision}:{safe_path(path)}")
        if len(raw) > 1024 * 1024:
            raise ValueError("manifest exceeds the observation limit")
        return tomllib.loads(raw.decode("utf-8"))


def parse_changes(raw: bytes) -> list[tuple[str, str]]:
    if not raw:
        return []
    tokens = raw.decode("utf-8").split("\0")
    if tokens.pop() != "":
        raise ValueError("Git diff must be NUL-terminated")
    changes = []
    index = 0
    while index < len(tokens):
        status = tokens[index]
        index += 1
        count = 2 if re.fullmatch(r"[RC][0-9]{1,3}", status) else 1
        if count == 1 and status not in {"A", "M", "D", "T", "U", "X", "B"}:
            raise ValueError("unsupported Git change status")
        if index + count > len(tokens):
            raise ValueError("truncated Git diff")
        for path in tokens[index:index + count]:
            changes.append((status, safe_path(path)))
        index += count
        if len(changes) > MAX_FILES:
            raise ValueError("too many changed paths")
    return changes


def workspace_graph(git: Git, revision: str, tree: dict[str, str]) -> tuple[dict, dict]:
    root = git.manifest(revision, "Cargo.toml", tree)
    workspace = root.get("workspace", {})
    members = workspace.get("members")
    if (not isinstance(members, list) or not members or len(members) > 256
            or len(members) != len(set(members))):
        raise ValueError("expected a bounded explicit workspace member list")
    # Implicit/path/glob members are deliberately not guessed in this phase.
    if "package" in root:
        raise ValueError("root packages require a reviewed inventory extension")
    manifests = {}
    owners = {}
    for member in members:
        if not isinstance(member, str) or any(c in member for c in "*?["):
            raise ValueError("workspace globs require a reviewed inventory extension")
        safe_path(member)
        document = git.manifest(revision, member + "/Cargo.toml", tree)
        name = document.get("package", {}).get("name")
        if not isinstance(name, str) or not NAME.fullmatch(name) or name in manifests:
            raise ValueError("invalid or repeated package name")
        owners[member] = name
        manifests[name] = (member, document)
    reverse = {name: set() for name in manifests}
    shared = workspace.get("dependencies", {})
    for consumer, (member, document) in manifests.items():
        sections = [document, *document.get("target", {}).values()]
        for section in sections:
            for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
                for alias, original in section.get(kind, {}).items():
                    dependency = original
                    base = member
                    if isinstance(dependency, dict) and dependency.get("workspace") is True:
                        if alias not in shared:
                            raise ValueError("unresolved inherited workspace dependency")
                        dependency, base = shared[alias], ""
                    if not isinstance(dependency, (str, dict)):
                        raise ValueError("unsupported dependency declaration")
                    provider = alias if isinstance(dependency, str) else dependency.get("package", alias)
                    if isinstance(dependency, dict) and "path" in dependency:
                        path = posixpath.normpath(posixpath.join(base, dependency["path"]))
                        if path not in owners:
                            raise ValueError("path dependency outside explicit workspace inventory")
                        if provider != owners[path]:
                            raise ValueError("path and package dependency identity disagree")
                    if provider in reverse:
                        # Include optional, target-specific, build and dev edges.
                        reverse[provider].add(consumer)
    return owners, reverse


def reverse_closure(direct: set[str], reverse: dict[str, set[str]]) -> list[str]:
    selected = set(direct)
    remaining = list(direct)
    while remaining:
        for consumer in reverse[remaining.pop()]:
            if consumer not in selected:
                selected.add(consumer)
                remaining.append(consumer)
    return sorted(selected)


def classify(changes: list[tuple[str, str]], owners: dict[str, str],
             tree: dict[str, str]) -> tuple[list[dict], set[str], set[str]]:
    files, direct, reasons = [], set(), set()
    for status, path in changes:
        parts = path.split("/")
        owner = next((owners[root] for root in sorted(owners, key=len, reverse=True)
                      if path.startswith(root + "/")), None)
        category = "unknown"
        if status not in {"A", "M"}:
            reasons.add("deletion-rename-or-type-change")
        elif tree.get(path) not in {"100644", "100755"}:
            reasons.add("non-regular-changed-file")
        if path.startswith((".github/", ".cargo/")) or parts[-1] in {
            "AGENTS.md", "SECURITY.md", "WORKFLOWS.md", "RELEASE_GUIDE.md",
        } or path == "docs/src/spec.md":
            category = "verification-or-security-policy"
            reasons.add("verification-or-security-policy")
        elif parts[-1] in {"Cargo.toml", "Cargo.lock", "build.rs", "rust-toolchain.toml",
                           "rust-toolchain", "deny.toml"}:
            category = "build-or-dependency-input"
            reasons.add("build-or-dependency-input")
        elif path in SITE_FILES:
            category = "site-presentation"
        elif path.endswith(".md"):
            category = "documentation"
            # Markdown can be included by Rust or consumed by packaging tests.
            reasons.add("documentation-may-be-executable")
        elif "fuzz" in parts or "tests" in parts or "fixtures" in parts:
            category = "test-or-fuzz-input"
            reasons.add("shared-test-or-fuzz-input")
        elif owner == "cargo-rullst":
            category = "generator-or-cli"
            reasons.add("generated-application-contracts")
        elif owner and path.endswith(".rs") and "src" in parts:
            category = "rust-source"
            if owner in CRITICAL_PACKAGES:
                reasons.add("critical-or-foundational-package")
        else:
            reasons.add("unknown-path")
        if owner:
            direct.add(owner)
        files.append({"path": path, "status": status, "category": category, "package": owner})
    return files, direct, reasons


def observe(root: Path, base: str, head: str) -> dict:
    report = {
        "schema": "rullst.verification-plan.v1", "phase": "observation-only",
        "candidate_scope": "full", "enforced_scope": "full", "may_skip_checks": False,
        "release_evidence_eligible": False, "base_sha": None, "head_sha": None,
        "policy_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "files": [], "direct_packages": [], "affected_packages": [],
        "fallback_reasons": [],
    }
    try:
        git = Git(root)
        before, after = git.commit(base), git.commit(head)
        report.update(base_sha=before, head_sha=after)
        # A failed ancestry check (including missing history) never narrows scope.
        git.run("merge-base", "--is-ancestor", before, after)
        tree = git.tree(after)
        changes = parse_changes(git.run("diff", "--no-ext-diff", "--no-textconv",
                                        "--find-renames", "--name-status", "-z", before, after, "--"))
        owners, reverse = workspace_graph(git, after, tree)
        files, direct, reasons = classify(changes, owners, tree)
        if not changes:
            reasons.add("empty-change-set")
        report.update(files=files, direct_packages=sorted(direct),
                      affected_packages=reverse_closure(direct, reverse),
                      fallback_reasons=sorted(reasons))
        if not reasons:
            report["candidate_scope"] = "affected-packages" if direct else "site-presentation"
    except (ValueError, TypeError, KeyError, AttributeError, OSError,
            subprocess.TimeoutExpired, RecursionError):
        # Output remains a full recommendation, never an empty success matrix.
        report.update(candidate_scope="full", fallback_reasons=["unavailable-or-unsupported-input"])
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, help="available ancestor commit/ref")
    parser.add_argument("--head", default="HEAD", help="committed tree to inspect (not working files)")
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    args = parser.parse_args()
    print(json.dumps(observe(args.repo, args.base, args.head), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
