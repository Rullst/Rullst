#!/usr/bin/env python3
"""Pin the generated workflows' actions to the versions Rullst's own workflows use.

`cargo rullst new` writes
cargo-rullst/src/generators/project/ci/.github/workflows/ci.yml and
`cargo rullst add browser-tests` writes
cargo-rullst/src/generators/browser_tests/assets/browser-tests.yml. Each
`uses:` line there must carry a commit SHA and `# version` comment that one of
Rullst's own workflows (.github/workflows/*.yml) uses; the cargo-rullst tests
`generators::project::ci::tests::the_workflow_pins_rullsts_own_actions_with_minimal_permissions`
and `generators::browser_tests::tests` fail otherwise. Dependabot updates only
.github/workflows (outside the root directory it reads composite `action.yml`
files only, see .github/dependabot.yml), so after a github-actions update run,
from the repository root:

    python3 .github/sync-template-action-pins.py          # rewrite the templates
    python3 .github/sync-template-action-pins.py --check  # fail when one is stale

Each template pin is replaced by the pin Rullst's workflows use for the same
action. When they use several, the newest version comment wins (`v7.1.0` over
`v7.0.1`, or the later date of a `main @ 2026-11-15` comment); comments that
cannot be ordered stop the script with the candidates listed. A template action
that no Rullst workflow uses is an error, as it is for the tests. Standard
library only; the tests are in .github/test-sync-template-action-pins.py.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORKFLOWS = ROOT / ".github/workflows"
TEMPLATES = (
    ROOT / "cargo-rullst/src/generators/project/ci/.github/workflows/ci.yml",
    ROOT / "cargo-rullst/src/generators/browser_tests/assets/browser-tests.yml",
)
COMMAND = "python3 .github/sync-template-action-pins.py"

USES = re.compile(
    r"^(?P<prefix> *(?:- +)?uses: *)(?P<action>[^@\s#]+)@(?P<ref>[^\s#]+)"
    r"(?: +# *(?P<comment>.*?))? *$"
)
SHA = re.compile(r"[0-9a-f]{40}")
SEMVER = re.compile(r"v?(\d+(?:\.\d+)*)\b")
DATE = re.compile(r"\b(\d{4}-\d{2}-\d{2})\b")

Pin = tuple[str, str]  # (sha, comment)


class SyncError(Exception):
    """A template pin that cannot be chosen automatically."""


def references(text: str, name: str, strict: bool) -> list[tuple[int, re.Match[str]]]:
    """Every `owner/repo@ref` `uses:` line of a workflow, with its index. A
    template (strict) may hold no other kind; Rullst's workflows may also call
    local reusable workflows (`uses: ./...`)."""
    found = []
    for index, line in enumerate(text.split("\n")):
        if re.match(r"^ *(?:- +)?uses:", line):
            match = USES.match(line)
            if match is not None:
                found.append((index, match))
            elif strict:
                raise SyncError(f"{name}:{index + 1}: not a plain `uses: owner/repo@sha # version` line")
    return found


def own_pins(workflows: Path) -> dict[str, set[Pin]]:
    """The SHA and comment each remote action has in Rullst's own workflows."""
    pins: dict[str, set[Pin]] = {}
    for path in sorted(workflows.glob("*.yml")):
        text = path.read_text(encoding="utf-8")
        for _, match in references(text, path.name, strict=False):
            action, ref, comment = match["action"], match["ref"], (match["comment"] or "").strip()
            if action.startswith(("./", "docker://")) or not SHA.fullmatch(ref) or not comment:
                continue
            pins.setdefault(action, set()).add((ref, comment))
    return pins


def order_key(comment: str) -> tuple[str, tuple]:
    """How a version comment sorts: a version number, else a date."""
    version = SEMVER.match(comment)
    if version:
        return "version", tuple(int(part) for part in version[1].split("."))
    date = DATE.search(comment)
    if date:
        return "date", (date[1],)
    return "unordered", (comment,)


def choose(action: str, candidates: set[Pin]) -> Pin:
    """The pin a template should use among those Rullst's workflows use."""
    if len(candidates) == 1:
        return next(iter(candidates))
    keyed = [(order_key(comment), (sha, comment)) for sha, comment in candidates]
    kinds = {kind for (kind, _), _ in keyed}
    if len(kinds) != 1 or "unordered" in kinds:
        listed = ", ".join(f"{sha} # {comment}" for sha, comment in sorted(candidates))
        raise SyncError(f"{action}: cannot order the pins Rullst's workflows use: {listed}")
    keyed.sort(key=lambda item: item[0][1])
    newest = keyed[-1][0]
    tied = {pin for key, pin in keyed if key == newest}
    if len(tied) > 1:
        listed = ", ".join(f"{sha} # {comment}" for sha, comment in sorted(tied))
        raise SyncError(f"{action}: several pins share the newest version: {listed}")
    return keyed[-1][1]


def sync_text(text: str, pins: dict[str, set[Pin]], name: str) -> tuple[str, list[str]]:
    """The template with every pin taken from `pins`, and what changed."""
    lines = text.split("\n")
    changes = []
    for index, match in references(text, name, strict=True):
        action = match["action"]
        if action not in pins:
            raise SyncError(f"{name}:{index + 1}: no Rullst workflow uses {action}")
        sha, comment = choose(action, pins[action])
        current = (match["ref"], (match["comment"] or "").strip())
        if current != (sha, comment):
            lines[index] = f"{match['prefix']}{action}@{sha} # {comment}"
            changes.append(f"{name}:{index + 1}: {action}@{current[0]} # {current[1]} -> {sha} # {comment}")
    return "\n".join(lines), changes


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    parser.add_argument("--check", action="store_true", help="change nothing; exit 1 when a template is stale")
    args = parser.parse_args(argv)

    # The new text of each stale template, by its index in TEMPLATES. Nothing
    # is written until every template has been read and synced.
    synced_texts: dict[int, str] = {}
    try:
        pins = own_pins(WORKFLOWS)
        for index, path in enumerate(TEMPLATES):
            name = path.relative_to(ROOT).as_posix() if path.is_relative_to(ROOT) else str(path)
            synced, changes = sync_text(path.read_text(encoding="utf-8"), pins, name)
            if changes:
                print("\n".join(changes))
                synced_texts[index] = synced
    except SyncError as error:
        print(f"sync-template-action-pins: {error}", file=sys.stderr)
        return 2

    if not synced_texts:
        print("The generated workflows pin the same actions as Rullst's own workflows.")
        return 0
    if args.check:
        print(f"Generated workflows are out of date: run `{COMMAND}` and commit the result.", file=sys.stderr)
        return 1
    for index, path in enumerate(TEMPLATES):
        if index in synced_texts:
            with path.open("w", encoding="utf-8") as template:
                template.write(synced_texts[index])
    print("Updated; commit the templates with the workflow change.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
