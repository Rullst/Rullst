#!/usr/bin/env python3
"""Mutate the ORM's optional backends against their suites on the SQLx Any pool.

Like `orm-sqlite`, but `rullst-orm` is also built with its Redis, MongoDB,
SurrealDB, Turso, Scout HTTP and Qdrant features, so the polyglot, search,
Redis data and query-cache modules and the tests that exercise them through
mocks and loopback servers are compiled. A disposable Redis server that this
wrapper owns serves the opt-in `RULLST_TEST_REDIS_URL` contracts of the query
cache and the Redis hash scope, which the all-feature build compiles out. The
testcontainers matrices are skipped. DuckDB, whose bundled build is large, is
left to targeted runs.
"""
from __future__ import annotations

import importlib.util
import os
import re
import subprocess
import sys
import time
import uuid
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("check_orm_sqlite", HERE / "check-orm-sqlite.py")
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot import check-orm-sqlite.py")
ORM_SQLITE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ORM_SQLITE)

REDIS_IMAGE = "redis:7.4-alpine@sha256:ff02b58f971e7d7d156a1267e283fcbbeee91773b6aa36c49dac28ecfe28eadf"
FEATURES = ["drivers-all", "redis", "mongodb", "surrealdb", "turso", "scout-http", "qdrant"]


def redis_url(name: str) -> str:
    binding = subprocess.check_output(["docker", "port", name, "6379/tcp"], text=True,
                                      timeout=10).strip().splitlines()[0]
    if not re.fullmatch(r"127\.0\.0\.1:[0-9]+", binding):
        raise RuntimeError("Redis must be bound only to loopback")
    return "redis://" + binding


def wait_ready(name: str) -> None:
    for _ in range(60):
        result = subprocess.run(["docker", "exec", name, "redis-cli", "ping"],
                                capture_output=True, text=True, timeout=10, check=False)
        if result.stdout.strip() == "PONG":
            return
        time.sleep(1)
    raise RuntimeError("Owned Redis server did not become ready")


def main(argv: list[str] | None = None) -> int:
    arguments = ORM_SQLITE.parse_arguments(__doc__, argv)
    mutants = ORM_SQLITE.command(arguments.mutants, arguments.shard, FEATURES)
    name = "rullst-orm-polyglot-" + uuid.uuid4().hex
    started = False
    try:
        subprocess.run(["docker", "run", "--detach", "--rm", "--name", name,
                        "--publish", "127.0.0.1::6379", REDIS_IMAGE],
                       check=True, timeout=300, stdout=subprocess.DEVNULL)
        started = True
        wait_ready(name)
        env = dict(os.environ, RULLST_TEST_REDIS_URL=redis_url(name))
        return ORM_SQLITE.run(mutants, env)
    finally:
        if started:
            subprocess.run(["docker", "stop", "--time", "5", name],
                           check=False, timeout=60, stdout=subprocess.DEVNULL)


if __name__ == "__main__":
    sys.exit(main())
