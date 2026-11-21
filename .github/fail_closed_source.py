"""The non-test Rust source that `check-fail-closed.py` scans, with its items.

Reads every workspace member except the example applications under
`examples/`, plus the `*.rs.template` files `cargo-rullst` writes into
generated projects. Skipped: `tests/`, `benches/`, `examples/`, `fuzz/`,
`node_modules/` and `target/` directories, directories ending in `_tests`,
`tests.rs`, `*_tests.rs` and `*_test.rs`, files that are `#![cfg(test)]`, the
files of `#[cfg(test)] mod x;` declarations, and every other `#[cfg(test)]`
item (blanked in place). A `cfg(any(test, ...))` item is kept, because a
feature can compile it outside tests.

Each file becomes `code` (comments blanked, string contents kept, so code
generated from string literals stays visible) and a list of item spans
(`fn`, `impl`, `trait`, `const`, `static`) that name the enclosing item of an
offset, such as `TursoStore::connect`. Offsets and newlines are preserved.
"""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

SKIPPED_DIRECTORIES = {".git", "benches", "examples", "fuzz", "node_modules", "target", "tests"}

IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
LEXEME = re.compile(r"""//|/\*|(?<![A-Za-z0-9_])(?:br|cr|r)#*"|(?<![A-Za-z0-9_])[bc]?"|'""")
CHAR_LITERAL = re.compile(r"""'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'""")
ITEM_PREFIX = re.compile(
    r'(?:\bpub\s*(?:\([^()]*\))?|\bunsafe|\bdefault|\basync|\bconst|\bextern(?:\s*"[^"]*")?)\s*$'
)
SCOPE = re.compile(r"\b(fn|impl|trait|const|static)\b")
CONST_ITEM = re.compile(r"\s+(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:")
CFG_ATTRIBUTE = re.compile(r"#\s*(!?)\s*\[\s*cfg\s*\(")
NEXT_ATTRIBUTE = re.compile(r"\s*#\s*\[")
MODULE_DECLARATION = re.compile(r"\s*(?:pub\s*(?:\([^()]*\))?\s*)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;")
PATH_ATTRIBUTE = re.compile(r'#\s*\[\s*path\s*=\s*"([^"]+)"\s*\]')

Span = tuple[int, int, str]


def _blank(target: list[str], text: str, start: int, end: int) -> None:
    target[start:end] = ["\n" if char == "\n" else " " for char in text[start:end]]


def lex(text: str) -> tuple[str, str]:
    """`(code, skeleton)`: comments blanked; in `skeleton`, literal contents too."""
    code = list(text)
    skeleton = list(text)
    index = 0
    length = len(text)
    while match := LEXEME.search(text, index):
        start = match.start()
        token = match.group(0)
        if token == "//":
            end = text.find("\n", start)
            end = length if end == -1 else end
            _blank(code, text, start, end)
            _blank(skeleton, text, start, end)
        elif token == "/*":
            depth, end = 0, start
            while end < length:
                if text.startswith("/*", end):
                    depth, end = depth + 1, end + 2
                elif text.startswith("*/", end):
                    depth, end = depth - 1, end + 2
                    if depth == 0:
                        break
                else:
                    end += 1
            _blank(code, text, start, end)
            _blank(skeleton, text, start, end)
        elif token == "'":
            literal = CHAR_LITERAL.match(text, start)
            end = literal.end() if literal else start + 1  # else a lifetime or label
            if literal:
                _blank(skeleton, text, start + 1, end - 1)
        elif "r" in token[:2]:
            hashes = token.count("#")
            close = text.find('"' + "#" * hashes, match.end())
            close = length if close == -1 else close
            _blank(skeleton, text, match.end(), close)
            end = close + 1 + hashes
        else:
            end = match.end()
            while end < length and text[end] != '"':
                end += 2 if text[end] == "\\" else 1
            _blank(skeleton, text, match.end(), min(end, length))
            end += 1
        index = max(end, start + 1)
    return "".join(code), "".join(skeleton)


def matching_close(skeleton: str, opening: int) -> int:
    """The offset of the bracket that closes the one at `opening`."""
    close = {"{": "}", "[": "]", "(": ")"}[skeleton[opening]]
    depth = 0
    for index in range(opening, len(skeleton)):
        if skeleton[index] == skeleton[opening]:
            depth += 1
        elif skeleton[index] == close:
            depth -= 1
            if depth == 0:
                return index
    return len(skeleton) - 1


def item_body(skeleton: str, start: int) -> tuple[int, int] | None:
    """The `{...}` body of the item whose header starts at `start`, if any."""
    depth = 0
    for index in range(start, len(skeleton)):
        char = skeleton[index]
        if char in "([<":
            depth += 1
        elif char in ")]>":
            if not (char == ">" and skeleton[index - 1] == "-"):
                depth = max(depth - 1, 0)
        elif char == ";" and depth == 0:
            return None
        elif char == "{" and depth == 0:
            return index, matching_close(skeleton, index)
    return None


def statement_end(skeleton: str, start: int) -> int:
    depth = 0
    for index in range(start, len(skeleton)):
        char = skeleton[index]
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
            if depth < 0:
                return index
        elif char == ";" and depth == 0:
            return index
    return len(skeleton)


def at_item_position(skeleton: str, index: int) -> bool:
    """Whether the keyword at `index` starts an item, not a type (`-> impl Trait`)."""
    before = skeleton[max(0, index - 300) : index].rstrip()
    while prefix := ITEM_PREFIX.search(before):
        before = before[: prefix.start()].rstrip()
    return not before or before[-1] in "{};]"


def strip_generics(header: str) -> str:
    result = []
    depth = 0
    for index, char in enumerate(header):
        if char == "<":
            depth += 1
        elif char == ">" and not (index and header[index - 1] == "-"):
            depth = max(depth - 1, 0)
        elif depth == 0:
            result.append(char)
    return "".join(result)


def scopes(skeleton: str) -> list[Span]:
    """`(start, end, name)` of each `fn`, `impl`, `trait`, `const` and `static` item."""
    found = []
    for match in SCOPE.finditer(skeleton):
        keyword = match.group(1)
        if keyword in {"const", "static"}:
            item = CONST_ITEM.match(skeleton, match.end())
            if item and at_item_position(skeleton, match.start()):
                found.append((match.start(), statement_end(skeleton, match.end()), item.group(1)))
            continue
        if keyword in {"impl", "trait"} and not at_item_position(skeleton, match.start()):
            continue
        body = item_body(skeleton, match.end())
        if body is None:
            continue
        header = skeleton[match.end() : body[0]]
        if keyword in {"fn", "trait"}:
            name = IDENT.match(header.strip())
            if not name:
                continue
            label = name.group(0)
        else:
            header = strip_generics(re.sub(r"\bwhere\b.*", "", header, flags=re.S))
            self_type = re.split(r"\bfor\b", header)[-1]
            names = [name for name in IDENT.findall(self_type) if name not in {"dyn", "mut"}]
            if not names:
                continue
            label = names[-1]
        found.append((body[0], body[1], label))
    return found


def enclosing(spans: list[Span], offset: int) -> tuple[str, tuple[int, int] | None]:
    """The qualified name of the items around `offset` and the innermost span."""
    around = sorted((start, end, name) for start, end, name in spans if start <= offset <= end)
    if not around:
        return "", None
    return "::".join(name for _, _, name in around), around[-1][:2]


def test_only(predicate: str) -> bool:
    """Whether a `cfg(...)` predicate holds only under `cfg(test)`."""
    predicate = predicate.strip()
    if predicate == "test":
        return True
    nested = re.fullmatch(r"all\s*\((.*)\)", predicate, flags=re.S)
    if not nested:
        return False
    parts, depth, current = [], 0, ""
    for char in nested.group(1):
        if char == "," and depth == 0:
            parts.append(current)
            current = ""
            continue
        depth += (char == "(") - (char == ")")
        current += char
    return any(test_only(part) for part in [*parts, current])


def strip_test_items(code: str, skeleton: str) -> tuple[str, str, list[tuple[str | None, str]], bool]:
    """Blank every `#[cfg(test)]` item and return its `mod x;` declarations.

    The last value is whether the whole file is `#![cfg(test)]`.
    """
    code_chars = list(code)
    skeleton_chars = list(skeleton)
    modules: list[tuple[str | None, str]] = []
    for match in CFG_ATTRIBUTE.finditer(skeleton):
        opening = match.end() - 1
        close = matching_close(skeleton, opening)
        if not test_only(skeleton[opening + 1 : close]):
            continue
        if match.group(1):
            return code, skeleton, modules, True
        position = skeleton.find("]", close) + 1
        path_override = None
        while attribute := NEXT_ATTRIBUTE.match(skeleton, position):
            end = matching_close(skeleton, attribute.end() - 1)
            if path := PATH_ATTRIBUTE.search(code, attribute.start(), end + 1):
                path_override = path.group(1)
            position = end + 1
        if declaration := MODULE_DECLARATION.match(skeleton, position):
            modules.append((path_override, declaration.group(1)))
            end = declaration.end()
        elif body := item_body(skeleton, position):
            end = body[1] + 1
        else:
            end = skeleton.find(";", position) + 1 or len(skeleton)
        _blank(code_chars, code, match.start(), end)
        _blank(skeleton_chars, skeleton, match.start(), end)
    return "".join(code_chars), "".join(skeleton_chars), modules, False


def test_module_paths(path: Path, modules: list[tuple[str | None, str]]) -> set[Path]:
    """The files and directories of `#[cfg(test)] mod x;` declarations."""
    directory = path.parent if path.name in {"mod.rs", "lib.rs", "main.rs"} else path.parent / path.stem
    found = set()
    for override, name in modules:
        if override is not None:
            found.add((path.parent / override).resolve())
        else:
            found.update(
                candidate.resolve()
                for candidate in (directory / f"{name}.rs", directory / name / "mod.rs", directory / name)
            )
    return found


def member_directories(root: Path) -> list[Path]:
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    directories = []
    for member in manifest.get("workspace", {}).get("members", []):
        if member.split("/")[0] != "examples":
            directories.extend(sorted(path for path in root.glob(member) if path.is_dir()))
    return directories


def is_test_file(name: str) -> bool:
    return name == "tests.rs" or name.endswith(("_tests.rs", "_test.rs"))


def candidate_files(root: Path) -> list[Path]:
    files = []
    for directory in member_directories(root):
        stack = [directory]
        while stack:
            current = stack.pop()
            for entry in sorted(current.iterdir()):
                if entry.is_symlink():
                    continue
                if entry.is_dir():
                    if entry.name not in SKIPPED_DIRECTORIES and not entry.name.endswith("_tests"):
                        stack.append(entry)
                elif entry.name.endswith((".rs", ".rs.template")) and not is_test_file(entry.name):
                    files.append(entry)
    return sorted(files)


def production_sources(root: Path) -> list[tuple[str, str, list[Span]]]:
    """`(relative path, code, item spans)` of every scanned file."""
    parsed = {}
    excluded: set[Path] = set()
    for path in candidate_files(root):
        code, skeleton = lex(path.read_text(encoding="utf-8", errors="replace"))
        code, skeleton, modules, whole_file = strip_test_items(code, skeleton)
        if not whole_file:
            parsed[path] = (code, skeleton)
        excluded.update(test_module_paths(path, modules))
    sources = []
    for path, (code, skeleton) in parsed.items():
        resolved = path.resolve()
        if resolved not in excluded and not excluded.intersection(resolved.parents):
            sources.append((path.relative_to(root).as_posix(), code, scopes(skeleton)))
    return sources
