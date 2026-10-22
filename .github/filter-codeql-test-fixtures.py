#!/usr/bin/env python3
"""Drop CodeQL test-fixture results (fixed secrets, logged test data) from test-only files.

Tests use fixed keys, passwords, salts and nonces on purpose (deterministic
fixtures, mutation-killing assertions). Every new batch of such tests opened
alerts that were then dismissed one by one as "used in tests", and assertion
messages that print a fixture (`assert!(…, "{debug}")`) opened
`rust/cleartext-logging` alerts the same way. This filter removes only results
of the rules in `RULES` located in files that are never part of a production
build:

- anything under a `tests/`, `benches/` or `fuzz/` directory;
- `tests.rs`, `test.rs`, `*_tests.rs` and `*_test.rs` files (in this
  repository these are `#[cfg(test)]` modules).

Every other rule, and these rules anywhere else (`src/`, examples, generator
templates), is kept. `PRODUCTION_FILES` lists the files whose names match the
test pattern but which are compiled into a release build; they are kept too.
Inline `#[cfg(test)] mod tests` blocks inside production files are not
recognised and still need a manual review.

Usage: filter-codeql-test-fixtures.py <sarif-file>  (rewritten in place)
"""

import json
import re
import sys
from pathlib import Path

RULES = frozenset({
    "rust/hard-coded-cryptographic-value",
    "rust/cleartext-logging",
})
TEST_PATH = re.compile(
    r"(^|/)(tests|benches|fuzz)/"
    r"|(^|/)(tests?|[^/]+_tests?)\.rs$"
)

# Names match TEST_PATH, but these modules are not `#[cfg(test)]`: the first
# generates an application's security test file, the second implements the
# `#[rullst_orm::test]` attribute.
PRODUCTION_FILES = frozenset({
    "cargo-rullst/src/blueprints/security_tests.rs",
    "rullst-orm-macros/src/sandbox_test.rs",
})


def result_path(result):
    locations = result.get("locations") or []
    if not locations:
        return None
    artifact = locations[0].get("physicalLocation", {}).get("artifactLocation", {})
    return artifact.get("uri")


def is_test_fixture(result):
    if result.get("ruleId") not in RULES:
        return False
    path = result_path(result)
    return (
        path is not None
        and path not in PRODUCTION_FILES
        and TEST_PATH.search(path) is not None
    )


def filter_sarif(sarif):
    """Removes test-fixture results in place and returns the removed paths."""
    removed = []
    for run in sarif.get("runs", []):
        kept = []
        for result in run.get("results", []):
            if is_test_fixture(result):
                removed.append(result_path(result))
            else:
                kept.append(result)
        run["results"] = kept
    return removed


def main(argv):
    if len(argv) != 2:
        print("usage: filter-codeql-test-fixtures.py <sarif-file>", file=sys.stderr)
        return 2
    path = Path(argv[1])
    sarif = json.loads(path.read_text(encoding="utf-8"))
    removed = filter_sarif(sarif)
    path.write_text(json.dumps(sarif), encoding="utf-8")
    print(f"removed {len(removed)} test-fixture result(s) ({', '.join(sorted(RULES))}) in test-only files")
    for item in sorted(set(removed)):
        print(f"  {item}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
