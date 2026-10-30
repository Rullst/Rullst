"""The first-hour blocks of the documentation: markers, commands and file edits.

A block a newcomer runs or applies is marked by an HTML comment on the line
just before its opening fence:

    <!-- first-hour: STEP [OPTION ...] -->
    ```bash
    cargo rullst new shop --default --blueprint blank --database sqlite
    ```

mdBook and GitHub do not render the comment, and .github/generate-llms-txt.py
drops it, so readers never see it. STEP names the block within its page (hooks
in first_hour_checks.py and the timing summary use it). Shell blocks (`bash`,
`sh`) run command by command; with `new-terminal` they start from the
journey's starting directory, as a reader opening a new terminal would. Every
other block takes exactly one action:

- `write=PATH [keep=REGEX]` replaces the file (a glob must match one file) with
  the block; `keep` keeps the file's own match of REGEX, such as the name of a
  generated migration the page says to keep.
- `insert=PATH after=TEXT` inserts the block after the first line containing
  TEXT, indented like it.
- `replace=PATH from=TEXT [through=TEXT]` replaces the lines from the first one
  containing `from` through the next one containing `through` (default: that
  line only), indented like the first.
- `output`: the output of the previous shell block contains the block.
- `expect-file=PATH`: every line of the block is a line of PATH.
- `apply`: a hook applies the block as the page's prose says.

Values with spaces or quotes use shell quoting (`after='get("/" => home),'`).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
import re
import shlex
import textwrap

ROOT = Path(__file__).resolve().parent.parent

# Reading order: each journey is one scratch directory; its pages share it.
JOURNEYS: dict[str, tuple[str, ...]] = {
    "readme": ("README.md",),
    "getting-started": ("docs/src/1-getting-started.md",),
    "hello-world": ("docs/src/tutorials/01-hello-world.md",),
    "rest-api": ("docs/src/tutorials/rest-api-quickstart.md",),
    "shop": (
        "docs/src/tutorials/02-cli-generators.md",
        "docs/src/tutorials/03-active-record-crud.md",
        "docs/src/tutorials/05-migrations-and-seeds.md",
        "docs/src/tutorials/11-authentication-system.md",
    ),
}

SHELL = frozenset({"bash", "sh"})
ACTIONS = {"write": {"keep"}, "insert": {"after"}, "replace": {"from", "through"},
           "output": set(), "expect-file": set(), "apply": set()}
REQUIRED = {"insert": {"after"}, "replace": {"from"}}
FLAGS = frozenset({"new-terminal", "output", "apply"})
EDITS = frozenset({"write", "insert", "replace"})
STEP = re.compile(r"[a-z0-9][a-z0-9-]*")
MARKER = re.compile(r"^(?P<indent>\s*)<!--\s*first-hour:\s*(?P<body>.*?)\s*-->\s*$")
FENCE = re.compile(r"^(?P<indent> {0,3})(?P<fence>`{3,}|~{3,})(?P<info>[^`]*)$")
HIDDEN = re.compile(r"^\s*#(?: |$)")
# Crates that are not on crates.io before a release; see release-order.json.
CLI = "cargo-rullst"


class MarkerError(ValueError):
    """A marker, its block or a documented command cannot run as written."""


@dataclass
class Block:
    page: str
    line: int
    step: str
    language: str
    code: str
    options: dict[str, str | bool] = field(default_factory=dict)

    @property
    def where(self) -> str:
        return f"{self.page}:{self.line} ({self.step})"

    @property
    def action(self) -> str | None:
        return next((name for name in ACTIONS if name in self.options), None)


def parse_options(body: str, where: str) -> tuple[str, dict[str, str | bool]]:
    try:
        tokens = shlex.split(body)
    except ValueError as error:
        raise MarkerError(f"{where}: unreadable marker: {error}") from error
    if not tokens or not STEP.fullmatch(tokens[0]):
        raise MarkerError(f"{where}: the marker must start with a step name like `create-project`")
    options: dict[str, str | bool] = {}
    known = FLAGS | set(ACTIONS) | {name for names in ACTIONS.values() for name in names}
    for token in tokens[1:]:
        key, separator, value = token.partition("=")
        if key not in known or (key in FLAGS) == bool(separator) or (separator and not value):
            raise MarkerError(f"{where}: unknown or malformed marker option `{token}`")
        if key in options:
            raise MarkerError(f"{where}: option `{key}` repeated")
        options[key] = value if separator else True
    return tokens[0], options


def check_options(block: Block) -> None:
    options = set(block.options)
    if block.language in SHELL:
        if options - {"new-terminal"}:
            raise MarkerError(f"{block.where}: a shell block takes only `new-terminal`")
        return
    actions = [name for name in ACTIONS if name in options]
    if len(actions) != 1:
        raise MarkerError(f"{block.where}: a `{block.language}` block needs exactly one of "
                          + ", ".join(ACTIONS))
    action = actions[0]
    extra = options - {action} - ACTIONS[action]
    missing = REQUIRED.get(action, set()) - options
    if extra or missing:
        raise MarkerError(f"{block.where}: `{action}` does not take {sorted(extra)} "
                          f"or misses {sorted(missing)}")
    if action in EDITS and any(HIDDEN.match(line) for line in block.code.splitlines()):
        raise MarkerError(f"{block.where}: a block applied to a file must not hide lines with `# `")


def parse_page(page: str, text: str) -> list[Block]:
    """The marked blocks of one page, in reading order."""
    lines = text.splitlines()
    blocks: list[Block] = []
    fence: str | None = None
    index = 0
    while index < len(lines):
        line = lines[index]
        if fence is not None:
            if line.strip().startswith(fence) and set(line.strip()) == {fence[0]}:
                fence = None
            index += 1
            continue
        opening = FENCE.match(line)
        if opening:
            fence = opening["fence"]
            index += 1
            continue
        marker = MARKER.match(line)
        if not marker:
            if "<!--" in line and "first-hour" in line:
                raise MarkerError(f"{page}:{index + 1}: malformed first-hour marker")
            index += 1
            continue
        where = f"{page}:{index + 1}"
        step, options = parse_options(marker["body"], where)
        opening = FENCE.match(lines[index + 1]) if index + 1 < len(lines) else None
        if not opening:
            raise MarkerError(f"{where}: a first-hour marker must be followed by a fenced block")
        language = opening["info"].strip().split(",")[0].split(" ")[0]
        closing, indent = opening["fence"], len(opening["indent"])
        body: list[str] = []
        cursor = index + 2
        while cursor < len(lines):
            current = lines[cursor]
            if current.strip().startswith(closing) and set(current.strip()) == {closing[0]}:
                break
            body.append(current[indent:] if current[:indent].strip() == "" else current)
            cursor += 1
        else:
            raise MarkerError(f"{where}: the marked block is never closed")
        block = Block(page, index + 2, step, language, "\n".join(body) + "\n", options)
        check_options(block)
        if any(other.step == step for other in blocks):
            raise MarkerError(f"{where}: step `{step}` is already used on this page")
        blocks.append(block)
        index = cursor + 1
    return blocks


def commands(code: str) -> list[list[str]]:
    """Shell commands of a block: `\\` continuations joined, `#` comments dropped."""
    found, pending = [], ""
    for line in code.splitlines():
        if line.rstrip().endswith("\\"):
            pending += line.rstrip()[:-1] + " "
            continue
        tokens = shlex.split(pending + line, comments=True)
        pending = ""
        if tokens:
            found.append(tokens)
    if pending.strip():
        raise MarkerError("a command ends with a line continuation")
    return found


def release_crates() -> list[str]:
    import json
    return json.loads((ROOT / ".github/release-order.json").read_text())


def check_command(tokens: list[str], version: str, crates: list[str]) -> None:
    """The documented release crates must name this candidate's exact version.

    Cargo never selects a prerelease by itself, so `cargo install cargo-rullst`
    or `cargo add rullst` without the exact version installs the last 12.x.
    """
    if tokens[:2] == ["cargo", "install"]:
        named = [token for token in tokens[2:] if token in crates]
        if named and named != [CLI]:
            raise MarkerError(f"`{shlex.join(tokens)}`: only {CLI} is installed from the archives")
        if named:
            requested = next((tokens[i + 1] for i, token in enumerate(tokens[:-1])
                              if token == "--version"), None)
            if requested not in (f"={version}", version):
                raise MarkerError(f"`{shlex.join(tokens)}` does not install {CLI} {version}: "
                                  f"name it with `--version ={version}`")
    if tokens[:2] == ["cargo", "add"]:
        for token in tokens[2:]:
            crate, _, requirement = token.partition("@")
            if crate in crates and requirement != f"={version}":
                raise MarkerError(f"`{shlex.join(tokens)}` does not add {crate} {version}: "
                                  f"write `{crate}@={version}`")


def load(root: Path = ROOT) -> dict[str, list[Block]]:
    """Every journey page's blocks; fails on any stray or broken marker."""
    pages = {page: parse_page(page, (root / page).read_text(encoding="utf-8"))
             for journey in JOURNEYS.values() for page in journey}
    for page, blocks in pages.items():
        if not blocks:
            raise MarkerError(f"{page} is a first-hour page without marked blocks")
    sources = [root / "README.md", *sorted((root / "docs/src").rglob("*.md"))]
    for source in sources:
        page = source.relative_to(root).as_posix()
        if page not in pages and "first-hour:" in source.read_text(encoding="utf-8"):
            raise MarkerError(f"{page} has first-hour markers but is in no journey")
    return pages


def reindent(code: str, like: str) -> list[str]:
    indent = like[: len(like) - len(like.lstrip())]
    return [indent + line if line.strip() else "" for line in textwrap.dedent(code).splitlines()]


def _anchor(lines: list[str], text: str, start: int, what: str) -> int:
    for number in range(start, len(lines)):
        if text in lines[number]:
            return number
    raise MarkerError(f"no line containing `{text}` {what}")


def edit(block: Block, old: str | None) -> str:
    """The file content after applying an edit block to `old` (None: no file)."""
    action = block.action
    if action == "write":
        new = textwrap.dedent(block.code)
        keep = block.options.get("keep")
        if isinstance(keep, str):
            mine = re.search(keep, old or "")
            if not mine or not re.search(keep, new):
                raise MarkerError(f"{block.where}: `keep={keep}` matches nothing")
            new = re.sub(keep, lambda _: mine[0], new)
        return new
    if old is None:
        raise MarkerError(f"{block.where}: there is no file to edit")
    lines = old.splitlines()
    if action == "insert":
        at = _anchor(lines, str(block.options["after"]), 0, "to insert after")
        lines[at + 1:at + 1] = reindent(block.code, lines[at])
    elif action == "replace":
        start = _anchor(lines, str(block.options["from"]), 0, "to replace")
        through = block.options.get("through")
        end = _anchor(lines, through, start, "ends the replacement") if isinstance(through, str) else start
        lines[start:end + 1] = reindent(block.code, lines[start])
    else:
        raise MarkerError(f"{block.where}: `{action}` is not a file edit")
    return "\n".join(lines) + "\n"


def contains_text(haystack: str, needle: str) -> bool:
    """Whitespace-insensitive containment, for documented output."""
    return " ".join(needle.split()) in " ".join(haystack.split())
