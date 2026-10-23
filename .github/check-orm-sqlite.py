#!/usr/bin/env python3
"""Mutate the ORM against its SQLite suite (the SQLx Any pool), without a server.

Targeted and full mutation runs build the workspace with every feature, which
selects the strict PostgreSQL pool and compiles the SQLite behaviour tests of
`rullst-orm` out, so mutants that only a database statement reaches are
reported as missed there. This run builds `rullst-orm` with `drivers-all`
instead: the pool is SQLx's `Any` pool and the integration tests run against
in-memory and temporary-file SQLite databases.

It mutates `rullst-orm` and `rullst-orm-macros` (test files excluded) and, for
every mutant, runs the `rullst-orm` test targets, so the code that the derive
generates is exercised too. `--mutants REGEX` selects mutants by name, for
example `^rullst-orm/src/pool/savepoint\\.rs:`. The testcontainers matrices are
skipped (`DOCKER_HOST` points nowhere) and the Redis-backed tests stay compiled
out without the `redis` feature, so no server is started.
"""
from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

INFORMATIONAL = {0, 2, 3}  # all caught, some missed, some timed out
SHARD = re.compile(r"[0-9]+/[1-9][0-9]*")
FILES = ["rullst-orm/src/**", "rullst-orm-macros/src/**"]
EXCLUDES = ["**/*_tests.rs", "**/tests.rs", "**/tests/**"]
FEATURES = ["drivers-all"]


def command(regex: str, shard: str | None, features: list[str] = FEATURES,
            targets: list[str] | None = None, test_arguments: list[str] | None = None,
            timeout: int = 300) -> list[str]:
    """The cargo-mutants command; `targets` default to every test target."""
    if shard is not None and not SHARD.fullmatch(shard):
        raise ValueError("--shard takes K/N")
    # In place: a copy of the workspace under the runner's temporary directory
    # runs out of quota once every ORM test binary is linked.
    mutants = ["cargo", "mutants", "--no-config", "--in-place",
               "--package", "rullst-orm", "--package", "rullst-orm-macros",
               "--test-package", "rullst-orm",
               "--no-default-features",
               "--features", ",".join("rullst-orm/" + feature for feature in features),
               "--gitignore", "true", "--jobs", "1", "--cap-lints", "false",
               "--build-timeout", "600", "--timeout", str(timeout),
               "--cargo-arg=--locked", "--cargo-arg=--offline",
               *("--cargo-arg=" + target for target in (targets or ["--tests"])),
               *(argument for path in FILES for argument in ("--file", path)),
               *(argument for path in EXCLUDES for argument in ("--exclude", path)),
               "--re", regex, "--caught", "--unviable"]
    if shard is not None:
        mutants += ["--shard", shard]
    if test_arguments:
        mutants += ["--", "--", *test_arguments]
    return mutants


def ran_baseline(output: Path) -> bool:
    return (output / "mutants.out" / "log" / "baseline.log").is_file()


def run(mutants: list[str], env: dict[str, str], containers: bool = False) -> int:
    """Runs cargo-mutants; the testcontainers matrices are skipped unless
    `containers` requires them, so a server that fails to start fails the
    baseline instead of skipping its contract."""
    env = dict(env)
    if containers:
        env.update(RULLST_REQUIRE_TESTCONTAINERS="true", TESTCONTAINERS_RYUK_DISABLED="true")
    else:
        env["DOCKER_HOST"] = "unix:///nonexistent"
        env.pop("RULLST_REQUIRE_TESTCONTAINERS", None)
    status = subprocess.run(mutants, env=env, check=False, timeout=19000).returncode
    if status in INFORMATIONAL and not ran_baseline(Path(env.get("CARGO_MUTANTS_OUTPUT", "."))):
        print("cargo-mutants ran no baseline; the regex or shard selected no mutant.",
              file=sys.stderr)
        return 1
    return status


def parse_arguments(description: str | None, argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=description)
    parser.add_argument("--mutants", metavar="REGEX", required=True,
                        help="cargo-mutants --re filter over the ORM's mutants")
    parser.add_argument("--shard", metavar="K/N", help="test only this cargo-mutants shard")
    arguments = parser.parse_args(argv)
    if arguments.shard is not None and not SHARD.fullmatch(arguments.shard):
        parser.error("--shard takes K/N")
    return arguments


def main(argv: list[str] | None = None) -> int:
    arguments = parse_arguments(__doc__, argv)
    return run(command(arguments.mutants, arguments.shard), dict(os.environ))


if __name__ == "__main__":
    sys.exit(main())
