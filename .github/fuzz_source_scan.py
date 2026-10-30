"""Token-level facts about Rust sources that can read files while compiling.

This is not a Rust parser. It lexes comments, strings and delimiters exactly
enough to find `include*!`, `#[path]`, macro definitions and calls, attributes,
`use` imports, path-like string literals and filesystem calls, plus the path
and built-in helpers the build graph applies to them. Callers fail closed on
anything it reports as dynamic or cannot lex.
"""

from __future__ import annotations

import posixpath
import re
import subprocess

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
_OPENERS = {("p", "("), ("p", "["), ("p", "{")}
_PLACEHOLDER = re.compile(r"[{}$%]")
_COMPARISONS = [[("p", "="), ("p", "=")], [("p", "!"), ("p", "=")]]

INCLUDES = {"include_str": "data", "include_bytes": "data", "include": "code"}
QUOTES = frozenset({"quote", "quote_spanned", "parse_quote", "parse_quote_spanned", "format_ident"})
# Built-in macros and attributes: formatting, assertions, includes (resolved
# separately) and quote! (generated code). Their arguments never name files.
STD_MACROS = frozenset({
    "format", "format_args", "print", "println", "eprint", "eprintln", "write", "writeln",
    "panic", "assert", "assert_eq", "assert_ne", "debug_assert", "debug_assert_eq",
    "debug_assert_ne", "unreachable", "unimplemented", "todo", "vec", "matches", "concat",
    "stringify", "env", "option_env", "compile_error", "dbg", "line", "column", "file",
    "module_path", "cfg", "thread_local", *INCLUDES, *QUOTES,
})
STD_ATTRIBUTES = frozenset({
    "doc", "cfg", "cfg_attr", "path", "allow", "warn", "deny", "forbid", "expect",
    "deprecated", "must_use", "should_panic", "ignore", "test", "inline", "cold", "derive",
    "repr", "non_exhaustive", "macro_export", "macro_use", "link", "link_name",
    "export_name", "link_section", "no_mangle", "track_caller", "automatically_derived",
    "diagnostic", "rustfmt", "clippy",
})
# Registry macros/attributes known to read files while compiling. Their mere
# presence makes a scope fail closed; the list is not exhaustive (see the
# path-like argument rule in fuzz_build_graph.py).
FILE_MACROS = frozenset({
    "migrate", "query", "query_as", "query_scalar", "query_unchecked", "query_as_unchecked",
    "query_scalar_unchecked", "query_file", "query_file_as", "query_file_scalar",
    "query_file_unchecked", "query_file_as_unchecked", "query_file_scalar_unchecked",
    "include_dir", "embed_migrations", "include_proto", "include_file_descriptor_set",
    "dotenv", "lazy_static_include_str", "lazy_static_include_bytes",
    "include_json", "include_toml", "include_yaml", "import_types",
})
FILE_ATTRIBUTES = frozenset({"folder", "grammar", "template", "include_dir"})
# Compile-time filesystem, process or environment access (proc macros).
_IO_ANYWHERE = frozenset({
    "fs", "process", "File", "OpenOptions", "DirBuilder", "Command", "read_dir",
    "read_to_string", "read_link", "canonicalize", "current_dir", "WalkDir", "walkdir",
    "glob", "tracked_path", "tracked_env",
})
_IO_CALLS = frozenset({"exists", "is_file", "is_dir", "is_symlink", "metadata", "try_exists",
                       "symlink_metadata"})
_ENV_PLACEHOLDER = re.compile(r"\$\{?[A-Za-z_]|%[A-Za-z_][A-Za-z0-9_]*%")


def climbs(value: str | None) -> bool:
    return value is None or ".." in value.split("/")


def path_like(value: str | None) -> bool:
    """A climbing, absolute or environment-relative (`$VAR`, `%VAR%`) string."""
    return climbs(value) or value.startswith("/") or _ENV_PLACEHOLDER.match(value) is not None


def under(path: str, directory: str) -> bool:
    return directory == "" or path == directory or path.startswith(directory + "/")


def join(base: str, relative: object) -> str | None:
    """Normalized repository path, or None for absolute, odd or escaping paths.

    A `..` after a placeholder component (`{dir}/..`, `$VAR/..`, `%X%/..`)
    is not cancelled: the placeholder's expansion is unknown.
    """
    if (not isinstance(relative, str) or relative.startswith("/") or "\\" in relative
            or "\0" in relative):
        return None
    parts = relative.split("/")
    if any(part == ".." and any(_PLACEHOLDER.search(item) for item in parts[:index])
           for index, part in enumerate(parts)):
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


def resolve_path(written: str, imports: dict) -> str:
    """Full path of a written macro/attribute path, through this file's `use`s."""
    head, separator, rest = written.partition("::")
    if head in imports:
        return imports[head] + separator + rest
    return written


def builtin(kind: str, full: str) -> bool:
    """Built-ins count only unqualified or as `std::`/`core::`/`alloc::` (quote: `quote::`/`syn::`)."""
    prefix, _, name = full.rpartition("::")
    root = prefix.split("::", 1)[0]
    if kind == "attr":
        return not prefix and name in STD_ATTRIBUTES or root in {"rustfmt", "clippy", "diagnostic"}
    if name in QUOTES:
        return not prefix or root in {"quote", "syn"}
    return name in STD_MACROS and (not prefix or root in {"std", "core", "alloc"})


def cat_file(root, oids) -> dict[str, bytes]:
    """Blob contents by object ID, read in one `git cat-file --batch`."""
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
    """Return `(kind, value)` pairs: `i` ident (without `r#`), `s` string, `p` punct, `o` other."""
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
                result.append(("i", value.removeprefix("r#")))
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

    __slots__ = ("error", "defs", "defines_macros", "quotes", "calls", "attrs", "aliases",
                 "renames", "imports", "refs", "literals", "exports", "compile_io", "runtime_io", "mods")

    def __init__(self) -> None:
        self.error: str | None = None
        self.defs: list[tuple[str, int, int]] = []        # macro_rules name, body range
        self.defines_macros = False                       # `macro_rules! $name`
        self.quotes: list[tuple[int, int]] = []           # quote!-family bodies
        # name, open, close, string literals, has `$`, nested macro names, written path
        self.calls: list[tuple[str, int, int, tuple, bool, tuple, str]] = []
        # Written attribute path (`sqlx::test`), range, has `$`; each item of
        # a `cfg_attr` is an attribute of its own.
        self.attrs: list[tuple[str, int, int, bool]] = []
        self.aliases: set[str] = set()                    # identifiers renamed with `as`
        self.renames: list[tuple[str, str]] = []
        self.imports: dict[str, str] = {}                 # `use` name -> full path
        self.refs: list[tuple[str, str, str | None, int]] = []  # kind, form, value, position
        # Path-like literals: every enclosing (kind, written path) call or
        # attribute from the outermost, value, position, `==`/`!=` operand.
        self.literals: list[tuple[tuple[tuple[str, str], ...], str | None, int, bool]] = []
        self.exports: set[str] = set()                    # proc-macro and derive-helper names
        self.compile_io: list[str] = []
        self.runtime_io = False
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
            # Build-script output; a part that climbs out of it is not resolvable.
            if any(kind == "s" and climbs(value) for kind, value in args):
                return "dynamic", None
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


def _attribute(toks, close, start, end, facts) -> None:
    if start + 1 < end and toks[start] == ("i", "path") and toks[start + 1] == ("p", "="):
        form, value = _value(toks, start + 2)
        facts.refs.append(("mod", form, value, start))
    elif start + 1 < end and toks[start] == ("i", "cfg_attr") and toks[start + 1] == ("p", "("):
        for item_start, item_end in _split(toks, close, start + 2, close[start + 1])[1:]:
            _attribute(toks, close, item_start, item_end, facts)


def _attribute_name(toks: list, start: int, end: int) -> str:
    """An attribute's written path (`crate::server_function`)."""
    parts, index = [], start
    while index < end and toks[index][0] == "i":
        parts.append(toks[index][1])
        if toks[index + 1:index + 3] != [("p", ":"), ("p", ":")]:
            break
        index += 3
    return "::".join(parts)


def _call_path(toks: list, index: int) -> str:
    """A macro call's written path (`sqlx::query`, `$crate::x`, `::core::env`)."""
    parts = [toks[index][1]]
    while index >= 3 and toks[index - 2:index] == [("p", ":"), ("p", ":")] and toks[index - 3][0] == "i":
        index -= 3
        parts.insert(0, ("$" if index and toks[index - 1] == ("p", "$") else "") + toks[index][1])
    return "::".join(parts)


def _use_tree(toks, close, start, end, prefix, imports) -> None:
    path, index = list(prefix), start
    while index < end:
        token = toks[index]
        if token == ("p", "{") and index in close:
            for item_start, item_end in _split(toks, close, index + 1, close[index]):
                _use_tree(toks, close, item_start, item_end, path, imports)
            return
        if token == ("i", "as") and index + 1 < end:
            if path:
                imports[toks[index + 1][1]] = "::".join(path)
            return
        if token[0] == "i" and token[1] != "self":
            path.append(token[1])
        elif token == ("p", "*"):
            return
        index += 1
    if path:
        imports[path[-1]] = "::".join(path)


def _attributes(toks, close, start, end, facts, owners) -> None:
    """Record one attribute; each `cfg_attr` item is an attribute of its own."""
    name = _attribute_name(toks, start, end)
    facts.attrs.append((name, start - 1, end, ("p", "$") in toks[start:end]))
    owners[start - 1] = (end, ("attr", name))
    if name == "cfg_attr" and start + 1 < end and toks[start + 1] == ("p", "(") and start + 1 in close:
        for item_start, item_end in _split(toks, close, start + 2, close[start + 1])[1:]:
            if item_start < item_end:
                _attributes(toks, close, item_start, item_end, facts, owners)


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

    owners: dict[int, tuple[int, tuple[str, str]]] = {}
    io_hits = []
    for index, (kind, value) in enumerate(toks):
        if (kind, value) == ("i", "use"):
            end = index + 1
            while end < size and toks[end] != ("p", ";"):
                end = close.get(end, end) + 1
            _use_tree(toks, close, index + 1, min(end, size), [], facts.imports)
        if kind == "i" and at(index + 1) == ("i", "as"):
            facts.aliases.add(value)
            if at(index + 2)[0] == "i":
                facts.renames.append((value, at(index + 2)[1]))
        if kind == "i" and value == "macro_rules" and at(index + 1) == ("p", "!"):
            if at(index + 2) == ("p", "$"):
                facts.defines_macros = True
            elif at(index + 2)[0] == "i" and index + 3 in close:
                facts.defs.append((at(index + 2)[1], index + 3, close[index + 3]))
        elif kind == "i" and at(index + 1) == ("p", "!") and index + 2 in close:
            start, end = index + 2, close[index + 2]
            inner = toks[start + 1:end]
            nested = tuple(inner[j][1] for j in range(len(inner) - 2)
                           if inner[j][0] == "i" and inner[j + 1] == ("p", "!") and inner[j + 2] in _OPENERS)
            path = _call_path(toks, index)
            facts.calls.append((value, start, end, tuple(v for k, v in inner if k == "s"),
                                ("p", "$") in inner, nested, path))
            owners[start] = (end, ("call", path))
            if value in QUOTES:
                facts.quotes.append((start, end))
            if value in INCLUDES:
                form, path = _include_form(inner)
                facts.refs.append((INCLUDES[value], form, path, index))
        elif (kind, value) == ("p", "#"):
            bracket = index + 2 if at(index + 1) == ("p", "!") else index + 1
            if at(bracket) == ("p", "[") and bracket in close:
                end = close[bracket]
                name = _attribute_name(toks, bracket + 1, end)
                _attributes(toks, close, bracket + 1, end, facts, owners)
                _attribute(toks, close, bracket + 1, end, facts)
                if name in {"proc_macro", "proc_macro_attribute"}:
                    following = toks[end + 1:end + 16]
                    for offset, token in enumerate(following[:-1]):
                        if token == ("i", "fn") and following[offset + 1][0] == "i":
                            facts.exports.add(following[offset + 1][1])
                            break
                elif name == "proc_macro_derive":
                    content = toks[bracket + 1:end]
                    for offset, token in enumerate(content):
                        if token == ("i", "attributes"):
                            facts.exports.update(v for k, v in content[offset + 1:] if k == "i")
        if kind == "i" and value == "mod" and at(index + 1)[0] == "i" and at(index + 2) == ("p", ";"):
            facts.mods = True
        # Filesystem/process/environment capability (outside generated code).
        if kind == "i" and (value in _IO_ANYWHERE
                            or value == "env" and at(index + 1) in {("p", ":"), ("i", "as")}
                            or value in _IO_CALLS and at(index + 1) == ("p", "(")):
            io_hits.append((index, value))
        if (kind, value) == ("s", "CARGO_MANIFEST_DIR"):
            io_hits.append((index, value))
        if kind == "i" and at(index + 1) == ("p", ":") and at(index + 2) == ("p", ":") and \
                value in {"fs", "env", "process", "File", "Command", "OpenOptions"}:
            facts.runtime_io = True
    # Every call or attribute enclosing each path-like literal.
    stack: list[tuple[int, tuple[str, str]]] = []
    for index, (kind, value) in enumerate(toks):
        while stack and stack[-1][0] <= index:
            stack.pop()
        if kind == "s" and path_like(value):
            compared = (toks[max(index - 2, 0):index] in _COMPARISONS
                        or toks[index + 1:index + 3] in _COMPARISONS)
            facts.literals.append((tuple(owner for _, owner in stack), value, index, compared))
        if index in owners:
            stack.append(owners[index])
    # Tokens inside quote! are code generated for the caller's program. Only
    # includes or `#[path]` generated there with computed paths are
    # compile-time I/O of the proc macro emitting them.
    facts.compile_io = [value for index, value in io_hits if not facts.inside(index, facts.quotes)]
    for kind, form, _, position in facts.refs:
        if form in {"meta", "dynamic"} and facts.inside(position, facts.quotes):
            facts.compile_io.append(f"generated {kind} path")
    return facts
