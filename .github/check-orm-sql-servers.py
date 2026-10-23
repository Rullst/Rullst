#!/usr/bin/env python3
"""Mutate the ORM against its PostgreSQL, MySQL and MariaDB matrices.

The SQL dialect branches of the ORM (PostgreSQL placeholders and advisory
locks, MySQL/MariaDB user locks, collations and payload types) run only on
those servers. This contract builds `rullst-orm` with `drivers-all` and, for
every mutant, runs `matrix_postgres`, `matrix_mysql` and `matrix_mariadb`,
whose testcontainers start a disposable server per test binary; a server that
cannot start fails the run instead of skipping its contract. Select the
dialect-specific mutants with `--mutants REGEX`, because each one waits for
three servers.
"""
from __future__ import annotations

import importlib.util
import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("check_orm_sqlite", HERE / "check-orm-sqlite.py")
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot import check-orm-sqlite.py")
ORM_SQLITE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ORM_SQLITE)

TARGETS = ["--test=matrix_postgres", "--test=matrix_mysql", "--test=matrix_mariadb"]


def main(argv: list[str] | None = None) -> int:
    arguments = ORM_SQLITE.parse_arguments(__doc__, argv)
    mutants = ORM_SQLITE.command(arguments.mutants, arguments.shard, ["drivers-all"],
                                 TARGETS, ["--test-threads=1"], timeout=900)
    return ORM_SQLITE.run(mutants, dict(os.environ), containers=True)


if __name__ == "__main__":
    sys.exit(main())
