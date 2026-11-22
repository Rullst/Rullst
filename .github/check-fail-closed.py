#!/usr/bin/env python3
"""Fail when an unreviewed offline-mock or local fallback appears in the source.

AGENTS.md section 3.5: an empty or missing credential, endpoint or URL selects
an offline mock (or a local fallback) only in development and test; in staging
and production the code that reads the settings returns a configuration error.
This check scans the non-test Rust source of the workspace and the Rust
templates of generated projects (`fail_closed_source.py` says which files),
finds each item that contains one of the `SIGNALS` below, and compares those
sites with the reviewed inventory `.github/fail-closed-sites.json`. It fails on
a site that is not in the inventory, an entry that matches no site any more,
and an invalid entry.

A site is the innermost item around a signal, named with its enclosing items
(`TursoStore::connect`, `billing_config`, a `const` or `static` name). A
signal outside any item, as in a template fragment, is a site of its own line;
its anchor is a snippet of that line. Each entry has a `path`, an `anchor`, a
`classification` and a one-line `reason`:

- `refuses-in-secure-environments`: the site refuses an empty value in staging
  and production (or the mock everywhere); `refusal` is an identifier of that
  refusal, which must appear in the site. Its file must not resolve the
  environment from the process variables alone (`PARTIAL_ENVIRONMENT`), since
  `Server` also reads `.env` and `[app] env`.
- `explicit-mock-only`: only an explicit `mock_*` value, or a mock type or
  feature the caller chose, reaches the mock.
- `caller-provided-values`: a constructor or helper that receives its values
  from the caller and reads no settings itself.
- `documented-exception`: reads settings and does not refuse; `docs` lists the
  files that state it, each of which must contain `documented_as` (by default
  the anchor).

    python3 .github/check-fail-closed.py           # check
    python3 .github/check-fail-closed.py --sites   # print every site as JSON

The patterns are calibrated on this repository and search text, not types:
they find the known shapes of the defect and do not prove that no other
fallback exists. Likewise `PARTIAL_ENVIRONMENT` finds the two calls that
leave out `.env` and `[app] env`; it cannot tell whether an environment read
in another file, or through `RullstConfig::global()` before `Server` has
started, sees every source.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

from fail_closed_source import enclosing, production_sources

ROOT = Path(__file__).resolve().parent.parent
INVENTORY = Path(".github/fail-closed-sites.json")

# Functions that decide from a credential whether the mock or the live
# service is used. A new helper is a site itself (its `mock` prefix check);
# listing it here also makes its callers sites. Capital's
# `stripe_contract::credential_mode` only tells live keys from test keys.
HELPERS = (
    "checked_mode",
    "credential_mode",
    "credential_mode_for_values",
    "default_driver_name",
    "from_credential",
    "is_mock_credential",
    "mock_requested",
    "offline_credential",
    "require_credential",
    "webhook_mode_from_secret",
)

CREDENTIAL_NAME = r"[A-Z0-9_]*(?:KEY|SECRET|TOKEN|PASSWORD|URL|URI|HOST|DSN|ENDPOINT|CREDENTIAL|AUTH)[A-Z0-9_]*"
MOCK_VARIANT = r"(?:[A-Z][A-Za-z0-9_]*::)+(?:Mock|Offline|Fixture)[A-Za-z0-9_]*\b"
QUOTE = r'\\?"'  # also an escaped quote, in code generated from a string literal
EMPTY_DEFAULT = (
    r"\.(?:unwrap_or_default\(\)"
    rf"|unwrap_or\(\s*(?:{QUOTE}{QUOTE}|String::new\(\))\s*\)"
    r"|unwrap_or_else\(\s*(?:\|_?\|\s*String::new\(\)|String::new)\s*\))"
)
LOCAL_STORE = rf"{QUOTE}(?:sqlite:[^\"\\]*|[^\"\\\s]*\.(?:db|sqlite3?)|:memory:){QUOTE}"

SIGNALS = {
    # A value compared with a `mock` prefix: `starts_with("mock_")`.
    "mock-prefix": re.compile(
        rf"\.(?:starts_with|strip_prefix|eq_ignore_ascii_case|ends_with)\(\s*{QUOTE}mock"
        rf"|(?:==|!=)\s*{QUOTE}mock\b|{QUOTE}mock[_:]?{QUOTE}\s*(?:==|!=)"
        r"|\.(?:starts_with|strip_prefix)\(\s*[A-Z_]*MOCK[A-Z_]*\s*\)"
    ),
    # A `Mock`, `Offline` or `Fixture` variant stored in a field or chosen by
    # an `if`/`else` branch, in a constructor (see `CONSTRUCTOR`).
    "mock-variant": re.compile(
        rf"\b[a-z_][a-z0-9_]*\s*:\s*{MOCK_VARIANT}"
        rf"|\{{\s*{MOCK_VARIANT}(?:\([^;{{}}]*\))?\s*\}}\s*else\b"
        rf"|\belse\s*\{{\s*{MOCK_VARIANT}"
    ),
    # A call to a credential-mode helper, or a check of an OAuth provider's mode.
    "mode-helper": re.compile(
        rf"(?<![.\w])(?<!fn )(?<!stripe_contract::)(?:{'|'.join(HELPERS)})\s*\("
        r"|\.credential_mode\(\)\s*\.is_mock\(\)"
    ),
    # A credential-like setting read by name that defaults to an empty string.
    "empty-setting-default": re.compile(
        rf"\b[A-Za-z_][A-Za-z0-9_:]*\(\s*{QUOTE}{CREDENTIAL_NAME}{QUOTE}\s*\)[^;{{}}]*?{EMPTY_DEFAULT}"
    ),
    # A constructor given a literal `mock_` credential.
    "mock-credential": re.compile(rf"\b(?:new|try_new|with_[a-z_]+|from_[a-z_]+)\(\s*{QUOTE}mock_"),
    # A `NAME=mock_…` setting, or a `("NAME", "mock_…")` pair, written into a
    # generated project's configuration.
    "generated-mock-setting": re.compile(
        rf"(?<![A-Z0-9_])[A-Z][A-Z0-9_]*=mock_|{QUOTE}_*[A-Z][A-Z0-9_]*{QUOTE}\s*,\s*{QUOTE}mock_"
    ),
    # An `is_empty()` condition whose block selects a mock, offline or fixture value.
    "empty-selects-mock": re.compile(
        r"\.is_empty\(\)[^;{}]*\{[^{}]{0,160}?"
        r"(?:\bMock\b|::(?:Mock|Offline|Fixture)\w*|\b(?:mock|offline)\(|OFFLINE_PATH)"
    ),
    # A local SQLite file or in-memory database used as the default.
    "local-store-default": re.compile(
        rf"(?:\bunwrap_or(?:_else)?\(|\breturn\s+)[^;{{}}]{{0,80}}?{LOCAL_STORE}"
    ),
}
# An environment resolved without `.env` and `[app] env`, which `Server` also
# reads: `Environment::detect(None)`, or `Environment::resolve` whose last
# argument (`[app] env`) is `None`.
PARTIAL_ENVIRONMENT = re.compile(
    r"\bEnvironment::detect\(\s*None\s*\)"
    r"|\bEnvironment::resolve\((?:[^()]|\([^()]*\))*?,\s*None\s*,?\s*\)"
)

# Elsewhere a mock variant is dispatch on a mode that was chosen earlier.
CONSTRUCTOR_ONLY = {"mock-variant"}
CONSTRUCTOR = re.compile(
    r"new|try_new|open|build|try_build|init\w*|connect\w*|from_\w+|with_\w+|offline|mock"
    r"|\w*mode\w*|\w*credential\w*"
)

CLASSIFICATIONS = {
    "refuses-in-secure-environments": {"refusal"},
    "explicit-mock-only": set(),
    "caller-provided-values": set(),
    "documented-exception": {"docs"},
}
OPTIONAL_FIELDS = {"documented-exception": {"documented_as"}}


class Site:
    def __init__(self, path: str, anchor: str, top_level: bool, body: str, code: str) -> None:
        self.path = path
        self.anchor = anchor
        self.top_level = top_level
        self.body = body
        self.code = code  # the whole file, test code blanked
        self.signals: dict[str, set[int]] = {}

    def describe(self) -> str:
        found = ", ".join(f"{name} line {min(lines)}" for name, lines in sorted(self.signals.items()))
        return f"{self.path} {self.anchor} ({found})"


def find_sites(root: Path) -> dict[tuple[str, str], Site]:
    sites: dict[tuple[str, str], Site] = {}
    for path, code, spans in production_sources(root):
        lines = code.splitlines()
        for signal, pattern in SIGNALS.items():
            for match in pattern.finditer(code):
                anchor, span = enclosing(spans, match.start())
                if signal in CONSTRUCTOR_ONLY and not CONSTRUCTOR.fullmatch(anchor.rsplit("::", 1)[-1]):
                    continue
                line = code.count("\n", 0, match.start()) + 1
                if span is None:
                    anchor, body = " ".join(lines[line - 1].split()), code
                else:
                    body = code[span[0] : span[1] + 1]
                site = sites.setdefault((path, anchor), Site(path, anchor, span is None, body, code))
                site.signals.setdefault(signal, set()).add(line)
    return sites


def entry_problems(root: Path, entry: dict) -> list[str]:
    """What is wrong with one entry, independently of the scan."""
    classification = entry.get("classification")
    if classification not in CLASSIFICATIONS:
        return [f"unknown classification {classification!r}"]
    required = {"path", "anchor", "classification", "reason"} | CLASSIFICATIONS[classification]
    allowed = required | OPTIONAL_FIELDS.get(classification, set())
    problems = [f"missing field {name!r}" for name in sorted(required - entry.keys())]
    problems += [f"unexpected field {name!r}" for name in sorted(entry.keys() - allowed)]
    if problems:
        return problems
    if not str(entry["reason"]).strip():
        problems.append("empty reason")
    if "refusal" in entry and not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", str(entry["refusal"])):
        problems.append("refusal is not an identifier")
    if classification == "documented-exception":
        named = entry.get("documented_as", entry["anchor"])
        if not entry["docs"]:
            problems.append("no docs listed")
        for doc in entry["docs"]:
            document = root / doc
            if not document.is_file():
                problems.append(f"doc {doc} does not exist")
            elif named not in document.read_text(encoding="utf-8", errors="replace"):
                problems.append(f"doc {doc} does not contain {named}")
    return problems


def covers(entry: dict, site: Site) -> bool:
    if entry["path"] != site.path:
        return False
    return entry["anchor"] in site.anchor if site.top_level else entry["anchor"] == site.anchor


def compare(root: Path, sites: dict[tuple[str, str], Site], entries: list[dict]) -> list[str]:
    """Every difference between the scan and the inventory, one line each."""
    findings = []
    seen = set()
    covered = set()
    for entry in entries:
        label = f"{entry.get('path')} {entry.get('anchor')}"
        if label in seen:
            findings.append(f"duplicate entry: {label}")
            continue
        seen.add(label)
        if problems := entry_problems(root, entry):
            findings.extend(f"invalid entry: {label}: {problem}" for problem in problems)
            continue
        matched = [key for key, site in sites.items() if covers(entry, site)]
        if not matched:
            findings.append(f"stale entry (no signal matches it): {label}")
        covered.update(matched)
        if entry["classification"] == "refuses-in-secure-environments":
            refusal = re.compile(rf"(?<![A-Za-z0-9_]){re.escape(entry['refusal'])}(?![A-Za-z0-9_])")
            if any(not refusal.search(sites[key].body) for key in matched):
                findings.append(f"refusal missing: {label} no longer contains `{entry['refusal']}`")
            for key in matched:
                code = sites[key].code
                if partial := PARTIAL_ENVIRONMENT.search(code):
                    line = code.count("\n", 0, partial.start()) + 1
                    findings.append(
                        f"partial environment: {label}: line {line} resolves the environment from fewer "
                        "sources than `Server` (the process, `.env`, then `[app] env`)"
                    )
                    break
    findings.extend(f"unlisted site: {sites[key].describe()}" for key in sorted(set(sites) - covered))
    return findings


def main(arguments: list[str], root: Path = ROOT) -> int:
    if arguments not in ([], ["--sites"]):
        print("usage: check-fail-closed.py [--sites]", file=sys.stderr)
        return 2
    sites = find_sites(root)
    if arguments == ["--sites"]:
        listing = [
            {
                "path": site.path,
                "anchor": site.anchor,
                "signals": {name: sorted(lines) for name, lines in sorted(site.signals.items())},
            }
            for _, site in sorted(sites.items())
        ]
        print(json.dumps(listing, indent=2))
        return 0
    entries = json.loads((root / INVENTORY).read_text(encoding="utf-8"))["sites"]
    findings = compare(root, sites, entries)
    if not findings:
        print(f"All {len(sites)} offline-fallback sites are classified in {INVENTORY}.")
        return 0
    print(f"{INVENTORY} does not match the source:")
    for finding in findings:
        print(f"  {finding}")
    print(
        "An empty or missing setting may select an offline mock or a local fallback only in "
        "development and test (AGENTS.md section 3.5). Every site needs a classified entry in "
        f"{INVENTORY}; .github/check-fail-closed.py describes the signals and classifications."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
