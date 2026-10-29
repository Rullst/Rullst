#!/usr/bin/env python3
"""Mutate the ORM against `matrix_mysql` with the strict MySQL pool.

`orm-sql-servers` builds the SQLx `Any` pool; code compiled only with
`strict-mysql` (native enum types, typed codecs and binds) runs here, in the
build that applications selecting that feature use. The testcontainers server
must start, or the run fails.
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


if __name__ == "__main__":
    sys.exit(SERVERS.main(features=["strict-mysql"], targets=["--test=matrix_mysql"],
                          description=__doc__))
