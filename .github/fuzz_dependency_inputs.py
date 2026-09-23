"""Conservative local dependency closure for fuzz input attribution.

No Cargo execution or network resolution is needed. All dependency kinds,
features and target tables are included; manifests remain global inputs.
Unsupported compile-time consumers fall back to the original global policy.
"""

from __future__ import annotations

import hashlib
import json
import posixpath
import re
import tomllib

SCOPE_REVIEW = ".github/fuzz-source-scope.json"
PACKAGE = re.compile(r"[A-Za-z0-9_-]+(?:/[A-Za-z0-9_-]+)*")
INCLUDE = re.compile(r"\binclude(?:_str|_bytes)?\s*!\s*\(\s*|#\s*\[\s*path\s*=\s*")
LITERAL = re.compile(r'"([^"\\\n]*)"')


class UnprovenScope(ValueError):
    """Use global attribution until the new layout has been reviewed."""


def tree_digest(files: dict, prefix: str) -> str:
    entries = [[path, *mode_oid] for path, mode_oid in sorted(files.items())
               if path.startswith(prefix + "/")]
    return hashlib.sha256(json.dumps(entries, separators=(",", ":")).encode()).hexdigest()


def source_context_digest(files: dict, package_directories, closure, reviewed_documents=()) -> str:
    """Pin the reviewed potential consumers of a selectively attributed crate.

    Cargo edges alone cannot prove absence of arbitrary runtime file reads.
    Unchanged reviewed consumers are required as well. Unknown future consumer
    code restores global attribution even after its own fresh campaign passed.
    """
    entries = []
    for path, mode_oid in sorted(files.items()):
        if path in reviewed_documents:
            continue
        owner = next((p for p in sorted(package_directories, key=len, reverse=True)
                      if path.startswith(p + "/")), None)
        if path == "Cargo.toml" or owner in closure:
            entries.append([path, *mode_oid])
    return hashlib.sha256(json.dumps(entries, separators=(",", ":")).encode()).hexdigest()


class DependencyScope:
    def __init__(self, snapshot, review: dict, reviewed_documents=()):
        self.snapshot = snapshot
        root = self.manifest("Cargo.toml")
        workspace = root.get("workspace", {})
        members = workspace.get("members")
        if (not isinstance(members, list) or not members
                or any(not isinstance(p, str) or PACKAGE.fullmatch(p) is None for p in members)
                or len(members) != len(set(members))):
            raise UnprovenScope("workspace members require explicit reviewed paths")
        self.workspace_dependencies = workspace.get("dependencies", {})
        self.directories = set(members) | snapshot.directories
        self.edges = {directory: set() for directory in self.directories}
        self.manifests = {p: self.manifest(p + "/Cargo.toml") for p in self.directories}
        if (set(review) != {"schema_version", "procedural_macros", "doctest_modules", "cargo_configs",
                           "scoped_packages", "source_contexts"}
                or review["schema_version"] != 1):
            raise ValueError("invalid dependency-scope review")
        configs = {p: list(value) for p, value in snapshot.files.items()
                   if re.search(r"(?:^|/)\.cargo/config(?:\.toml)?$", p)}
        if configs != review["cargo_configs"]:
            raise UnprovenScope("Cargo configuration requires scope review")
        macros = {p for p, m in self.manifests.items() if m.get("lib", {}).get("proc-macro")}
        if macros != set(review["procedural_macros"]):
            raise UnprovenScope("procedural macro inventory changed")
        for directory in macros:
            reviewed = review["procedural_macros"][directory]
            # Legacy profiles pin one tree. A reviewed metadata-only migration
            # may retain two exact trees; no unreviewed macro tree is accepted.
            trees = [reviewed] if isinstance(reviewed, str) else reviewed
            if (not isinstance(trees, list) or not 1 <= len(trees) <= 2
                    or any(not isinstance(t, str) or re.fullmatch(r"[0-9a-f]{64}", t) is None
                           for t in trees) or len(set(trees)) != len(trees)):
                raise ValueError("invalid reviewed procedural macro tree identity")
            if tree_digest(snapshot.files, directory) not in trees:
                raise UnprovenScope("procedural macro implementation requires review")
        self.macros = macros
        self.included_files = set()
        for directory, manifest in self.manifests.items():
            self.add_dependencies(directory, manifest)
        self.add_source_inclusions(review["doctest_modules"])
        self.consumers = {directory: set() for directory in self.directories}
        self.closures = {}
        for fuzz_directory in snapshot.directories:
            pending, visited = [fuzz_directory], set()
            while pending:
                directory = pending.pop()
                if directory in visited:
                    continue
                visited.add(directory)
                self.consumers[directory].add(fuzz_directory)
                pending.extend(self.edges[directory] - visited)
            self.closures[fuzz_directory] = visited
        if (not isinstance(review["scoped_packages"], list)
                or any(not isinstance(p, str) for p in review["scoped_packages"])
                or not isinstance(review["source_contexts"], dict)):
            raise ValueError("invalid reviewed dependency profile")
        for directory, contexts in review["source_contexts"].items():
            if (directory not in snapshot.directories or not isinstance(contexts, list)
                    or any(not isinstance(c, str) or re.fullmatch(r"[0-9a-f]{64}", c) is None for c in contexts)):
                raise ValueError("invalid reviewed source context identity")
        self.scoped_packages = set(review["scoped_packages"])
        if not self.scoped_packages or not self.scoped_packages <= set(members):
            raise ValueError("invalid reviewed package scope")
        self.contexts = {directory: source_context_digest(snapshot.files, self.directories, closure,
                                                         reviewed_documents)
                         for directory, closure in self.closures.items()}
        self.unproven_consumers = {d for d, context in self.contexts.items()
                                   if context not in review["source_contexts"].get(d, [])}

    def manifest(self, path: str) -> dict:
        if path not in self.snapshot.files:
            raise UnprovenScope("local dependency manifest is unavailable")
        try:
            value = tomllib.loads(self.snapshot.read(path).decode("utf-8-sig"))
        except (tomllib.TOMLDecodeError, UnicodeDecodeError) as error:
            raise UnprovenScope("unreadable Cargo manifest requires full preflight") from error
        if value.get("patch") or value.get("replace"):
            raise UnprovenScope("dependency replacement requires scope review")
        return value

    def owner(self, path: str) -> str | None:
        return next((directory for directory in sorted(self.directories, key=len, reverse=True)
                     if path.startswith(directory + "/")), None)

    def add_dependencies(self, directory: str, manifest: dict) -> None:
        workspace_path = manifest.get("package", {}).get("workspace")
        if workspace_path is not None and posixpath.normpath(directory + "/" + workspace_path) != ".":
            raise UnprovenScope("package belongs to an unreviewed workspace")
        build = manifest.get("package", {}).get("build")
        if (build is not None and build is not False) or directory + "/build.rs" in self.snapshot.files:
            raise UnprovenScope("local build scripts require global attribution")
        for target in (manifest.get("lib", {}), *manifest.get("bin", [])):
            if "path" in target:
                path = posixpath.normpath(directory + "/" + target["path"])
                if (self.owner(path) != directory
                        or not any(path.startswith(directory + "/" + prefix)
                                   for prefix in ("src/", "fuzz_targets/"))):
                    raise UnprovenScope("external Cargo target source requires review")
        sections = [manifest, *manifest.get("target", {}).values()]
        for section in sections:
            for kind in ("dependencies", "build-dependencies", "dev-dependencies"):
                for name, dependency in section.get(kind, {}).items():
                    if not isinstance(dependency, dict):
                        continue
                    origin = directory
                    if dependency.get("workspace"):
                        if directory in self.snapshot.directories:
                            raise UnprovenScope("nested fuzz workspace inheritance requires review")
                        if name not in self.workspace_dependencies:
                            raise UnprovenScope("unresolved workspace dependency")
                        if any(k in dependency for k in ("path", "git", "version", "package")):
                            raise UnprovenScope("ambiguous inherited dependency")
                        dependency = self.workspace_dependencies[name]
                        origin = "."
                    if not isinstance(dependency, dict) or "path" not in dependency:
                        continue
                    if "git" in dependency:
                        raise UnprovenScope("mixed Git/path dependency requires review")
                    child = posixpath.normpath(origin + "/" + dependency["path"])
                    if child not in self.directories:
                        raise UnprovenScope("dependency outside the declared workspace")
                    self.edges[directory].add(child)

    def add_source_inclusions(self, doctests: dict) -> None:
        for path, (_, oid) in self.snapshot.files.items():
            owner = self.owner(path)
            if (owner is None or not path.endswith(".rs")
                    or not any(path.startswith(owner + "/" + prefix)
                               for prefix in ("src/", "fuzz_targets/"))):
                continue
            # Only an exact, reviewed cfg(doctest) module/parent pair is omitted.
            ignored = doctests.get(path)
            if (ignored and oid == ignored["blob"]
                    and self.snapshot.files.get(ignored["parent"], (None, None))[1] == ignored["parent_blob"]):
                continue
            source = self.snapshot.read(path).decode("utf-8-sig")
            if re.search(r"\bcfg_attr\s*\([^)]*\bpath\s*=", source, re.DOTALL):
                raise UnprovenScope("conditional module path requires scope review")
            for match in INCLUDE.finditer(source):
                literal = LITERAL.match(source, match.end())
                if literal is None:
                    raise UnprovenScope("dynamic source inclusion requires global attribution")
                relative = literal[1]
                if relative.startswith("/") or not relative:
                    raise UnprovenScope("external source inclusion requires review")
                origins = [posixpath.dirname(path)]
                if match[0].lstrip().startswith("#"):
                    reviewed_parser = (path == "rullst-orm/fuzz/fuzz_targets/fuzz_parser.rs"
                                       and relative == "../../../rullst-orm-macros/src/parser.rs")
                    if ".." in relative.split("/") and not reviewed_parser:
                        raise UnprovenScope("ambiguous module path requires global attribution")
                    # #[path] inside nested/inline modules can resolve from a
                    # module directory. Conservatively account for both bases.
                    if not reviewed_parser:
                        origins.append(path.removesuffix(".rs"))
                for origin in origins:
                    included = posixpath.normpath(origin + "/" + relative)
                    if included.startswith("../"):
                        raise UnprovenScope("source inclusion escapes the repository")
                    if included.endswith(".rs") and not any(
                            included.startswith(d + "/" + prefix)
                            for d in self.directories for prefix in ("src/", "fuzz_targets/")):
                        raise UnprovenScope("Rust inclusion outside reviewed source directories")
                    self.included_files.add(included)
                    dependency = self.owner(included)
                    if dependency is not None:
                        self.edges[owner].add(dependency)
                    # Files outside packages retain their global identity.

    def recipients(self, path: str) -> set[str] | None:
        # Cargo configuration, manifests, compile-time macro code and unknown
        # paths remain global. Complete package contents otherwise travel with
        # every conservative transitive consumer, including tests and assets.
        owner = self.owner(path)
        if (owner not in self.scoped_packages or owner in self.macros or path.endswith("/Cargo.toml")
                or "/.cargo/" in path):
            return None
        return self.consumers[owner] | self.unproven_consumers
