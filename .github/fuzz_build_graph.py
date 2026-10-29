"""Per-fuzz-package input scope derived from the Cargo path-dependency graph.

Everything is read from the Git objects of one commit, never a checkout. A
scope is the set of package directories a fuzz workspace reaches through path
dependencies, plus files outside them that their sources, build scripts or
reviewed proc macros reference. Anything that cannot be resolved statically
records a reason; the caller then treats the whole repository as that fuzz
package's input (fail closed).
"""

from __future__ import annotations

import bisect
import posixpath
import subprocess
import tomllib
from pathlib import Path

from fuzz_source_scan import Facts, source_facts

DEPENDENCY_TABLES = ("dependencies", "dev-dependencies", "dev_dependencies",
                     "build-dependencies", "build_dependencies")
TARGET_TABLES = ("lib", "bin", "example", "test", "bench")
# Files in a parent directory of a reached package that change its checkout or build.
ANCESTOR_FILES = (".gitattributes", "rust-toolchain", "rust-toolchain.toml", ".cargo")
CARGO_CONFIGS = (".cargo/config", ".cargo/config.toml")
# Cargo configuration that cannot point at other repository files. Any other
# key (paths, patch, source, env, target linker/runner, ...) fails closed.
CARGO_CONFIG_KEYS = {"alias": None, "term": None, "net": None, "cargo-new": None, "profile": None,
                     "build": {"jobs", "rustflags", "rustdocflags", "incremental"}}
# Proc macros run inside the compiler. Filesystem, process or environment
# access is accepted only in these reviewed blobs. `rullst-macros`' i18n
# `catalog!` reads `<invoking package>/<dir>/*.toml` (default `locales`); the
# package is part of the scope and a `dir = "…"` that leaves it is resolved by
# the path-argument scan. The test modules are `#[cfg(test)]`. Any other blob
# fails closed until reviewed; keep earlier blobs while history may use them.
REVIEWED_COMPILE_IO = {
    "rullst-macros/src/i18n/catalog.rs": frozenset({"fbbfbb76f1e80bfc9938675c6b172dee7ee97d18"}),
    "rullst-macros/src/i18n/catalog_tests.rs": frozenset({"9129f8024076e3a971c430fd70babe7779932f80"}),
    "rullst-macros/src/i18n/emit.rs": frozenset({"19fbff453d52faa3263078214dd7d2cf1eae2224"}),
    "rullst-macros/src/i18n/tests.rs": frozenset({"6327b1f10dd2daf52ee3a9834e773a182dbc47ca"}),
}
# Cargo test, bench and example targets are never part of a proc-macro library.
NON_LIBRARY_DIRECTORIES = ("tests", "benches", "examples")
MAX_ROUNDS = 8

_FACTS: dict[str, Facts] = {}
_TOML: dict[str, object] = {}


def under(path: str, directory: str) -> bool:
    return directory == "" or path == directory or path.startswith(directory + "/")


def join(base: str, relative: object) -> str | None:
    """Normalized repository path, or None for absolute, odd or escaping paths."""
    if (not isinstance(relative, str) or relative.startswith("/") or "\\" in relative
            or "\0" in relative):
        return None
    joined = posixpath.normpath(posixpath.join(base, relative) if base else relative)
    if joined == ".":
        return ""
    if joined == ".." or joined.startswith("../"):
        return None
    return joined


def ancestors(path: str) -> list[str]:
    result = []
    while path:
        path = posixpath.dirname(path)
        result.append(path)
    return result


def cat_file(root: Path, oids) -> dict[str, bytes]:
    oids = list(dict.fromkeys(oids))
    if not oids:
        return {}
    output = subprocess.run(["git", "cat-file", "--batch"], cwd=root, check=True,
                            capture_output=True, input="".join(f"{oid}\n" for oid in oids).encode()).stdout
    result, position = {}, 0
    for oid in oids:
        end = output.index(b"\n", position)
        name, kind, size = output[position:end].decode().split()
        if name != oid or kind != "blob":
            raise ValueError("unexpected Git object")
        result[oid] = output[end + 1:end + 1 + int(size)]
        position = end + 2 + int(size)
    return result


def _inherits(value: object) -> bool:
    if isinstance(value, dict):
        return value.get("workspace") is True or any(_inherits(item) for item in value.values())
    return isinstance(value, list) and any(_inherits(item) for item in value)


def _config_problem(config: object) -> str | None:
    if not isinstance(config, dict):
        return "unparsable"
    for key, value in config.items():
        allowed = CARGO_CONFIG_KEYS.get(key, False)
        if allowed is False or allowed and (not isinstance(value, dict) or set(value) - allowed):
            return f"key {key!r}"
    return None


class Scope:
    def __init__(self, directory: str):
        self.directory = directory
        self.packages: set[str] = set()   # reached package directories, all files below
        self.files: set[str] = set()      # referenced paths, the path itself and all below
        self.reasons: list[str] = []      # non-empty: the whole repository is the input

    def covers(self, path: str) -> bool:
        return any(under(path, item) for item in (*self.packages, *self.files))

    def close(self, reason: str) -> None:
        if reason not in self.reasons:
            self.reasons.append(reason)


class Graph:
    def __init__(self, files: dict[str, tuple[str, str]], root: Path, directories):
        self.files, self.root, self.directories = files, root, sorted(directories)
        self.paths = sorted(files)
        self.info: dict[str, dict] = {}

    def below(self, directory: str) -> list[str]:
        if not directory:
            return self.paths
        start = bisect.bisect_left(self.paths, directory)
        end = bisect.bisect_left(self.paths, directory + "0")
        return [path for path in self.paths[start:end] if under(path, directory)]

    def sources(self, package: str) -> list[str]:
        """Rust sources below a package; fuzz corpora/artifacts are run-time data."""
        data = [f"{directory}/{name}" for directory in self.directories for name in ("corpus", "artifacts")]
        return [path for path in self.below(package)
                if path.endswith(".rs") and not any(under(path, item) for item in data)]

    def facts(self, paths) -> dict[str, Facts]:
        paths = [path for path in paths if path in self.files]
        missing = [self.files[path][1] for path in paths if self.files[path][1] not in _FACTS]
        for oid, data in cat_file(self.root, missing).items():
            _FACTS[oid] = source_facts(data)
        return {path: _FACTS[self.files[path][1]] for path in paths}

    def toml(self, path: str) -> object:
        if path not in self.files:
            return None
        oid = self.files[path][1]
        if oid not in _TOML:
            try:
                _TOML[oid] = tomllib.loads(cat_file(self.root, [oid])[oid].decode("utf-8-sig"))
            except (UnicodeDecodeError, tomllib.TOMLDecodeError):
                _TOML[oid] = "invalid"
        return _TOML[oid]

    def manifest(self, directory: str) -> object:
        return self.toml(f"{directory}/Cargo.toml" if directory else "Cargo.toml")

    def workspace_root(self, directory: str, manifest: dict) -> tuple[str | None, str | None]:
        explicit = manifest["package"].get("workspace")
        if explicit is not None:
            root = join(directory, explicit)
            found = self.manifest(root) if root is not None else None
            if isinstance(found, dict) and isinstance(found.get("workspace"), dict):
                return root, None
            return None, "explicit workspace root is not a workspace"
        if isinstance(manifest.get("workspace"), dict):
            return directory, None
        for parent in ancestors(directory):
            found = self.manifest(parent)
            if found == "invalid":
                return None, f"unparsable manifest {parent or '.'}/Cargo.toml"
            if isinstance(found, dict) and isinstance(found.get("workspace"), dict):
                return parent, None
        return None, None

    def package(self, directory: str) -> dict:
        """Edges and properties of one package, cached for every fuzz scope."""
        if directory in self.info:
            return self.info[directory]
        info = {"edges": [], "reasons": [], "files": set(), "build": None, "proc_macro": False,
                "lib": "src/lib.rs"}
        self.info[directory] = info
        label = directory or "."
        manifest = self.manifest(directory)
        if not isinstance(manifest, dict) or not isinstance(manifest.get("package"), dict):
            info["reasons"].append(f"{label}: path dependency without a package manifest")
            return info
        root, problem = self.workspace_root(directory, manifest)
        if problem:
            info["reasons"].append(f"{label}: {problem}")
        if root and root != directory:
            info["files"].add(f"{root}/Cargo.toml")
        if root is None and _inherits(manifest):
            info["reasons"].append(f"{label}: workspace inheritance without a workspace root")
        tables = [manifest.get(name) for name in DEPENDENCY_TABLES]
        for section in ("target", "patch"):
            value = manifest.get(section, {})
            if not isinstance(value, dict):
                tables.append(value)
                continue
            for nested in value.values():
                if section == "patch" or not isinstance(nested, dict):
                    tables.append(nested)
                else:
                    tables.extend(nested.get(name) for name in DEPENDENCY_TABLES)
        tables.append(manifest.get("replace"))
        workspace = self.manifest(root).get("workspace") if root is not None else None
        inherited = workspace.get("dependencies") if isinstance(workspace, dict) else None
        for table in tables:
            if table is None:
                continue
            if not isinstance(table, dict):
                info["reasons"].append(f"{label}: malformed dependency table")
                continue
            for name, spec in table.items():
                base = directory
                if isinstance(spec, dict) and spec.get("workspace") is True:
                    spec = inherited.get(name) if isinstance(inherited, dict) else None
                    base = root
                    if spec is None:
                        info["reasons"].append(f"{label}: unresolved inherited dependency {name}")
                        continue
                if isinstance(spec, str):
                    continue
                if not isinstance(spec, dict) or spec.get("workspace") is not None:
                    info["reasons"].append(f"{label}: unsupported dependency {name}")
                elif "path" in spec:
                    target = join(base, spec["path"])
                    if target is None:
                        info["reasons"].append(f"{label}: dependency {name} path leaves the repository")
                    else:
                        info["edges"].append(target)
        for table in TARGET_TABLES:
            targets = manifest.get(table, [])
            for target in targets if isinstance(targets, list) else [targets]:
                if not isinstance(target, dict):
                    info["reasons"].append(f"{label}: malformed [{table}] target")
                    continue
                if "path" in target:
                    resolved = join(directory, target["path"])
                    if resolved is None or not under(resolved, directory):
                        info["reasons"].append(f"{label}: [{table}] source outside the package")
                    elif table == "lib":
                        info["lib"] = posixpath.relpath(resolved, directory or ".")
                if table == "lib" and (target.get("proc-macro") or target.get("proc_macro")):
                    info["proc_macro"] = True
        build = manifest["package"].get("build")
        if isinstance(build, str):
            resolved = join(directory, build)
            if resolved is None or not under(resolved, directory):
                info["reasons"].append(f"{label}: build script outside the package")
            else:
                info["build"] = resolved
        elif build is None or build is True:
            default = f"{directory}/build.rs" if directory else "build.rs"
            info["build"] = default if default in self.files else None
        elif build is not False:
            info["reasons"].append(f"{label}: malformed build setting")
        return info

    def scopes(self) -> dict[str, Scope]:
        result = {}
        for directory in self.directories:
            scope = Scope(directory)
            queue = [directory]
            while queue:
                package = queue.pop()
                if package in scope.packages:
                    continue
                scope.packages.add(package)
                info = self.package(package)
                for reason in info["reasons"]:
                    scope.close(reason)
                scope.files.update(info["files"])
                queue.extend(info["edges"])
            for package in list(scope.packages):
                for parent in ancestors(package)[:-1]:
                    scope.files.update(f"{parent}/{name}" for name in ANCESTOR_FILES)
            # Cargo reads configuration from the fuzz directory and every parent.
            for parent in [directory, *ancestors(directory)]:
                for name in CARGO_CONFIGS:
                    path = f"{parent}/{name}" if parent else name
                    if path in self.files:
                        problem = _config_problem(self.toml(path))
                        if problem:
                            scope.close(f"{path}: unsupported Cargo configuration {problem}")
            if not scope.reasons:
                self.scan(scope)
            result[directory] = scope
        return result

    def scan(self, scope: Scope) -> None:
        code = {path for package in scope.packages for path in self.sources(package)}
        for _ in range(MAX_ROUNDS):
            facts = self.facts(code)
            added, edges = self.resolve(scope, facts)
            for path, item in facts.items():
                # Modules of a referenced file outside the packages live below it.
                if item.mods and not any(under(path, package) for package in scope.packages):
                    tree = posixpath.dirname(path)
                    if not tree:
                        scope.close(f"{path}: root-level module tree")
                        continue
                    scope.files.add(tree)
                    added.update(source for source in self.below(tree) if source.endswith(".rs"))
            if added <= code:
                break
            code |= added
        else:
            scope.close("too many nested source inclusions")
            return
        self.compile_time(scope, facts, edges)

    def compile_time(self, scope: Scope, facts: dict[str, Facts], edges: set) -> None:
        for path in self.sources(scope.directory):
            if facts[path].runtime_io:
                scope.close(f"{path}: fuzz harness accesses files, processes or the environment")
        for package in sorted(scope.packages):
            info = self.package(package)
            build = info["build"]
            if build is not None:
                item = self.facts([build]).get(build)
                if item is None or item.error or item.mods:
                    scope.close(f"{build}: build script with modules or unreadable source")
                else:
                    for value in item.build_paths:
                        target = join(package, value) if value is not None else None
                        if target is None:
                            scope.close(f"{build}: build script path {value!r} leaves the repository")
                        elif not scope.covers(target):
                            scope.files.add(target)
                    for operation in sorted(set(item.build_dynamic)):
                        scope.close(f"{build}: build script uses {operation}")
            if not info["proc_macro"]:
                continue
            excluded = {f"{package}/{name}" if package else name for name in NON_LIBRARY_DIRECTORIES
                        if not under(info["lib"], name)}
            if any(not any(under(source, item) for item in excluded)
                   and any(under(target, item) for item in excluded)
                   for source, target in edges if under(source, package)):
                excluded = set()
            for path in self.sources(package):
                if (not any(under(path, item) for item in excluded)
                        and facts[path].compile_io
                        and self.files[path][1] not in REVIEWED_COMPILE_IO.get(path, ())):
                    uses = ", ".join(sorted(set(facts[path].compile_io)))
                    scope.close(f"{path}: unreviewed compile-time access ({uses})")

    def resolve(self, scope: Scope, facts: dict[str, Facts]) -> tuple[set[str], set]:
        calls, aliases, added, edges = {}, set(), set(), set()
        for path, item in facts.items():
            if item.error:
                scope.close(f"{path}: cannot scan source ({item.error})")
                continue
            for call in item.calls:
                calls.setdefault(call[0], []).append((path, *call[1:]))
            aliases |= item.aliases

        def sites(name: str, seen: frozenset) -> set[str] | None:
            """Files whose location resolves paths expanded from `name!`."""
            if name in aliases:
                return None
            result = set()
            for path, start, *_ in calls.get(name, ()):
                outer = facts[path].macro_at(start)
                if facts[path].inside(start, facts[path].quotes):
                    return None
                if outer is None:
                    result.add(path)
                elif outer != name and outer not in seen:
                    nested = sites(outer, seen | {name})
                    if nested is None:
                        return None
                    result |= nested
            return result

        def target(kind: str, source: str, value: str | None) -> None:
            directory = posixpath.dirname(source)
            if kind == "key":
                # Only a relative argument that climbs can leave its package;
                # absolute keyword values are routes, not repository files.
                if value is None or value.startswith("/") or ".." not in value.split("/"):
                    return
                candidates = {join(directory, value)} | {
                    join(package, value) for package in scope.packages if under(source, package)}
            elif value is None or "\\" in value or value.startswith("/") or not value.strip("./"):
                scope.close(f"{source}: unresolvable {kind} path {value!r}")
                return
            elif kind == "mod":
                # Inline modules make the base deeper; consider every depth.
                parts = posixpath.normpath(value).split("/")
                ups = 0
                while ups < len(parts) and parts[ups] == "..":
                    ups += 1
                rest = "/".join(parts[ups:])
                candidates = {join(directory, "/".join([".."] * up + [rest])) for up in range(ups + 1)}
            else:
                candidates = {join(directory, value)}
            for candidate in candidates:
                if candidate is None:
                    scope.close(f"{source}: {kind} path {value!r} leaves the repository")
                    continue
                if kind in {"code", "mod"}:
                    edges.add((source, candidate))
                    if candidate in self.files:
                        added.add(candidate)  # scan included code, even a non-.rs file
                if not scope.covers(candidate):
                    scope.files.add(candidate)

        for path, item in facts.items():
            if item.error:
                continue
            for kind, form, value, position in item.refs:
                macro = item.macro_at(position)
                if item.inside(position, item.quotes):
                    # Generated code resolves against an unknown invocation site;
                    # computed paths are proc-macro compile-time access.
                    if (form in {"lit", "manifest"} and kind != "key" and value is not None
                            and (".." in value.split("/") or form == "lit" and value.startswith("/"))):
                        scope.close(f"{path}: generated {kind} path {value!r} may leave its package")
                elif form == "out" or form == "dynamic" and kind == "key":
                    continue
                elif form == "dynamic" or form == "manifest" and macro is not None:
                    scope.close(f"{path}: {kind} path cannot be resolved statically")
                elif form == "manifest" and not value.startswith("/"):
                    scope.close(f"{path}: {kind} path {value!r} after CARGO_MANIFEST_DIR")
                elif form == "manifest":
                    for package in [package for package in scope.packages if under(path, package)]:
                        target(kind, f"{package}/Cargo.toml" if package else "Cargo.toml", value[1:])
                elif macro is None and form == "meta":
                    scope.close(f"{path}: macro variable outside macro_rules!")
                elif macro is None:
                    target(kind, path, value)
                elif form == "lit":
                    bases = sites(macro, frozenset())
                    if bases is None:
                        scope.close(f"{path}: {kind} path inside macro {macro}! has unknown call sites")
                    for base in bases or ():
                        target(kind, base, value)
                else:
                    for site, start, _, strings, dollar in calls.get(macro, ()):
                        outer = facts[site].macro_at(start)
                        if outer == macro:
                            continue
                        bases = {site} if outer is None else sites(outer, frozenset({macro}))
                        if (macro in aliases or bases is None or outer is not None and dollar
                                or facts[site].inside(start, facts[site].quotes)):
                            scope.close(f"{path}: {kind} argument of macro {macro}! cannot be resolved")
                            break
                        for base in bases:
                            for string in strings:
                                target(kind, base, string)
        return added, edges
