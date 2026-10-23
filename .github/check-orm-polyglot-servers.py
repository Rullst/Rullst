#!/usr/bin/env python3
"""Mutate the ORM's optional backends against their server matrices.

`orm-polyglot` reaches the MongoDB, SurrealDB, Qdrant, Meilisearch, Redis
data and Turso adapters through mocks and loopback fakes. This contract runs
their testcontainers matrices for every selected mutant, so the code that
talks to a real server is measured too; a server that cannot start fails the
run. Select the adapters' live paths with `--mutants REGEX`.
"""
from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("check_orm_sql_servers", HERE / "check-orm-sql-servers.py")
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot import check-orm-sql-servers.py")
SERVERS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SERVERS)

FEATURES = ["drivers-all", "redis", "mongodb", "surrealdb", "turso", "scout-http", "qdrant"]
MATRICES = ["matrix_mongodb", "matrix_surrealdb", "matrix_document_recovery", "matrix_qdrant",
            "matrix_meilisearch", "matrix_redis_data", "matrix_turso"]


if __name__ == "__main__":
    sys.exit(SERVERS.main(features=FEATURES, targets=["--test=" + name for name in MATRICES],
                          description=__doc__))
