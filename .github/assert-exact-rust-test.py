#!/usr/bin/env python3
"""Require one exact, non-ignored libtest success after Cargo exits successfully.

This validates execution, not a pre-build listing. The caller must independently
require Cargo's zero exit status; a log is not a signed release receipt.
"""

import argparse
import re
import sys
from pathlib import Path

MAX_LOG_BYTES = 32 * 1024 * 1024
TEST_NAME = re.compile(r"[A-Za-z0-9_:]{1,512}")
RESULT = re.compile(
    r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
    r"(\d+) measured; (\d+) filtered out; finished in ([0-9.]+)s"
)


def validate(output: str, expected: str) -> None:
    if not TEST_NAME.fullmatch(expected):
        raise ValueError("unsafe exact test name")
    lines = output.splitlines()
    successes = [line for line in lines if line in {
        f"test {expected} ... ok", f"test {expected} - should panic ... ok",
    }]
    summaries = [line for line in lines if line.startswith("test result:")]
    starts = [line for line in lines if re.fullmatch(r"running \d+ tests?", line)]
    if len(successes) != 1 or starts != ["running 1 test"] or len(summaries) != 1:
        raise ValueError("expected exactly one execution of the named test")
    summary = RESULT.fullmatch(summaries[0])
    if summary is None or summary.groups()[:5] != ("ok", "1", "0", "0", "0"):
        raise ValueError("the exact test did not pass without ignored or failed cases")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("log", type=Path)
    parser.add_argument("test")
    args = parser.parse_args()
    try:
        with args.log.open("rb") as log:
            raw = log.read(MAX_LOG_BYTES + 1)
        if len(raw) > MAX_LOG_BYTES:
            raise ValueError("test output exceeds the 32 MiB evidence limit")
        validate(raw.decode("utf-8"), args.test)
    except (OSError, UnicodeError, ValueError) as error:
        print(f"Exact-test evidence rejected: {error}", file=sys.stderr)
        return 1
    print(f"Verified exact execution: {args.test}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
