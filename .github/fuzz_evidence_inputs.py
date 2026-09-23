"""Conservative, Git-object-based input identity for the reviewed fuzz surface."""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
import tomllib
from functools import lru_cache
from pathlib import Path

from release_line import FUZZ_TARGET_COUNTS, policy_line
from fuzz_dependency_inputs import DependencyScope, SCOPE_REVIEW, UnprovenScope

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ".github/workflows/fuzzing.yml"
INVENTORY = ".github/fuzz-targets.json"
DOC_REVIEW = ".github/fuzz-reviewed-publication-docs.json"
MAINTENANCE_DOC_REVIEW = ".github/fuzz-reviewed-maintenance-docs.json"
REGISTRY_DOC_REVIEW = ".github/fuzz-reviewed-registry-docs.json"
SHA = re.compile(r"[0-9a-f]{40}")


class FuzzSurfaceChanged(ValueError):
    """A historical inventory cannot certify the current release surface."""

# Reviewed non-inputs, not a glob-based exclusion of all tests or documentation.
# The fuzz crates are separate workspaces with their own retained Cargo.lock.
# Admission/control code is trusted reviewed policy, never execution evidence.
NON_INPUTS = frozenset({
    DOC_REVIEW,
    "Cargo.lock", "WORKFLOWS.md", "docs/src/fuzz-evidence.md",
    ".github/mobile-ui-browser-smoke.mjs", ".github/billing-csp-browser-smoke.mjs",
    "rullst-mail/tests/feedback.rs",
    ".github/fuzz_evidence_inputs.py", ".github/fuzz_evidence.py",
    ".github/test-fuzz-evidence.py", ".github/test-fuzz-evidence-inputs.py",
    ".github/test-fuzz-preflight.py",
    ".github/check-release-admission.py", ".github/test-check-release-admission.py",
    ".github/workflows/release.yml", ".github/workflows/workflow-lint.yml",
    ".github/fuzz_dependency_inputs.py", ".github/test-fuzz-dependency-inputs.py",
    SCOPE_REVIEW, MAINTENANCE_DOC_REVIEW, REGISTRY_DOC_REVIEW,
    ".github/workflows/coverage.yml",
    ".github/test-fuzz-target-quality.py", "rullst/tests/fuzz_harness_contracts.rs",
})


def git(*args: str, root: Path = ROOT) -> bytes:
    return subprocess.check_output(["git", *args], cwd=root, stderr=subprocess.PIPE)


def digest(value: object) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


@lru_cache(maxsize=4096)
def read_blob(oid: str, root: Path) -> bytes:
    return git("cat-file", "blob", oid, root=root)


def dependency_scope_review() -> dict:
    return json.loads((ROOT / SCOPE_REVIEW).read_text())


def reviewed_document_blobs() -> dict[str, list[str]]:
    """Combine explicitly reviewed document contents; future edits remain inputs."""
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
    maintenance = json.loads((ROOT / MAINTENANCE_DOC_REVIEW).read_text())
    if (set(maintenance) != {"schema_version", "baseline_commit", "blobs"}
            or maintenance["schema_version"] != 1
            or not isinstance(maintenance["baseline_commit"], str)
            or SHA.fullmatch(maintenance["baseline_commit"]) is None
            or not isinstance(maintenance["blobs"], dict)):
        raise ValueError("invalid maintenance documentation review")
    for path, blobs in maintenance["blobs"].items():
        if (path not in {"CHANGELOG.md", "docs/src/spec.md", "docs/src/v12-1-1-review.md"}
                or not isinstance(blobs, list) or len(blobs) != 2
                or any(not isinstance(oid, str) or SHA.fullmatch(oid) is None for oid in blobs)
                or blobs[0] == blobs[1]):
            raise ValueError("invalid reviewed maintenance documentation identity")
        # A maintenance review must not silently supersede another exception.
        if path in review["blobs"]:
            raise ValueError("overlapping documentation reviews require explicit policy migration")
    documents = review["blobs"] | maintenance["blobs"]
    registry = json.loads((ROOT / REGISTRY_DOC_REVIEW).read_text())
    # This is an explicit migration for the 12.1.1 registry README patch, not
    # permission to ignore Markdown files or arbitrary future document edits.
    allowed = {
        "CHANGELOG.md", "README.md", "docs/src/3-rullst-studio.md",
        "docs/src/4-rullst-nexus.md", "docs/src/spec.md", "docs/src/v12-1-1-review.md",
        "rullst-ai/README.md", "rullst-capital/README.md", "rullst-connect/README.md",
        "rullst-core/README.md", "rullst-iot/README.md", "rullst-mail/README.md",
        "rullst-messaging/README.md", "rullst-nexus/README.md",
        "rullst-orm-macros/README.md", "rullst-orm/README.md", "rullst-studio/README.md",
    }
    if (set(registry) != {"schema_version", "before_commit", "after_commit", "blobs"}
            or registry["schema_version"] != 1
            or any(not isinstance(registry[key], str) or SHA.fullmatch(registry[key]) is None
                   for key in ("before_commit", "after_commit"))
            or not isinstance(registry["blobs"], dict) or set(registry["blobs"]) != allowed):
        raise ValueError("invalid registry documentation review")
    for path, blobs in registry["blobs"].items():
        if (not isinstance(blobs, list) or len(blobs) != 2
                or any(not isinstance(oid, str) or SHA.fullmatch(oid) is None for oid in blobs)
                or blobs[0] == blobs[1]):
            raise ValueError("invalid reviewed registry documentation identity")
        # Preserve the prior canonical blob and prior reviewed states so both
        # original maintenance campaigns and newer Auth campaigns can qualify.
        documents[path] = list(dict.fromkeys(documents.get(path, []) + blobs))
    return documents


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
        self.global_entries = []
        self.packages: dict[str, list] = {directory: [] for directory in self.directories}
        # Includes, process/file access or cross-package path dependencies make
        # package isolation uncertain. In that case include all fuzz inputs in
        # the shared identity as well, so any fuzz change reruns every target.
        isolated = True
        for directory in self.directories:
            manifest = tomllib.loads(self.read(f"{directory}/Cargo.toml").decode("utf-8-sig"))
            if (manifest.get("build") or manifest.get("package", {}).get("build")
                    or f"{directory}/build.rs" in self.files):
                isolated = False
            for section in ("dependencies", "build-dependencies", "dev-dependencies"):
                for dependency in manifest.get(section, {}).values():
                    if isinstance(dependency, dict) and "path" in dependency:
                        # The reviewed shape is a fuzz crate depending on its
                        # parent runtime crate; unusual paths become global.
                        if dependency["path"] != "..":
                            isolated = False
            if manifest.get("target") or manifest.get("patch"):
                isolated = False
            for path in self.files:
                if path.startswith(directory + "/") and path.endswith(".rs"):
                    source = self.read(path)
                    if path == "rullst-orm/fuzz/fuzz_targets/fuzz_parser.rs":
                        # This reviewed parser module is already a global input.
                        source = source.replace(b'#[path = "../../../rullst-orm-macros/src/parser.rs"]', b"")
                    if re.search(rb"include|\bpath\s*=|\b(?:fs|env|process)\s*::|\b(?:File|Command)\s*::", source):
                        isolated = False
        reviewed_docs = reviewed_document_blobs()
        self.scope_reason = "dependency closure"
        try:
            scope = DependencyScope(self, dependency_scope_review(), reviewed_docs)
        except UnprovenScope as error:
            scope = None
            self.scope_reason = str(error)
        self.dependency_scope = scope
        for path, (mode, oid) in sorted(self.files.items()):
            if path in NON_INPUTS or path == WORKFLOW:
                continue
            # Only explicitly reviewed contents are equivalent. Keep path/mode/
            # deletion in the identity; an unreviewed blob is a new input.
            if (path in reviewed_docs and mode == "100644" and oid in reviewed_docs[path]
                    and (scope is None or path not in scope.included_files)):
                # A frozen document review is not a blanket promise about future
                # consumers. Unreviewed contexts also retain its actual bytes,
                # including after their own new source campaign has succeeded.
                unproven = scope.unproven_consumers if scope is not None else self.directories
                for directory in sorted(unproven):
                    self.packages[directory].append([path, mode, oid])
                oid = reviewed_docs[path][0]
            package = next((directory for directory in self.directories
                            if path.startswith(directory + "/")), None)
            if path == "rullst-mail/Cargo.toml":
                # A dependency crate's integration-test dependencies are not
                # built by these independent fuzz workspaces. Retain every
                # other manifest field, including normal/build/target deps.
                manifest = tomllib.loads(self.read(path).decode("utf-8-sig"))
                manifest.pop("dev-dependencies", None)
                oid = digest(manifest)
            entry = [path, mode, oid]
            recipients = (scope.recipients(path)
                          if scope is not None and isolated and package is None
                          and path not in reviewed_docs else None)
            if package:
                self.packages[package].append(entry)
            elif recipients is not None:
                for directory in sorted(recipients):
                    self.packages[directory].append(entry)
            else:
                self.global_entries.append(entry)
            if package and not isolated:
                self.global_entries.append(entry)
        self.global_hash = digest({"files": self.global_entries, "execution": self.contract})

    def read(self, path: str) -> bytes:
        if hasattr(self, "files") and path in self.files:
            return read_blob(self.files[path][1], self.root)
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
        # Hash the entire package, including every sibling harness and lock.
        return digest({"global": self.global_hash, "package": self.packages[directory]})
