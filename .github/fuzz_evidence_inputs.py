"""Conservative, Git-object-based input identity for the reviewed fuzz surface."""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path

from release_line import policy_line

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ".github/workflows/fuzzing.yml"
INVENTORY = ".github/fuzz-targets.json"
DOC_REVIEW = ".github/fuzz-reviewed-publication-docs.json"
SHA = re.compile(r"[0-9a-f]{40}")

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
        self.release_branch = policy_line(json.loads(self.read(".github/release-required-workflows.json")))[1]
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
        if not isinstance(self.inventory, list) or len(self.inventory) != 40:
            raise ValueError("fuzz inventory must contain exactly 40 targets")
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
        for path, (mode, oid) in sorted(self.files.items()):
            if path in NON_INPUTS or path == WORKFLOW:
                continue
            # Only the two explicitly reviewed contents are equivalent. Keep
            # path/mode/deletion in the identity; any third blob is a new input.
            if path in reviewed_docs and mode == "100644" and oid in reviewed_docs[path]:
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
            (self.packages[package] if package else self.global_entries).append(entry)
            if package and not isolated:
                self.global_entries.append(entry)
        self.global_hash = digest({"files": self.global_entries, "execution": self.contract})

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
        # Hash the entire package, including every sibling harness and lock.
        return digest({"global": self.global_hash, "package": self.packages[directory]})
