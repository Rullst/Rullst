#!/usr/bin/env python3
"""Mutation mode shared by the wrappers that own a disposable server.

A wrapper starts its server and calls `run` with its adapter files and the
contract tests of its first (exercise) phase. cargo-mutants then starts every
Cargo test binary through this file, set as the target runner, which resets
the owned server first. Each mutant therefore sees the fresh state the
baseline saw, not rows, runtime roles, `ALTER SYSTEM` settings or a full Redis
left behind by an earlier mutant whose test stopped halfway. The database name
and URL stay the same, because the contracts assert them.

`python3 .github/mutation_contract.py BINARY [ARGS...]` is that runner; it is
not meant to be started by hand.
"""

from __future__ import annotations

import json
import os
import re
import shlex
import subprocess
import sys
import tempfile
import time
from pathlib import Path


RESET = "RULLST_MUTANTS_RESET"
INNER_RUNNER = "RULLST_MUTANTS_INNER_RUNNER"
RESET_FAILURES = "RULLST_MUTANTS_RESET_FAILURES"
INFORMATIONAL = {0, 2, 3}  # all caught, some missed, some timed out
SHARD = re.compile(r"[0-9]+/[1-9][0-9]*")

# Runtime roles and ALTER SYSTEM settings outlive a dropped database.
TERMINATE = ("SELECT count(pg_catalog.pg_terminate_backend(pid)) FROM pg_catalog.pg_stat_activity "
             "WHERE pid <> pg_catalog.pg_backend_pid() AND datname IS NOT NULL")
DROP_ROLES = ("DO $$DECLARE r record; BEGIN FOR r IN SELECT rolname FROM pg_catalog.pg_roles "
              "WHERE rolname <> 'postgres' AND rolname NOT LIKE 'pg\\_%' "
              "LOOP EXECUTE format('DROP ROLE %I', r.rolname); END LOOP; END$$")
APPLIED = ("SELECT NOT EXISTS (SELECT 1 FROM pg_catalog.pg_settings "
           "WHERE sourcefile LIKE '%postgresql.auto.conf')")


def postgres(container: str, database: str) -> dict[str, str]:
    """Reset for a trust-authenticated PostgreSQL container and its one database."""
    if not re.fullmatch(r"[a-z_][a-z0-9_]*", database):
        raise ValueError("database names are plain lowercase identifiers")
    return {"kind": "postgres", "container": container, "database": database}


def redis(container: str, password_env: str) -> dict[str, str]:
    """Reset for a password-protected Redis container; the password stays in the environment."""
    return {"kind": "redis", "container": container, "password_env": password_env}


def psql(container: str, *args: str) -> str:
    return subprocess.run(
        ["docker", "exec", container, "psql", "--no-psqlrc", "--quiet", "--tuples-only",
         "--no-align", "--set", "ON_ERROR_STOP=1", "--username", "postgres",
         "--dbname", "postgres", *args],
        check=True, capture_output=True, text=True, timeout=60,
    ).stdout.strip()


def reset(spec: dict[str, str]) -> None:
    container = spec["container"]
    if spec["kind"] == "postgres":
        database = spec["database"]
        psql(container, "--command", TERMINATE,
             "--command", f'DROP DATABASE IF EXISTS "{database}"',
             "--command", DROP_ROLES,
             "--command", "ALTER SYSTEM RESET ALL",
             "--command", "SELECT pg_catalog.pg_reload_conf()",
             "--command", f'CREATE DATABASE "{database}"')
        # The reload is asynchronous; a new session shows when it applied.
        for _ in range(100):
            if psql(container, "--command", APPLIED) == "t":
                return
            time.sleep(0.1)
        raise RuntimeError("PostgreSQL did not reload the reset configuration")
    if spec["kind"] == "redis":
        reply = subprocess.run(
            ["docker", "exec", "--env", "REDISCLI_AUTH=" + os.environ[spec["password_env"]],
             container, "redis-cli", "FLUSHALL", "SYNC"],
            check=True, capture_output=True, text=True, timeout=60,
        ).stdout.strip()
        if reply != "OK":
            raise RuntimeError("Redis did not flush")
        return
    raise ValueError("unknown reset kind")


def runner(argv: list[str]) -> int:
    """Reset the owned server, then replace this process with the test binary."""
    if not argv:
        print(__doc__, file=sys.stderr)
        return 2
    try:
        reset(json.loads(os.environ[RESET]))
    except (KeyError, ValueError, RuntimeError, OSError, subprocess.SubprocessError) as error:
        message = f"{Path(argv[0]).name}: {type(error).__name__}"
        print("mutation contract reset failed, test binary not started: " + message,
              file=sys.stderr)
        failures = os.environ.get(RESET_FAILURES)
        if failures:
            with open(failures, "a", encoding="utf-8") as log:
                log.write(message + "\n")
        return 101
    command = json.loads(os.environ.get(INNER_RUNNER) or "[]") + argv
    os.execvp(command[0], command)
    return 127  # not reached


def host_runner_variable() -> str:
    host = subprocess.run(["rustc", "-vV"], check=True, capture_output=True,
                          text=True, timeout=60).stdout
    triple = re.search(r"^host: (\S+)$", host, re.MULTILINE)
    if triple is None:
        raise RuntimeError("cannot read the host target triple")
    return "CARGO_TARGET_" + re.sub(r"[^A-Z0-9]", "_", triple.group(1).upper()) + "_RUNNER"


def baseline_tests(output: Path) -> set[str] | None:
    """Tests the unmutated baseline started, or None when cargo-mutants ran no baseline.

    A failing baseline already ends cargo-mutants with status 4, so a test that
    started there also passed. Only the start of the line is matched: a child
    process sharing stdout can print before libtest's `ok`.
    """
    log = output / "mutants.out" / "log" / "baseline.log"
    if not log.is_file():
        return None
    return set(re.findall(r"^test (\S+) \.\.\. ", log.read_text(errors="replace"),
                          re.MULTILINE))


def run(*, package: str, features: str, files: list[str], targets: list[str],
        tests: list[str], regex: str, shard: str | None, env: dict[str, str],
        reset_spec: dict[str, str], timeout: int, default_features: bool = False) -> int:
    """Mutate `files` and run only `tests` for each mutant; returns cargo-mutants' status.

    `targets` are Cargo target selectors such as `--lib` or `--test=NAME`, so a
    mutant rebuilds only the binaries that hold the contract tests.
    """
    if shard is not None and not SHARD.fullmatch(shard):
        raise ValueError("--shard takes K/N")
    env = dict(env)
    variable = host_runner_variable()
    command = [sys.executable, str(Path(__file__).resolve())]
    if any(re.search(r"\s", part) for part in command):
        raise RuntimeError("Cargo splits the runner on whitespace; move the checkout")
    env[INNER_RUNNER] = json.dumps(shlex.split(env.get(variable, "")))
    env[variable] = " ".join(command)
    env[RESET] = json.dumps(reset_spec)
    mutants = ["cargo", "mutants", "--no-config", "--package", package,
               "--features", features, "--gitignore", "true", "--jobs", "1",
               "--cap-lints", "false", "--build-timeout", "300", "--timeout", str(timeout),
               "--cargo-arg=--locked", "--cargo-arg=--offline",
               *("--cargo-arg=" + target for target in targets),
               *(argument for path in files for argument in ("--file", path)),
               "--re", regex, "--caught", "--unviable"]
    if not default_features:
        mutants.append("--no-default-features")
    if shard is not None:
        mutants += ["--shard", shard]
    mutants += ["--", "--", "--ignored", "--exact", "--test-threads=1", *tests]
    with tempfile.TemporaryDirectory(prefix="rullst-mutants-reset-") as directory:
        failures = Path(directory, "failures.log")
        env[RESET_FAILURES] = str(failures)
        status = subprocess.run(mutants, env=env, check=False, timeout=19000).returncode
        failed = failures.read_text() if failures.exists() else ""
    if failed:
        print("The owned server could not be reset before these test binaries; "
              "their mutants may be misclassified:\n" + failed, file=sys.stderr)
        return 1
    if status in INFORMATIONAL:
        started = baseline_tests(Path(env.get("CARGO_MUTANTS_OUTPUT", ".")))
        if started is None:
            print("cargo-mutants ran no baseline; the regex or shard selected no mutant.",
                  file=sys.stderr)
            return 1
        missing = sorted(set(tests) - started)
        if missing:
            print("The unmutated baseline did not run these contract tests: "
                  + ", ".join(missing), file=sys.stderr)
            return 1
    return status


if __name__ == "__main__":
    sys.exit(runner(sys.argv[1:]))
