"""Per-fuzz-package input scope derived from the Cargo path-dependency graph.

A scope is the set of package directories a fuzz workspace reaches through
path dependencies, read from one commit's Git objects, plus files outside them
that their sources include. Build-time code or anything not resolvable
statically records a reason: that package's input becomes the whole
repository (fail closed). Presence is preferred over modelling.
"""

from __future__ import annotations

import bisect
import posixpath
import tomllib
from pathlib import Path

from fuzz_source_scan import (FILE_ATTRIBUTES, FILE_MACROS, INCLUDES, Facts, ancestors, builtin, cat_file,
                              climbs, join, resolve_path, source_facts, under)

DEPENDENCY_TABLES = ("dependencies", "dev-dependencies", "dev_dependencies")
BUILD_TABLES = ("build-dependencies", "build_dependencies")
TARGET_TABLES = ("lib", "bin", "example", "test", "bench")
# Files in a parent directory of a reached package that change how it builds.
ANCESTOR_FILES = ("rust-toolchain", "rust-toolchain.toml", ".cargo")
CARGO_CONFIGS = (".cargo/config", ".cargo/config.toml")
# Cargo configuration that cannot point at other repository files. Any other
# key (paths, patch, source, env, target linker/runner, ...) fails closed.
CARGO_CONFIG_KEYS = {"alias": None, "term": None, "net": None, "cargo-new": None, "profile": None,
                     "build": {"jobs", "rustflags", "rustdocflags", "incremental"}}
# Proc macros run inside the compiler. Filesystem, process or environment
# access is accepted only in these reviewed blobs. `rullst-macros`' i18n
# `catalog!` reads `<invoking package>/<dir>/*.toml` (default `locales`); the
# package is part of the scope, and a `dir` that climbs with `..` fails closed
# (path-like argument rule). The test modules are `#[cfg(test)]`. Any other
# blob fails closed until reviewed; keep earlier blobs while history uses them.
REVIEWED_COMPILE_IO = {
    "rullst-macros/src/i18n/catalog.rs": frozenset({"fbbfbb76f1e80bfc9938675c6b172dee7ee97d18"}),
    "rullst-macros/src/i18n/catalog_tests.rs": frozenset({"9129f8024076e3a971c430fd70babe7779932f80"}),
    "rullst-macros/src/i18n/emit.rs": frozenset({"19fbff453d52faa3263078214dd7d2cf1eae2224"}),
    "rullst-macros/src/i18n/tests.rs": frozenset({"6327b1f10dd2daf52ee3a9834e773a182dbc47ca"}),
}
# Registry macros reviewed never to read files, by full path.
REVIEWED_PLAIN_MACROS = frozenset({"serde_json::json", "libfuzzer_sys::fuzz_target"})
# The only registry crates a proc macro may link (what rullst-macros and
# rullst-orm-macros use today); anything else could read files for it.
REVIEWED_PROC_MACRO_DEPENDENCIES = frozenset({"syn", "quote", "proc-macro2", "toml"})
# Cargo test, bench and example targets are never part of a proc-macro library.
NON_LIBRARY_DIRECTORIES = ("tests", "benches", "examples")
MAX_ROUNDS = 8

_FACTS: dict[str, Facts] = {}
_TOML: dict[str, object] = {}


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
            if not (isinstance(found, dict) and isinstance(found.get("workspace"), dict)):
                return None, "explicit workspace root is not a workspace"
        elif isinstance(manifest.get("workspace"), dict):
            return directory, None
        else:
            for root in ancestors(directory):
                found = self.manifest(root)
                if found == "invalid":
                    return None, f"unparsable manifest {root or '.'}/Cargo.toml"
                if isinstance(found, dict) and isinstance(found.get("workspace"), dict):
                    break
            else:
                return None, None
        # Cargo skips a workspace that excludes the package; do not model it.
        exclude = found["workspace"].get("exclude", [])
        if not isinstance(exclude, list) or any(
                join(root, entry) is None or under(directory, join(root, entry)) for entry in exclude):
            return None, f"excluded from the workspace at {root or '.'}"
        return root, None

    def package(self, directory: str) -> dict:
        """Edges and properties of one package, cached for every fuzz scope."""
        if directory in self.info:
            return self.info[directory]
        info = {"edges": [], "reasons": [], "files": set(), "proc_macro": False, "lib": "src/lib.rs",
                "crates": set()}
        self.info[directory] = info
        label = directory or "."
        manifest = self.manifest(directory)
        if not isinstance(manifest, dict) or not isinstance(manifest.get("package"), dict):
            info["reasons"].append(f"{label}: path dependency without a package manifest")
            return info
        info["crates"].add(str(manifest["package"].get("name", "")).replace("-", "_"))
        root, problem = self.workspace_root(directory, manifest)
        if problem:
            info["reasons"].append(f"{label}: {problem}")
        if root and root != directory:
            info["files"].add(f"{root}/Cargo.toml")
        if root is None and _inherits(manifest):
            info["reasons"].append(f"{label}: workspace inheritance without a workspace root")
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
                kinds = target.get("crate-type", target.get("crate_type", []))
                if table == "lib" and (target.get("proc-macro") or target.get("proc_macro")
                                       or isinstance(kinds, list) and "proc-macro" in kinds):
                    info["proc_macro"] = True
                if table == "lib" and isinstance(target.get("name"), str):
                    info["crates"].add(target["name"])
        # (linked into the library, table); dev, patch and replace tables are not.
        tables = [(name == "dependencies", manifest.get(name)) for name in DEPENDENCY_TABLES]
        build_tables = [manifest.get(name) for name in BUILD_TABLES]
        for section in ("target", "patch"):
            value = manifest.get(section, {})
            if not isinstance(value, dict):
                tables.append((False, value))
                continue
            for nested in value.values():
                if section == "patch" or not isinstance(nested, dict):
                    tables.append((False, nested))
                else:
                    tables.extend((name == "dependencies", nested.get(name)) for name in DEPENDENCY_TABLES)
                    build_tables.extend(nested.get(name) for name in BUILD_TABLES)
        tables.append((False, manifest.get("replace")))
        # Build-time code (build scripts and their dependencies) is not modelled.
        if any(table is not None for table in build_tables):
            info["reasons"].append(f"{label}: build-dependencies run at build time")
        workspace = self.manifest(root).get("workspace") if root is not None else None
        inherited = workspace.get("dependencies") if isinstance(workspace, dict) else None
        for linked, table in tables:
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
                if not isinstance(spec, (str, dict)) or isinstance(spec, dict) and spec.get("workspace") is not None:
                    info["reasons"].append(f"{label}: unsupported dependency {name}")
                elif isinstance(spec, dict) and "path" in spec:
                    target = join(base, spec["path"])
                    info["crates"].add(name.replace("-", "_"))
                    if target is None:
                        info["reasons"].append(f"{label}: dependency {name} path cannot be resolved")
                    else:
                        info["edges"].append(target)
                    if info["proc_macro"]:
                        # A helper crate would hide the proc macro's I/O from review.
                        info["reasons"].append(f"{label}: proc macro has path dependency {name}")
                elif info["proc_macro"] and linked:
                    crate = spec.get("package", name) if isinstance(spec, dict) else name
                    if crate not in REVIEWED_PROC_MACRO_DEPENDENCIES:
                        info["reasons"].append(f"{label}: proc macro depends on unreviewed crate {crate}")
        build = manifest["package"].get("build")
        script = f"{directory}/build.rs" if directory else "build.rs"
        if build is not False and (build is not None or script in self.files):
            info["reasons"].append(f"{label}: build script runs at build time")
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
        self.checks(scope, facts, edges)

    def checks(self, scope: Scope, facts: dict[str, Facts], edges: set) -> None:
        for path in self.sources(scope.directory):
            if facts[path].runtime_io:
                scope.close(f"{path}: fuzz harness accesses files, processes or the environment")
        for package in sorted(scope.packages):
            info = self.package(package)
            if not info["proc_macro"]:
                continue
            # Everything the proc-macro library compiles, including `#[path]`
            # and `include!` targets outside its directory.
            excluded = [f"{package}/{name}" if package else name for name in NON_LIBRARY_DIRECTORIES
                        if not under(info["lib"], name)]
            compiled = {path for path in self.sources(package)
                        if not any(under(path, item) for item in excluded)}
            while True:
                found = {target for source, target in edges if source in compiled and target in facts}
                found |= {path for target in found if facts[target].mods and not under(target, package)
                          for path in self.below(posixpath.dirname(target)) if path in facts}
                if found <= compiled:
                    break
                compiled |= found
            for path in sorted(compiled):
                item = facts[path]
                if item.compile_io and self.files[path][1] not in REVIEWED_COMPILE_IO.get(path, ()):
                    uses = ", ".join(sorted(set(item.compile_io)))
                    scope.close(f"{path}: unreviewed compile-time access ({uses})")
                for chain, value, position, compared in item.literals:
                    # A proc macro must not hand a climbing path to any reader
                    # (comparing a segment with ".." is not reading one).
                    owner = chain[-1][1].rsplit("::", 1)[-1] if chain else None
                    if (climbs(value) and not compared and not item.inside(position, item.quotes)
                            and owner not in {"path", *INCLUDES}):
                        scope.close(f"{path}: proc macro uses path {value!r}")

    def resolve(self, scope: Scope, facts: dict[str, Facts]) -> tuple[set[str], set]:
        calls, aliases, renames, added, edges = {}, set(), [], set(), set()
        defined, opaque, exports, carriers = set(), set(), set(), set()
        proc_macros = {package for package in scope.packages if self.package(package)["proc_macro"]}
        # Paths that name a workspace macro rather than a registry one.
        local = {"crate", "$crate", "self", "super"}
        for package in scope.packages:
            local |= self.package(package)["crates"]
        for path, item in facts.items():
            if item.error:
                scope.close(f"{path}: cannot scan source ({item.error})")
                continue
            for call in item.calls:
                calls.setdefault(call[0], []).append((path, *call[1:]))
            aliases |= item.aliases
            renames += item.renames
            if any(under(path, package) for package in proc_macros):
                exports |= item.exports
            if item.defines_macros:
                scope.close(f"{path}: macro_rules! that defines macros")
            for name in sorted(INCLUDES.keys() & item.aliases):
                scope.close(f"{path}: built-in {name}! is renamed")
            for original, _ in item.renames:
                if original in FILE_MACROS or original in FILE_ATTRIBUTES:
                    scope.close(f"{path}: file-reading {original} is renamed")
            for name, *_ in item.calls:
                if name in FILE_MACROS:
                    scope.close(f"{path}: {name}! reads files at compile time")
            for name, *_ in item.attrs:
                if name.rsplit("::", 1)[-1] in FILE_ATTRIBUTES:
                    scope.close(f"{path}: #[{name}] reads files at compile time")
            for name, start, end in item.defs:
                defined.add(name)
                # Transparent: arguments only reach built-ins (or plain code).
                if (any(start < open_ < end and dollar and not builtin("call", resolve_path(written, item.imports))
                        for _, open_, _, _, dollar, _, written in item.calls)
                        or any(start < open_ < end and dollar and not builtin("attr", resolve_path(written, item.imports))
                               for written, open_, _, dollar in item.attrs)):
                    opaque.add(name)
                # Carrier: its body holds a path-like literal.
                if any(start < position < end for _, _, position, _ in item.literals):
                    carriers.add(name)
        for original, alias in renames:
            if original in exports:
                exports.add(alias)
            if original == "self" or original in local:  # `extern crate self as rullst;`
                local.add(alias)
        transparent = defined - opaque

        def trusted(kind: str, written: str, imports: dict, value: str | None) -> bool:
            full = resolve_path(written, imports)
            prefix, _, name = full.rpartition("::")
            workspace = not prefix or prefix.split("::", 1)[0] in local
            if builtin(kind, full) or kind == "call" and (
                    full in REVIEWED_PLAIN_MACROS or workspace and name in transparent):
                return True
            # Workspace proc macros are I/O-checked; routes are fine, `..` never.
            return not climbs(value) and workspace and name in exports

        for path, item in facts.items():
            for chain, value, *_ in item.literals:
                # Path-like arguments reaching any macro or attribute whose code
                # is not scanned (registry crates) may name files: fail closed.
                for kind, written in chain:
                    if not trusted(kind, written, item.imports, value):
                        scope.close(f"{path}: path-like argument {value!r} of {written}")
                        break
            for name, _, _, _, _, nested, written in item.calls:
                # A path produced by a nested workspace macro is still a path.
                if set(nested) & carriers and not trusted("call", written, item.imports, "/"):
                    scope.close(f"{path}: {written}! receives a path from a nested macro")

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
            if value is None or "\\" in value or value.startswith("/") or not value.strip("./"):
                scope.close(f"{source}: unresolvable {kind} path {value!r}")
                return
            if kind == "mod":
                # Inline modules make the base deeper; consider every depth.
                parts = posixpath.normpath(value).split("/")
                ups = 0
                while ups < len(parts) and parts[ups] == "..":
                    ups += 1
                rest = "/".join(parts[ups:])
                candidates = {join(directory, "/".join([".."] * up + [rest])) for up in range(ups + 1)}
                if join(directory, value) is None:
                    candidates.add(None)
            else:
                candidates = {join(directory, value)}
            for candidate in candidates:
                if candidate is None:
                    scope.close(f"{source}: {kind} path {value!r} cannot be resolved")
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
                    if form in {"lit", "manifest"} and (climbs(value) or form == "lit" and value.startswith("/")):
                        scope.close(f"{path}: generated {kind} path {value!r} may leave its package")
                elif form == "out":
                    continue
                elif form == "dynamic" or form == "manifest" and macro is not None:
                    scope.close(f"{path}: {kind} path cannot be resolved statically")
                elif form == "manifest":
                    owners = [package for package in scope.packages if under(path, package)]
                    if not value.startswith("/") or not owners:
                        scope.close(f"{path}: CARGO_MANIFEST_DIR {kind} path {value!r} cannot be resolved")
                    for package in owners if value.startswith("/") else ():
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
                elif macro in aliases:
                    scope.close(f"{path}: {kind} argument of renamed macro {macro}!")
                else:
                    for site, start, _, strings, dollar, nested, _ in calls.get(macro, ()):
                        outer = facts[site].macro_at(start)
                        if outer == macro:
                            continue
                        bases = {site} if outer is None else sites(outer, frozenset({macro}))
                        if (bases is None or nested or outer is not None and dollar
                                or facts[site].inside(start, facts[site].quotes)):
                            scope.close(f"{path}: {kind} argument of macro {macro}! cannot be resolved")
                            break
                        for base in bases:
                            for string in strings:
                                target(kind, base, string)
        return added, edges
