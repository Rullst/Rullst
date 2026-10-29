"""Conservative, Git-object-based input identity for the reviewed fuzz surface.

Each fuzz package's inputs are its Cargo path-dependency graph (see
`fuzz_build_graph.py`) plus a small global set; anything the graph cannot
resolve statically makes the whole repository that package's input.
"""

from __future__ import annotations

import bisect
import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path

from release_line import FUZZ_TARGET_COUNTS, policy_line
from fuzz_build_graph import Graph, under
from fuzz_reviewed_context import REVIEW as CONTEXT_REVIEW, normalized_files

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ".github/workflows/fuzzing.yml"
INVENTORY = ".github/fuzz-targets.json"
DOC_REVIEW = ".github/fuzz-reviewed-publication-docs.json"
SHA = re.compile(r"[0-9a-f]{40}")


class FuzzSurfaceChanged(ValueError):
    """A historical inventory cannot certify the current release surface."""


# Inputs of every fuzz package: workspace manifest (profiles, patches, inherited
# dependencies), toolchain, Cargo configuration, checkout attributes, the
# inventory/release line and the scripts the fuzz workflow runs. `.github/**`
# paths named by the workflow, and sibling modules they import, are added.
GLOBAL_FILES = frozenset({
    "Cargo.toml", "rust-toolchain", "rust-toolchain.toml", ".gitattributes",
    INVENTORY, ".github/release-required-workflows.json", ".github/release_line.py",
    ".github/validate-fuzz-targets.py",
})
GLOBAL_TREES = (".cargo",)

# Reviewed non-inputs, not a glob-based exclusion of all tests or documentation.
# They apply inside reached packages, to the global set and to a scope that
# fails closed; a file a reached package explicitly references stays an input.
# The fuzz crates are separate workspaces with their own retained Cargo.lock.
# Admission/control code is trusted reviewed policy, never execution evidence.
NON_INPUTS = frozenset({
    DOC_REVIEW, CONTEXT_REVIEW,
    ".github/fuzz_reviewed_context.py", ".github/test-fuzz-reviewed-context.py",
    "Cargo.lock", "WORKFLOWS.md", "docs/src/fuzz-evidence.md",
    ".github/mobile-ui-browser-smoke.mjs", ".github/billing-csp-browser-smoke.mjs",
    "rullst-mail/tests/feedback.rs",
    ".github/fuzz_evidence_inputs.py", ".github/fuzz_evidence.py",
    ".github/fuzz_build_graph.py", ".github/fuzz_source_scan.py",
    ".github/test-fuzz-evidence.py", ".github/test-fuzz-evidence-inputs.py",
    ".github/test-fuzz-preflight.py",
    ".github/check-release-admission.py", ".github/test-check-release-admission.py",
    ".github/workflows/release.yml", ".github/workflows/workflow-lint.yml",
    # Archive audit policy inspects packaged files and never builds fuzz code.
    ".github/audit-packages.sh", ".github/test-package-release.py",
    # The packaged-distribution check installs archives and inspects generated
    # projects; it never builds fuzz code either.
    ".github/test-packaged-distribution.sh",
    # Advisory-exception policy is enforced by the audit scanners, which never
    # build fuzz code.
    "deny.toml", "docs/src/security-advisory-exceptions.md",
    ".github/workflows/audit.yml", ".github/workflows/security-audit.yml",
    ".github/workflows/pqc-compliance.yml",
    ".github/workflows/ai-sentinel-pr.yml",
    # `#[cfg(test)]` Core modules: fuzz targets never compile library unit tests.
    "rullst-core/src/server/dev_telemetry/n_plus_one_tests.rs",
    "rullst-core/src/server/stack_tests.rs",
})


def git(*args: str, root: Path = ROOT) -> bytes:
    return subprocess.check_output(["git", *args], cwd=root, stderr=subprocess.PIPE)


def digest(value: object) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def reviewed_document_blobs() -> dict[str, list[str]]:
    """Read trusted policy for one reviewed docs patch, never ignore future edits."""
    review = json.loads((ROOT / DOC_REVIEW).read_text())
    if (set(review) != {"schema_version", "before_commit", "after_commit", "blobs"}
            or review["schema_version"] != 1
            or any(not isinstance(review[key], str) or SHA.fullmatch(review[key]) is None
                   for key in ("before_commit", "after_commit"))
            or not isinstance(review["blobs"], dict)):
        raise ValueError("invalid publication documentation review")
    for path, blobs in review["blobs"].items():
        if (not isinstance(path, str) or not path or path.startswith("/")
                or ".." in path.split("/") or not isinstance(blobs, list) or len(blobs) != 2
                or any(not isinstance(oid, str) or SHA.fullmatch(oid) is None for oid in blobs)
                or blobs[0] == blobs[1]):
            raise ValueError("invalid reviewed document blob identity")
    return review["blobs"]


def execution_contract(workflow: str) -> str:
    """Include the actual build/run steps, pinned tools, flags, caches and runner.

    Only the scheduler's empty-matrix condition is removed. The plan and result
    checker are reviewed policy; actual source jobs must separately prove the
    full 19,800-second step and an eligible release boundary.
    """
    # This deliberately accepts the repository's explicit YAML layout, not
    # arbitrary YAML. Anchors/merges could move execution inputs into a section
    # excluded as scheduling; reject them instead of guessing their expansion.
    if re.search(r"(?m)^[ \t]*(?!#)[^\n]*[\s:\[,][&*][A-Za-z0-9_]", workflow):
        raise ValueError("YAML anchors/aliases require execution policy review")
    for line in workflow.splitlines():
        if line and not line[0].isspace() and not line.startswith("#"):
            if re.match(r"[A-Za-z_][A-Za-z0-9_-]*:", line) is None:
                raise ValueError("unsupported top-level YAML layout")
    settings, jobs = workflow.split("\njobs:\n", 1)
    if any(re.match(r"^  \S", line) and re.fullmatch(r"  [A-Za-z_][A-Za-z0-9_-]*:\s*", line) is None
           for line in jobs.splitlines() if not line.lstrip().startswith("#")):
        raise ValueError("unsupported fuzz workflow job layout")
    # Keep defaults, permissions, environment and unknown root settings. Only
    # workflow identity, dispatch UI and cancellation scheduling are non-inputs.
    settings = re.sub(r"(?ms)^(?:name|on|concurrency):.*?(?=^[A-Za-z_][A-Za-z0-9_-]*:|\Z)", "", settings)
    matches = list(re.finditer(r"(?m)^  ([A-Za-z_][A-Za-z0-9_-]*):\s*$", jobs))
    if not {"targets", "preflight", "fuzz", "evidence-boundary"}.issubset({match[1] for match in matches}):
        raise ValueError("unsupported fuzz workflow job layout")
    execution = []
    for index, match in enumerate(matches):
        if match[1] in {"targets", "evidence-boundary"}:
            continue
        end = matches[index + 1].start() if index + 1 < len(matches) else len(jobs)
        block = jobs[match.start():end].rstrip()
        block = block.replace("    if: needs.targets.outputs.selected_count != '0'\n", "")
        execution.append(block)
    return digest({"settings": settings.strip(), "execution": execution})


class Snapshot:
    def __init__(self, sha: str, root: Path = ROOT):
        if SHA.fullmatch(sha) is None:
            raise ValueError("source must be a full lowercase commit SHA")
        self.sha, self.root = sha, root
        major, self.release_branch = policy_line(json.loads(self.read(".github/release-required-workflows.json")))
        self.files: dict[str, tuple[str, str]] = {}
        for entry in git("ls-tree", "-rz", sha, root=root).split(b"\0"):
            if not entry:
                continue
            meta, path = entry.split(b"\t", 1)
            mode, kind, oid = meta.decode().split()
            if kind != "blob" or mode not in {"100644", "100755"}:
                raise ValueError("submodules/symlinks require a fresh campaign and policy review")
            self.files[path.decode()] = (mode, oid)
        self.inventory = json.loads(self.read(INVENTORY))
        count = FUZZ_TARGET_COUNTS[major]
        if not isinstance(self.inventory, list):
            raise ValueError("fuzz inventory must be an array")
        if len(self.inventory) != count:
            raise FuzzSurfaceChanged(f"v{major} fuzz inventory must contain exactly {count} targets")
        seen = set()
        for item in self.inventory:
            if (not isinstance(item, dict) or set(item) != {"dir", "target"}
                    or not isinstance(item["dir"], str) or not isinstance(item["target"], str)
                    or re.fullmatch(r"[a-z0-9-]+/fuzz", item["dir"]) is None
                    or re.fullmatch(r"[A-Za-z0-9_]+", item["target"]) is None
                    or item["target"] in seen):
                raise ValueError("invalid or duplicate fuzz target")
            seen.add(item["target"])
        self.directories = {item["dir"] for item in self.inventory}
        self.workflow = self.read(WORKFLOW).decode()
        self.contract = execution_contract(self.workflow)
        # Discovery reads the original Git objects; reviewed normalization only
        # changes the identity of documents that remain inputs.
        self.scopes = Graph(self.files, root, self.directories).scopes()
        self.fail_closed = {directory: scope.reasons for directory, scope in sorted(self.scopes.items())
                            if scope.reasons}
        reviewed_docs = reviewed_document_blobs()
        identity_files, self.context_review = normalized_files(
            self.files, self.contract, self.release_branch,
            json.loads((ROOT / CONTEXT_REVIEW).read_text()))

        def entry(path: str) -> list:
            mode, oid = identity_files[path]
            # Only the two explicitly reviewed contents are equivalent. Keep
            # path/mode/deletion in the identity; any third blob is a new input.
            if path in reviewed_docs and mode == "100644" and oid in reviewed_docs[path]:
                oid = reviewed_docs[path][0]
            if path == "rullst-mail/Cargo.toml":
                # A dependency crate's integration-test dependencies are not
                # built by these independent fuzz workspaces. Retain every
                # other manifest field, including normal/build/target deps.
                manifest = tomllib.loads(self.read(path).decode("utf-8-sig"))
                manifest.pop("dev-dependencies", None)
                oid = digest(manifest)
            return [path, mode, oid]

        global_paths = self.global_paths(identity_files)
        self.global_entries = [entry(path) for path in sorted(global_paths)]
        self.packages: dict[str, list] = {}
        self.reached: dict[str, list[str] | None] = {}
        paths = sorted(identity_files)
        for directory, scope in self.scopes.items():
            if scope.reasons:
                # Fail closed: every tracked file except reviewed non-inputs.
                selected, referenced = set(paths), set()
                self.reached[directory] = None
            else:
                selected = {path for package in scope.packages for path in self._below(paths, package)}
                referenced = {path for item in scope.files for path in self._below(paths, item)}
                selected |= referenced
                self.reached[directory] = sorted(scope.packages)
            self.packages[directory] = [
                entry(path) for path in sorted(selected)
                if path not in global_paths
                and (path in referenced or path not in NON_INPUTS and path != WORKFLOW)]
        self.global_hash = digest({"files": self.global_entries, "execution": self.contract})

    @staticmethod
    def _below(paths: list[str], directory: str) -> list[str]:
        start = bisect.bisect_left(paths, directory)
        end = bisect.bisect_left(paths, directory + "0") if directory else len(paths)
        return [path for path in paths[start:end] if under(path, directory)]

    def global_paths(self, identity_files: dict) -> set[str]:
        """Shared inputs, minus reviewed policy and the separately hashed workflow."""
        named = {match.rstrip("./") for match in re.findall(r"\.github/[A-Za-z0-9_./-]+", self.workflow)}
        named.discard(".github")
        pending = [path for path in named if path.endswith(".py") and path not in NON_INPUTS]
        while pending:
            script = pending.pop()
            if script not in self.files:
                continue
            for module in re.findall(r"(?m)^\s*(?:from|import)\s+([A-Za-z_][A-Za-z0-9_]*)",
                                     self.read(script).decode("utf-8", "replace")):
                sibling = f".github/{module}.py"
                if sibling in self.files and sibling not in named and sibling not in NON_INPUTS:
                    named.add(sibling)
                    pending.append(sibling)
        return {path for path in identity_files
                if (path in GLOBAL_FILES or any(under(path, item) for item in (*GLOBAL_TREES, *named)))
                and path not in NON_INPUTS and path != WORKFLOW}

    def read(self, path: str) -> bytes:
        return git("show", f"{self.sha}:{path}", root=self.root)

    def ancestor_of(self, candidate: "Snapshot") -> bool:
        result = subprocess.run(["git", "merge-base", "--is-ancestor", self.sha, candidate.sha],
                                cwd=self.root, capture_output=True, check=False)
        if result.returncode not in {0, 1}:
            raise ValueError("cannot prove source ancestry; fetch complete history")
        return result.returncode == 0

    def fingerprint(self, directory: str) -> str:
        if directory not in self.packages:
            raise ValueError("unknown fuzz package")
        # Hash every reached package (each sibling harness and lock included),
        # referenced outside file and, when closed, the whole repository.
        return digest({"global": self.global_hash, "package": self.packages[directory],
                       "reached": self.reached[directory]})
