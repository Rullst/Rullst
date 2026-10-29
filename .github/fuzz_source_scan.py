"""Token-level facts about Rust sources that can read files while compiling.

This is not a Rust parser. It lexes comments, strings and delimiters exactly
enough to find `include*!`, `#[path]`, path-like macro arguments, macro
definitions/invocations and filesystem calls. Callers fail closed on anything
it reports as dynamic or cannot lex.
"""

from __future__ import annotations

import re

_TOKEN = re.compile(r"""
    (?P<space>\s+)
  | (?P<comment>//[^\n]*)
  | (?P<block>/\*)
  | (?P<raw>[bc]?r(?P<hashes>\#*)")
  | (?P<str>[bc]?"[^"\\]*(?:\\.[^"\\]*)*")
  | (?P<char>b?'(?:[^'\\\n]|\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]{1,8}\}|[^\n]))')
  | (?P<life>'[A-Za-z_][A-Za-z0-9_]*)
  | (?P<ident>r\#[A-Za-z_][A-Za-z0-9_]*|[A-Za-z_][A-Za-z0-9_]*)
  | (?P<num>[0-9][A-Za-z0-9_]*)
  | (?P<punct>.)
""", re.X | re.S)
_BLOCK = re.compile(r"/\*|\*/")
_ESCAPE = re.compile(r"\\(?:\n\s*|x([0-9a-fA-F]{2})|u\{([0-9a-fA-F_]{1,8})\}|(.))", re.S)
_SIMPLE = {"n": "\n", "t": "\t", "r": "\r", "0": "\0", "\\": "\\", '"': '"', "'": "'"}
_OPEN = {")": "(", "]": "[", "}": "{"}

INCLUDES = {"include_str": "data", "include_bytes": "data", "include": "code"}
QUOTES = frozenset({"quote", "quote_spanned", "parse_quote", "parse_quote_spanned"})
# Keyword arguments of attributes/macros that third-party or workspace macros
# commonly resolve against the source file or the compiling package.
PATH_KEYS = frozenset({"path", "dir", "folder", "file", "directory"})
# Compile-time filesystem, process or environment access (proc macros).
_IO_ANYWHERE = frozenset({
    "fs", "process", "File", "OpenOptions", "DirBuilder", "Command", "read_dir",
    "read_to_string", "read_link", "canonicalize", "current_dir", "WalkDir", "walkdir",
    "glob", "tracked_path", "tracked_env",
})
# Build-script operations whose target cannot be resolved from literals.
_BUILD_DYNAMIC = frozenset({
    "parent", "ancestors", "canonicalize", "current_dir", "home_dir", "set_current_dir",
    "current_exe", "Command", "vars", "vars_os", "read_link", "glob", "WalkDir", "walkdir",
})
_BUILD_ENV = frozenset({"OUT_DIR", "TARGET", "HOST", "PROFILE", "OPT_LEVEL", "DEBUG",
                        "NUM_JOBS", "RUSTC", "RUSTDOC", "RUSTC_LINKER"})


def _unescape(body: str) -> str | None:
    def replace(match: re.Match) -> str:
        if match[1]:
            return chr(int(match[1], 16))
        if match[2]:
            return chr(int(match[2].replace("_", ""), 16))
        if match[3] is not None:
            return _SIMPLE.get(match[3], "\\" + match[3])
        return ""  # line continuation
    try:
        return _ESCAPE.sub(replace, body)
    except (ValueError, OverflowError):
        return None


def tokens(text: str) -> list[tuple[str, str | None]]:
    """Return `(kind, value)` pairs: `i` ident, `s` string, `p` punct, `o` other."""
    result: list[tuple[str, str | None]] = []
    position, size = 0, len(text)
    while position < size:
        match = _TOKEN.match(text, position)
        kind = match.lastgroup
        if kind in {"space", "comment"}:
            position = match.end()
        elif kind == "block":
            depth, position = 1, match.end()
            while depth:
                found = _BLOCK.search(text, position)
                if found is None:
                    raise ValueError("unterminated block comment")
                depth += 1 if found.group() == "/*" else -1
                position = found.end()
        elif kind == "raw":
            end = text.find('"' + match["hashes"], match.end())
            if end < 0:
                raise ValueError("unterminated raw string")
            result.append(("s", text[match.end():end]))
            position = end + 1 + len(match["hashes"])
        else:
            value = match.group()
            if kind == "str":
                result.append(("s", _unescape(value[value.index('"') + 1:-1])))
            elif kind == "ident":
                result.append(("i", value))
            elif kind == "punct":
                result.append(("p", value))
            else:
                result.append(("o", None))
            position = match.end()
    return result


def _pairs(toks: list) -> dict[int, int]:
    stack, close = [], {}
    for index, (kind, value) in enumerate(toks):
        if kind != "p":
            continue
        if value in "([{":
            stack.append(index)
        elif value in ")]}":
            if not stack or toks[stack[-1]][1] != _OPEN[value]:
                raise ValueError("unbalanced delimiters")
            close[stack.pop()] = index
    if stack:
        raise ValueError("unbalanced delimiters")
    return close


class Facts:
    """Content-derived facts; positions are token indexes of one blob."""

    __slots__ = ("error", "defs", "quotes", "calls", "aliases", "refs", "compile_io",
                 "runtime_io", "build_paths", "build_dynamic", "mods")

    def __init__(self) -> None:
        self.error: str | None = None
        self.defs: list[tuple[str, int, int]] = []        # macro_rules name, body range
        self.quotes: list[tuple[int, int]] = []           # quote!-family bodies
        self.calls: list[tuple[str, int, int, tuple, bool]] = []  # name, open, close, strings, has `$`
        self.aliases: set[str] = set()                    # identifiers renamed with `as`
        self.refs: list[tuple[str, str, str | None, int]] = []  # kind, form, value, position
        self.compile_io: list[str] = []
        self.runtime_io = False
        self.build_paths: list[str | None] = []
        self.build_dynamic: list[str] = []
        self.mods = False

    def inside(self, position: int, ranges) -> bool:
        return any(start < position < end for *_, start, end in ranges)

    def macro_at(self, position: int) -> str | None:
        """Innermost `macro_rules!` body containing a position."""
        best = None
        for name, start, end in self.defs:
            if start < position < end and (best is None or start > best[1]):
                best = (name, start)
        return best[0] if best else None


def _include_form(args: list) -> tuple[str, str | None]:
    if args and args[-1] == ("p", ","):
        args = args[:-1]
    if len(args) == 1 and args[0][0] == "s":
        return "lit", args[0][1]
    if len(args) == 2 and args[0] == ("p", "$") and args[1][0] == "i":
        return "meta", args[1][1]
    head = [("i", "concat"), ("p", "!"), ("p", "("), ("i", "env"), ("p", "!"), ("p", "("),
            ("s", "CARGO_MANIFEST_DIR"), ("p", ")")]
    if args[:8] == head and args[-1:] == [("p", ")")]:
        rest = args[8:-1]
        if rest and rest[-1] == ("p", ","):
            rest = rest[:-1]
        parts = rest[1::2]
        if (rest and all(token == ("p", ",") for token in rest[0::2])
                and all(kind == "s" and value is not None for kind, value in parts)):
            return "manifest", "".join(value for _, value in parts)
    for index in range(len(args) - 3):
        if args[index:index + 4] == [("i", "env"), ("p", "!"), ("p", "("), ("s", "OUT_DIR")]:
            return "out", None
    return "dynamic", None


def _value(toks: list, index: int) -> tuple[str, str | None]:
    token = toks[index] if index < len(toks) else ("o", None)
    if token[0] == "s":
        return "lit", token[1]
    if token == ("p", "$") and index + 1 < len(toks) and toks[index + 1][0] == "i":
        return "meta", toks[index + 1][1]
    return "dynamic", None


def _split(toks: list, close: dict, start: int, end: int) -> list[tuple[int, int]]:
    items, index, begin = [], start, start
    while index < end:
        if index in close:
            index = close[index] + 1
            continue
        if toks[index] == ("p", ","):
            items.append((begin, index))
            begin = index + 1
        index += 1
    items.append((begin, end))
    return items


def _attribute(toks, close, start, end, facts, std_paths) -> None:
    if start + 1 < end and toks[start] == ("i", "path") and toks[start + 1] == ("p", "="):
        form, value = _value(toks, start + 2)
        facts.refs.append(("mod", form, value, start))
        std_paths.add(start)
    elif start + 1 < end and toks[start] == ("i", "cfg_attr") and toks[start + 1] == ("p", "("):
        for item_start, item_end in _split(toks, close, start + 2, close[start + 1])[1:]:
            _attribute(toks, close, item_start, item_end, facts, std_paths)


def source_facts(data: bytes) -> Facts:
    facts = Facts()
    try:
        toks = tokens(data.decode("utf-8", "replace").removeprefix("\ufeff"))
        close = _pairs(toks)
    except ValueError as error:
        facts.error = str(error)
        return facts
    size = len(toks)

    def at(index: int) -> tuple:
        return toks[index] if 0 <= index < size else ("o", None)

    groups, std_paths, io_hits = [], set(), []
    for index, (kind, value) in enumerate(toks):
        if kind == "i" and at(index + 1) == ("i", "as"):
            facts.aliases.add(value)
        if kind == "i" and value == "macro_rules" and at(index + 1) == ("p", "!") \
                and at(index + 2)[0] == "i" and index + 3 in close:
            facts.defs.append((at(index + 2)[1], index + 3, close[index + 3]))
        elif kind == "i" and at(index + 1) == ("p", "!") and index + 2 in close:
            start, end = index + 2, close[index + 2]
            inner = toks[start + 1:end]
            facts.calls.append((value, start, end, tuple(v for k, v in inner if k == "s"),
                                ("p", "$") in inner))
            groups.append((start, end))
            if value in QUOTES:
                facts.quotes.append((start, end))
            if value in INCLUDES:
                form, path = _include_form(inner)
                facts.refs.append((INCLUDES[value], form, path, index))
        elif (kind, value) == ("p", "#"):
            bracket = index + 2 if at(index + 1) == ("p", "!") else index + 1
            if at(bracket) == ("p", "[") and bracket in close:
                groups.append((bracket, close[bracket]))
                _attribute(toks, close, bracket + 1, close[bracket], facts, std_paths)
        if kind == "i" and value == "mod" and at(index + 1)[0] == "i" and at(index + 2) == ("p", ";"):
            facts.mods = True
        # Filesystem/process/environment capability (outside generated code).
        if kind == "i" and (value in _IO_ANYWHERE
                            or value == "env" and at(index + 1) in {("p", ":"), ("i", "as")}):
            io_hits.append((index, value))
        if (kind, value) == ("s", "CARGO_MANIFEST_DIR"):
            io_hits.append((index, value))
        if kind == "i" and at(index + 1) == ("p", ":") and at(index + 2) == ("p", ":") and \
                value in {"fs", "env", "process", "File", "Command", "OpenOptions"}:
            facts.runtime_io = True
        if kind == "s" and (value is None or "\\" in value or value.startswith("/")
                            or ".." in value.split("/")):
            facts.build_paths.append(value)
        if kind == "i" and value in _BUILD_DYNAMIC:
            facts.build_dynamic.append(value)
        if kind == "i" and value in {"var", "var_os"} and at(index + 1) == ("p", "("):
            name = at(index + 2)
            if name[0] != "s" or name[1] is None or not (
                    name[1] in _BUILD_ENV or name[1].startswith(("CARGO_", "DEP_"))):
                facts.build_dynamic.append(f"environment variable {name[1] or '<computed>'}")
    seen = set()
    for start, end in groups:
        for index in range(start + 1, end - 1):
            if (index in seen or index in std_paths or toks[index][0] != "i"
                    or toks[index][1] not in PATH_KEYS or toks[index + 1] != ("p", "=")
                    or toks[index - 1] not in {("p", "("), ("p", ","), ("p", "["), ("p", "{")}):
                continue
            seen.add(index)
            form, value = _value(toks, index + 2)
            if form != "dynamic":
                facts.refs.append(("key", form, value, index))
    # Tokens inside quote! are code generated for the caller's program. Only
    # includes or `#[path]` generated there with computed paths are
    # compile-time I/O of the proc macro emitting them.
    facts.compile_io = [value for index, value in io_hits if not facts.inside(index, facts.quotes)]
    for kind, form, _, position in facts.refs:
        if kind != "key" and form in {"meta", "dynamic"} and facts.inside(position, facts.quotes):
            facts.compile_io.append(f"generated {kind} path")
    return facts
