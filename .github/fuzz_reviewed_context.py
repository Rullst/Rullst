"""Exact documentation transitions bound to a reviewed complete source context."""

from __future__ import annotations

import hashlib
import json
import re

from release_line import EVIDENCE_BRANCHES

REVIEW = ".github/fuzz-reviewed-context.json"
WORKFLOW = ".github/workflows/fuzzing.yml"
# These are reviewed policy/reporting paths, never campaign execution evidence.
# The fuzz workflow is represented separately by its complete execution contract.
CONTROL_PATHS = frozenset({
    REVIEW, ".github/fuzz_reviewed_context.py", ".github/test-fuzz-reviewed-context.py",
    ".github/fuzz_evidence_inputs.py", ".github/fuzz_evidence.py",
    ".github/workflows/workflow-lint.yml", ".github/workflows/release.yml",
    "docs/src/fuzz-evidence.md",
})
DOCUMENT_PATHS = frozenset({
    ".typos.toml", "CHANGELOG.md", "CONTRIBUTING.md", "GOVERNANCE.md", "README.md",
    "SECURITY.md", "docs/src/SUMMARY.md", "docs/src/crates/connect.md",
    "docs/src/openssf-scorecard.md", "docs/src/spec.md", "docs/src/v12-1-2-review.md",
    "docs/src/v12.md",
})
DEVELOPMENT_DOCUMENT_PATHS = DOCUMENT_PATHS - {
    "docs/src/crates/connect.md", "docs/src/v12-1-2-review.md", "docs/src/v12.md",
}


def document_paths(branch: str) -> frozenset:
    return DOCUMENT_PATHS if branch == "v12" else DEVELOPMENT_DOCUMENT_PATHS


SHA = re.compile(r"[0-9a-f]{40}")
SHA256 = re.compile(r"[0-9a-f]{64}")


def context_digest(files: dict, contract: str, branch: str) -> str:
    """Pin every other tracked path, mode and blob, including all source consumers."""
    payload = {"files": [[path, *entry] for path, entry in sorted(files.items())
                         if path not in document_paths(branch) | CONTROL_PATHS | {WORKFLOW}],
               "execution": contract, "release_branch": branch}
    return hashlib.sha256(json.dumps(payload, sort_keys=True,
                                    separators=(",", ":")).encode()).hexdigest()


def validate(review: dict) -> None:
    if (not isinstance(review, dict)
            or set(review) != {"schema_version", "release_branch", "before_commit",
                               "after_commit", "context_sha256", "documents"}
            or type(review["schema_version"]) is not int or review["schema_version"] != 1
            or not isinstance(review["release_branch"], str)
            or review["release_branch"] not in EVIDENCE_BRANCHES
            or any(not isinstance(review[key], str) or SHA.fullmatch(review[key]) is None
                   for key in ("before_commit", "after_commit"))
            or review["before_commit"] == review["after_commit"]
            or not isinstance(review["context_sha256"], str)
            or SHA256.fullmatch(review["context_sha256"]) is None
            or not isinstance(review["documents"], dict)
            or set(review["documents"]) != document_paths(review["release_branch"])):
        raise ValueError("invalid complete-context documentation review")
    for states in review["documents"].values():
        if not isinstance(states, dict) or set(states) != {"before", "after"}:
            raise ValueError("invalid reviewed document states")
        for name, entry in states.items():
            # Only the two reviewed additions may have an absent old state.
            if entry is None and name == "before":
                continue
            if (not isinstance(entry, list) or len(entry) != 2 or entry[0] != "100644"
                    or not isinstance(entry[1], str) or SHA.fullmatch(entry[1]) is None):
                raise ValueError("invalid reviewed document mode/blob")
        if states["before"] == states["after"]:
            raise ValueError("document review must describe a change")
    additions = {path for path, states in review["documents"].items()
                 if states["before"] is None}
    if additions != {"GOVERNANCE.md", "docs/src/openssf-scorecard.md"}:
        raise ValueError("unsupported reviewed document additions")


def normalized_files(files: dict, contract: str, branch: str, review: dict) -> tuple[dict, str | None]:
    """Normalize only one entire reviewed state with an identical source context.

    The caller retains its original Git mapping for source/dependency discovery.
    Unknown content, partial states and changed consumers receive no exception,
    even if those new consumers already have their own successful campaign.
    """
    validate(review)
    if (branch != review["release_branch"]
            or context_digest(files, contract, branch) != review["context_sha256"]):
        return files, None
    documents = review["documents"]
    if not any(all((list(files[path]) if path in files else None) == states[state]
                   for path, states in documents.items()) for state in ("before", "after")):
        return files, None
    result = dict(files)
    for path, states in documents.items():
        if states["before"] is None:
            result.pop(path, None)
        else:
            result[path] = tuple(states["before"])
    return result, review["context_sha256"]
