#!/usr/bin/env python3
"""Generate or check the committed public-API listing of each library crate.

    python3 .github/public-api.py                # rewrite .github/public-api/*.txt
    python3 .github/public-api.py --check        # fail when a listing differs
    python3 .github/public-api.py rullst-iot     # only the named packages

Each listing is rendered from the rustdoc JSON of the package's library, built
with the pinned nightly below and the features and rustdoc arguments of its
`[package.metadata.docs.rs]` table, so it covers what docs.rs shows. Items
hidden with `#[doc(hidden)]` are not listed. An item reachable at several
paths is listed in full at its shortest path; each other path is a
`pub use <path> = <kind> <listed path>` line. Signatures name the types
through those listed paths, or through the defining crate's path for other
crates. Blanket implementations are left out.
"""

from __future__ import annotations

import argparse
import difflib
import json
import os
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SNAPSHOTS = ROOT / ".github" / "public-api"
TOOLCHAIN = "nightly-2026-08-21"
# rustdoc JSON format of TOOLCHAIN; checked so a toolchain bump is deliberate.
FORMAT_VERSION = 61
EXCLUDED = {
    # Its crate documentation says that the library is the CLI's implementation,
    # not a framework API; the supported interface is the commands.
    "cargo-rullst": "the CLI implementation library is not a framework API",
}
# Stable auto traits; the toolchain's unstable ones (Freeze, UnsafeUnpin) and
# the derive-internal StructuralPartialEq are left out.
AUTO_TRAITS = {"Send", "Sync", "Unpin", "UnwindSafe", "RefUnwindSafe"}
UNSTABLE_TRAITS = {"Freeze", "UnsafeUnpin", "StructuralPartialEq"}
SKIPPED_KINDS = {"impl", "primitive", "struct_field", "variant"}


def one(value):
    """The single (key, value) pair of a serde enum, or (name, None) for a unit variant."""
    if isinstance(value, str):
        return value, None
    ((key, inner),) = value.items()
    return key, inner


class Listing:
    def __init__(self, krate: dict):
        if krate["format_version"] != FORMAT_VERSION:
            raise SystemExit(
                f"rustdoc JSON format {krate['format_version']} is not {FORMAT_VERSION}; "
                f"build it with {TOOLCHAIN} or update FORMAT_VERSION with the toolchain"
            )
        self.index = krate["index"]
        self.paths = krate["paths"]
        self.root = str(krate["root"])
        self.found: dict[str, set[str]] = defaultdict(set)
        self.external: list[tuple[str, dict]] = []
        self.canonical: dict[str, str] = {}
        self.unnamed: dict[str, set[str]] = defaultdict(set)
        self.context = ""
        self.attached: set[str] = set()

    # Pass 1: every public path of every item.

    def collect(self, module: str, prefix: str, stack: frozenset) -> None:
        for child in self.index[module]["inner"]["module"]["items"]:
            item = self.index.get(str(child))
            if item is None:
                continue
            kind, inner = one(item["inner"])
            if kind == "use":
                if item["visibility"] == "public":
                    self.collect_use(inner, prefix, stack)
            elif kind not in SKIPPED_KINDS and item["visibility"] == "public":
                self.add(str(child), f"{prefix}::{item['name']}", stack)

    def collect_use(self, use: dict, prefix: str, stack: frozenset) -> None:
        target = str(use["id"]) if use["id"] is not None else None
        local = target if target in self.index else None
        if use["is_glob"]:
            if local and one(self.index[local]["inner"])[0] == "module":
                if local not in stack:
                    self.collect(local, prefix, stack | {local})
            else:
                self.external.append((f"{prefix}::*", use))
        elif local:
            self.add(local, f"{prefix}::{use['name']}", stack)
        else:
            self.external.append((f"{prefix}::{use['name']}", use))

    def add(self, item_id: str, path: str, stack: frozenset) -> None:
        self.found[item_id].add(path)
        if one(self.index[item_id]["inner"])[0] == "module" and item_id not in stack:
            self.collect(item_id, path, stack | {item_id})

    # Rendering of types, bounds and generics.

    def path(self, path: dict) -> str:
        item_id = str(path["id"])
        if item_id in self.canonical:
            name = self.canonical[item_id]
        elif item_id in self.paths:
            name = "::".join(self.paths[item_id]["path"])
            if self.paths[item_id]["crate_id"] == 0:
                self.unnamed[name].add(self.context)
        else:
            name = path["path"]
        return name + self.args(path.get("args"))

    def args(self, args) -> str:
        if not args:
            return ""
        kind, inner = one(args)
        if kind == "angle_bracketed":
            parts = [self.arg(arg) for arg in inner["args"]]
            parts += [self.constraint(c) for c in inner["constraints"]]
            return f"<{', '.join(parts)}>" if parts else ""
        if kind == "parenthesized":
            output = f" -> {self.ty(inner['output'])}" if inner["output"] else ""
            return f"({', '.join(map(self.ty, inner['inputs']))}){output}"
        return "(..)"

    def arg(self, arg) -> str:
        kind, inner = one(arg)
        if kind == "lifetime":
            return inner
        if kind == "type":
            return self.ty(inner)
        if kind == "const":
            return inner["expr"]
        return "_"

    def constraint(self, constraint: dict) -> str:
        head = constraint["name"] + self.args(constraint["args"])
        kind, inner = one(constraint["binding"])
        if kind == "equality":
            return f"{head} = {self.term(inner)}"
        return f"{head}: {self.bounds(inner)}"

    def term(self, term: dict) -> str:
        kind, inner = one(term)
        return self.ty(inner) if kind == "type" else inner["expr"]

    def bounds(self, bounds: list) -> str:
        return " + ".join(self.bound(bound) for bound in bounds)

    def bound(self, bound) -> str:
        kind, inner = one(bound)
        if kind == "trait_bound":
            modifier = {"none": "", "maybe": "?", "maybe_const": "[const] "}[inner["modifier"]]
            return self.hrtb(inner["generic_params"]) + modifier + self.path(inner["trait"])
        if kind == "outlives":
            return inner
        return f"use<{', '.join(next(iter(arg.values())) for arg in inner)}>"

    def hrtb(self, params: list) -> str:
        names = [param["name"] for param in params]
        return f"for<{', '.join(names)}> " if names else ""

    def ty(self, ty) -> str:
        kind, inner = one(ty)
        if kind == "resolved_path":
            return self.path(inner)
        if kind in ("generic", "primitive"):
            return inner
        if kind == "tuple":
            return f"({', '.join(map(self.ty, inner))}{',' if len(inner) == 1 else ''})"
        if kind == "slice":
            return f"[{self.ty(inner)}]"
        if kind == "array":
            return f"[{self.ty(inner['type'])}; {inner['len']}]"
        if kind == "pat":
            return self.ty(inner["type"])
        if kind == "impl_trait":
            return f"impl {self.bounds(inner)}"
        if kind == "raw_pointer":
            return ("*mut " if inner["is_mutable"] else "*const ") + self.ty(inner["type"])
        if kind == "borrowed_ref":
            lifetime = f"{inner['lifetime']} " if inner["lifetime"] else ""
            target = self.ty(inner["type"])
            if " + " in target and target.startswith(("dyn ", "impl ")):
                target = f"({target})"
            return f"&{lifetime}{'mut ' if inner['is_mutable'] else ''}{target}"
        if kind == "qualified_path":
            base = self.ty(inner["self_type"])
            if inner["trait"]:
                base = f"<{base} as {self.path(inner['trait'])}>"
            return f"{base}::{inner['name']}{self.args(inner['args'])}"
        if kind == "dyn_trait":
            parts = [self.hrtb(p["generic_params"]) + self.path(p["trait"]) for p in inner["traits"]]
            if inner["lifetime"]:
                parts.append(inner["lifetime"])
            return "dyn " + " + ".join(parts)
        if kind == "function_pointer":
            return self.hrtb(inner["generic_params"]) + self.header(inner["header"]) + "fn" + self.signature(inner["sig"], named=False)
        return "_"

    def params(self, generics: dict) -> str:
        parts = []
        for param in generics["params"]:
            kind, inner = one(param["kind"])
            if kind == "lifetime":
                outlives = f": {' + '.join(inner['outlives'])}" if inner["outlives"] else ""
                parts.append(param["name"] + outlives)
            elif kind == "type":
                if inner["is_synthetic"]:
                    continue
                text = param["name"]
                if inner["bounds"]:
                    text += f": {self.bounds(inner['bounds'])}"
                if inner["default"]:
                    text += f" = {self.ty(inner['default'])}"
                parts.append(text)
            else:
                default = f" = {inner['default']}" if inner["default"] else ""
                parts.append(f"const {param['name']}: {self.ty(inner['type'])}{default}")
        return f"<{', '.join(parts)}>" if parts else ""

    def where(self, generics: dict) -> str:
        parts = []
        for predicate in generics["where_predicates"]:
            kind, inner = one(predicate)
            if kind == "bound_predicate":
                parts.append(f"{self.hrtb(inner['generic_params'])}{self.ty(inner['type'])}: {self.bounds(inner['bounds'])}")
            elif kind == "lifetime_predicate":
                parts.append(f"{inner['lifetime']}: {' + '.join(inner['outlives'])}")
            else:
                parts.append(f"{self.ty(inner['lhs'])} = {self.term(inner['rhs'])}")
        return f" where {', '.join(parts)}" if parts else ""

    def header(self, header: dict) -> str:
        text = "const " if header["is_const"] else ""
        text += "async " if header["is_async"] else ""
        text += "unsafe " if header["is_unsafe"] else ""
        abi = header["abi"]
        if abi != "Rust":
            name, inner = one(abi)
            name = inner if name == "Other" else name + ("-unwind" if inner and inner.get("unwind") else "")
            text += f'extern "{name}" '
        return text

    def signature(self, sig: dict, named: bool = True) -> str:
        inputs = []
        for name, ty in sig["inputs"]:
            if name == "self" and ty == {"generic": "Self"}:
                inputs.append("self")
            elif name == "self" and "borrowed_ref" in ty and ty["borrowed_ref"]["type"] == {"generic": "Self"}:
                ref = ty["borrowed_ref"]
                lifetime = f"{ref['lifetime']} " if ref["lifetime"] else ""
                inputs.append(f"&{lifetime}{'mut ' if ref['is_mutable'] else ''}self")
            elif named:
                inputs.append(f"{name}: {self.ty(ty)}")
            else:
                inputs.append(self.ty(ty))
        if sig["is_c_variadic"]:
            inputs.append("...")
        output = f" -> {self.ty(sig['output'])}" if sig["output"] is not None else ""
        return f"({', '.join(inputs)}){output}"

    def function(self, path: str, function: dict) -> str:
        generics = function["generics"]
        return (f"{self.header(function['header'])}fn {path}{self.params(generics)}"
                f"{self.signature(function['sig'])}{self.where(generics)}")

    # Pass 2: one block of lines per listed item.

    def attributes(self, item: dict) -> str:
        text = "#[deprecated] " if item["deprecation"] else ""
        return text + ("#[non_exhaustive] " if "non_exhaustive" in item["attrs"] else "")

    def member(self, owner: str, item_id: str) -> str | None:
        item = self.index[item_id]
        kind, inner = one(item["inner"])
        name = f"{owner}::{item['name']}"
        self.context = name
        prefix = self.attributes(item) + "pub "
        if kind == "function":
            return prefix + self.function(name, inner)
        if kind == "assoc_const":
            return f"{prefix}const {name}: {self.ty(inner['type'])}"
        if kind == "assoc_type":
            bounds = f": {self.bounds(inner['bounds'])}" if inner["bounds"] else ""
            default = f" = {self.ty(inner['type'])}" if inner["type"] else ""
            return f"{prefix}type {name}{self.params(inner['generics'])}{bounds}{default}{self.where(inner['generics'])}"
        return None

    def impl(self, impl: dict) -> str:
        generics = impl["generics"]
        head = "unsafe impl" if impl["is_unsafe"] else "impl"
        head += self.params(generics) + " "
        if impl["trait"]:
            head += ("!" if impl["is_negative"] else "") + self.path(impl["trait"]) + " for "
        return head + self.ty(impl["for"]) + self.where(generics)

    def implementations(self, path: str, impl_ids: list, lines: list) -> None:
        inherent: dict[str, list[str]] = defaultdict(list)
        traits, autos = [], []
        for impl_id in impl_ids:
            impl = self.index[str(impl_id)]["inner"]["impl"]
            trait = impl["trait"]["path"].rsplit("::", 1)[-1] if impl["trait"] else None
            if impl["blanket_impl"] is not None or trait in UNSTABLE_TRAITS:
                continue
            if impl["is_synthetic"] and trait not in AUTO_TRAITS:
                continue
            self.context = path
            head = self.impl(impl)
            if impl["is_synthetic"]:
                autos.append(head)
            elif impl["trait"] is None:
                for member in impl["items"]:
                    if self.index[str(member)]["visibility"] == "public":
                        line = self.member(path, str(member))
                        if line:
                            inherent[head].append(line)
            else:
                assoc = []
                for member in impl["items"]:
                    item = self.index.get(str(member))
                    kind, inner = one(item["inner"]) if item else (None, None)
                    if kind == "assoc_type" and inner["type"]:
                        assoc.append(f"type {item['name']} = {self.ty(inner['type'])}")
                    elif kind == "assoc_const":
                        assoc.append(f"const {item['name']}: {self.ty(inner['type'])}")
                traits.append(head + (f" {{ {'; '.join(sorted(assoc))} }}" if assoc else ""))
        for head in sorted(inherent):
            lines.append(head)
            lines.extend(sorted(set(inherent[head])))
        lines.extend(sorted(set(traits)))
        lines.extend(sorted(set(autos)))

    def fields(self, owner: str, ids: list, lines: list, visible=("public",)) -> None:
        for field_id in ids:
            field = self.index.get(str(field_id)) if field_id is not None else None
            if field and field["visibility"] in visible:
                self.context = f"{owner}::{field['name']}"
                lines.append(f"pub {owner}::{field['name']}: {self.ty(field['inner']['struct_field'])}")

    def tuple(self, ids: list) -> str:
        parts = []
        for field_id in ids:
            field = self.index.get(str(field_id)) if field_id is not None else None
            visible = field and field["visibility"] in ("public", "default")
            parts.append(("pub " if field["visibility"] == "public" else "") + self.ty(field["inner"]["struct_field"]) if visible else "_")
        return f"({', '.join(parts)})"

    def block(self, item_id: str, path: str) -> list[str]:
        item = self.index[item_id]
        kind, inner = one(item["inner"])
        self.context = path
        prefix = self.attributes(item) + "pub "
        lines: list[str] = []
        if kind == "module":
            lines.append(f"{prefix}mod {path}")
        elif kind in ("struct", "union"):
            generics = inner["generics"]
            head = f"{prefix}{kind} {path}{self.params(generics)}"
            shape, fields = one(inner["kind"]) if kind == "struct" else ("plain", {"fields": inner["fields"]})
            if shape == "unit":
                lines.append(head + self.where(generics) + ";")
            elif shape == "tuple":
                lines.append(head + self.tuple(fields) + self.where(generics))
            else:
                lines.append(head + self.where(generics))
                self.fields(path, fields["fields"], lines)
            self.implementations(path, inner["impls"], lines)
        elif kind == "enum":
            generics = inner["generics"]
            lines.append(f"{prefix}enum {path}{self.params(generics)}{self.where(generics)}")
            for variant_id in inner["variants"]:
                variant = self.index[str(variant_id)]
                vpath = f"{path}::{variant['name']}"
                self.context = vpath
                vkind, vinner = one(variant["inner"]["variant"]["kind"])
                discriminant = variant["inner"]["variant"]["discriminant"]
                text = f"{self.attributes(variant)}pub {vpath}"
                if vkind == "tuple":
                    text += self.tuple(vinner)
                elif vkind == "struct":
                    text += " { .. }"
                lines.append(text + (f" = {discriminant['expr']}" if discriminant else ""))
                if vkind == "struct":
                    self.fields(vpath, vinner["fields"], lines, ("public", "default"))
            self.implementations(path, inner["impls"], lines)
        elif kind == "trait":
            generics = inner["generics"]
            supertraits = f": {self.bounds(inner['bounds'])}" if inner["bounds"] else ""
            unsafe = "unsafe " if inner["is_unsafe"] else ""
            lines.append(f"{prefix}{unsafe}trait {path}{self.params(generics)}{supertraits}{self.where(generics)}")
            members = [self.member(path, str(member)) for member in inner["items"]]
            lines.extend(sorted(line for line in members if line))
            self.context = path
            foreign = [impl for impl in inner["implementations"] if str(impl) not in self.attached]
            lines.extend(sorted({self.impl(self.index[str(impl)]["inner"]["impl"]) for impl in foreign
                                 if str(impl) in self.index}))
        elif kind == "trait_alias":
            lines.append(f"{prefix}trait {path}{self.params(inner['generics'])} = {self.bounds(inner['params'])}")
        elif kind == "function":
            lines.append(prefix + self.function(path, inner))
        elif kind == "type_alias":
            generics = inner["generics"]
            lines.append(f"{prefix}type {path}{self.params(generics)} = {self.ty(inner['type'])}{self.where(generics)}")
        elif kind == "constant":
            lines.append(f"{prefix}const {path}: {self.ty(inner['type'])}")
        elif kind == "static":
            mutable = "mut " if inner["is_mutable"] else ""
            lines.append(f"{prefix}static {mutable}{path}: {self.ty(inner['type'])}")
        elif kind == "macro":
            lines.append(f"{prefix}macro {path}!")
        elif kind == "proc_macro":
            macro_kind = inner["kind"]
            if macro_kind == "bang":
                lines.append(f"{prefix}macro {path}!")
            elif macro_kind == "attr":
                lines.append(f"{prefix}attribute #[{path}]")
            else:
                helpers = f" (attributes: {', '.join(sorted(inner['helpers']))})" if inner["helpers"] else ""
                lines.append(f"{prefix}derive #[derive({path})]{helpers}")
        elif kind == "extern_crate":
            lines.append(f"pub use {path} = crate {inner['rename'] or inner['name']}")
        elif kind == "extern_type":
            lines.append(f"{prefix}extern type {path}")
        else:
            raise SystemExit(f"unsupported public item kind {kind} at {path}")
        return lines

    def render(self) -> list[str]:
        self.collect(self.root, self.index[self.root]["name"], frozenset({self.root}))
        for item_id, paths in self.found.items():
            # The shortest path outside a prelude, then the first in byte order.
            self.canonical[item_id] = min(
                paths, key=lambda path: ("prelude" in path.split("::"), path.count("::"), path))
        # Implementations listed under a type are not repeated under the trait.
        self.attached = set()
        for item_id in self.found:
            kind, inner = one(self.index[item_id]["inner"])
            if kind in ("struct", "enum", "union"):
                self.attached.update(str(impl) for impl in inner["impls"])
        blocks: list[tuple[str, int, list[str]]] = []
        for item_id, paths in self.found.items():
            canonical = self.canonical[item_id]
            kind = one(self.index[item_id]["inner"])[0]
            blocks.append((canonical, 0, self.block(item_id, canonical)))
            for alias in paths - {canonical}:
                blocks.append((alias, 1, [f"pub use {alias} = {kind} {canonical}"]))
        for path, use in self.external:
            summary = self.paths.get(str(use["id"])) if use["id"] is not None else None
            target = f"{summary['kind']} {'::'.join(summary['path'])}" if summary else use["source"]
            blocks.append((path, 1, [f"pub use {path} = {target}"]))
        lines: list[str] = []
        for _, _, block in sorted(blocks, key=lambda entry: (entry[0].split("::"), entry[1], entry[2])):
            lines.extend(block)
        return lines


def metadata() -> dict:
    output = subprocess.check_output(
        ["cargo", f"+{TOOLCHAIN}", "metadata", "--no-deps", "--locked", "--format-version", "1"], cwd=ROOT)
    return json.loads(output)


def packages(meta: dict, selected: list[str]) -> list[dict]:
    order = json.loads((ROOT / ".github" / "release-order.json").read_text())
    by_name = {package["name"]: package for package in meta["packages"]}
    names = selected or [name for name in order if name not in EXCLUDED]
    for name in names:
        if name not in order or name in EXCLUDED:
            raise SystemExit(f"{name} is not a listed library package of .github/release-order.json")
    return [by_name[name] for name in names]


def docs_rs_command(package: dict) -> list[str]:
    docs = (((package.get("metadata") or {}).get("docs") or {}).get("rs")) or {}
    command = ["cargo", f"+{TOOLCHAIN}", "rustdoc", "--locked", "--package", package["name"], "--lib"]
    if docs.get("all-features"):
        command.append("--all-features")
    elif docs.get("features"):
        command += ["--features", ",".join(docs["features"])]
    if docs.get("no-default-features"):
        command.append("--no-default-features")
    return command + ["--", *docs.get("rustdoc-args", []), "-Z", "unstable-options", "--output-format", "json"]


def listing(package: dict, target_directory: str, build: bool = True) -> str:
    command = docs_rs_command(package)
    if build:
        subprocess.run(command, cwd=ROOT, check=True)
    library = next(target for target in package["targets"]
                   if {"lib", "proc-macro"} & set(target["kind"]))
    krate = json.loads((Path(target_directory) / "doc" / f"{library['name'].replace('-', '_')}.json").read_text())
    lines = Listing(krate).render()
    header = [
        f"# Public API of {package['name']} (docs.rs features), generated by .github/public-api.py.",
        f"# {' '.join(command[1:])}",
    ]
    return "\n".join(header + lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true", help="compare with the committed listings")
    parser.add_argument("--no-build", action="store_true",
                        help="reuse the rustdoc JSON that the previous run left in the target directory")
    parser.add_argument("packages", nargs="*", help="packages from .github/release-order.json (default: all)")
    args = parser.parse_args()
    meta = metadata()
    target_directory = os.environ.get("CARGO_TARGET_DIR") or meta["target_directory"]
    stale = []
    for package in packages(meta, args.packages):
        text = listing(package, target_directory, not args.no_build)
        snapshot = SNAPSHOTS / f"{package['name']}.txt"
        if args.check:
            committed = snapshot.read_text() if snapshot.exists() else ""
            if committed != text:
                stale.append(package["name"])
                sys.stdout.writelines(difflib.unified_diff(
                    committed.splitlines(keepends=True), text.splitlines(keepends=True),
                    f"a/{snapshot.relative_to(ROOT)}", f"b/{snapshot.relative_to(ROOT)}"))
        else:
            SNAPSHOTS.mkdir(exist_ok=True)
            snapshot.write_text(text)
    if stale:
        sys.stdout.flush()
        print(f"\nThe public API of {', '.join(stale)} differs from .github/public-api/. If the change is "
              f"intended, run `python3 .github/public-api.py {' '.join(stale)}` and commit the listings.",
              file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
