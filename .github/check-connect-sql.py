#!/usr/bin/env python3
"""Exercise Connect's SQL login-challenge store on owned PostgreSQL and MySQL servers."""
import argparse
import os
import re
import subprocess
import sys
import time
import uuid

import mutation_contract

POSTGRES_IMAGE = "postgres@sha256:29342cb52157b098821961d2c14eec3c019071f56a5d559e990cf07cf541ea9b"
MYSQL_IMAGE = "mysql@sha256:76f0358884cb92d9a086d24e5671c38048d3fef40f2588703d73eb55392d6b69"
DATABASE = "rullst_connect_contract"
TESTS = ["postgres_challenge_contract", "mysql_challenge_contract"]


def wait_until(command, what):
    for _ in range(180):
        result = subprocess.run(command, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                timeout=10)
        if result.returncode == 0:
            return
        time.sleep(1)
    raise RuntimeError(f"Owned {what} server did not become ready")


def loopback(name, port):
    binding = subprocess.check_output(["docker", "port", name, port], text=True, timeout=10).strip()
    if not re.fullmatch(r"127\.0\.0\.1:[0-9]+", binding):
        raise RuntimeError("Owned database servers must be bound only to loopback")
    return binding


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--coverage", action="store_true",
                        help="accumulate profiles with cargo llvm-cov --no-report")
    parser.add_argument("--mutants", metavar="REGEX",
                        help="mutate the PostgreSQL and MySQL adapters and run their contracts")
    parser.add_argument("--shard", metavar="K/N", help="with --mutants, test only this cargo-mutants shard")
    args = parser.parse_args()
    if args.mutants and args.coverage:
        parser.error("mutation runs use the workspace, without coverage")
    if args.shard and not args.mutants:
        parser.error("--shard needs --mutants")
    suffix = uuid.uuid4().hex
    postgres = "rullst-connect-postgres-" + suffix
    mysql = "rullst-connect-mysql-" + suffix
    started = []
    try:
        subprocess.run(["docker", "run", "--detach", "--rm", "--name", postgres,
                        "--env", "POSTGRES_HOST_AUTH_METHOD=trust", "--env", "POSTGRES_DB=" + DATABASE,
                        "--publish", "127.0.0.1::5432", POSTGRES_IMAGE],
                       check=True, timeout=300, stdout=subprocess.DEVNULL)
        started.append(postgres)
        subprocess.run(["docker", "run", "--detach", "--rm", "--name", mysql,
                        "--env", "MYSQL_ALLOW_EMPTY_PASSWORD=yes", "--env", "MYSQL_DATABASE=" + DATABASE,
                        "--publish", "127.0.0.1::3306", MYSQL_IMAGE],
                       check=True, timeout=300, stdout=subprocess.DEVNULL)
        started.append(mysql)
        wait_until(["docker", "exec", postgres, "pg_isready", "--host", "127.0.0.1",
                    "--username", "postgres", "--dbname", DATABASE], "PostgreSQL")
        # The image's first start runs a socket-only bootstrap server; TCP
        # answers only once the final server is up with the database created.
        wait_until(["docker", "exec", mysql, "mysqladmin", "ping", "--host", "127.0.0.1",
                    "--user", "root", "--silent"], "MySQL")
        env = dict(os.environ)
        env.update({
            "RULLST_CONNECT_TEST_POSTGRES_URL":
                "postgres://postgres@" + loopback(postgres, "5432/tcp") + "/" + DATABASE,
            "RULLST_CONNECT_TEST_MYSQL_URL":
                "mysql://root@" + loopback(mysql, "3306/tcp") + "/" + DATABASE,
        })
        if args.mutants:
            # Each contract drops its table first; the runner also recreates
            # the PostgreSQL database before every test binary.
            return mutation_contract.run(
                package="rullst-connect", features="postgres,mysql",
                files=["rullst-connect/src/challenge/sql/postgres.rs",
                       "rullst-connect/src/challenge/sql/mysql.rs"],
                targets=["--test=challenge_sql_servers"], tests=TESTS,
                regex=args.mutants, shard=args.shard, env=env, timeout=120,
                reset_spec=mutation_contract.postgres(postgres, DATABASE))
        command = ["cargo", "llvm-cov", "--no-report"] if args.coverage else ["cargo", "test"]
        command += ["--locked", "-p", "rullst-connect", "--no-default-features",
                    "--features", "postgres,mysql", "--test", "challenge_sql_servers",
                    "--", "--ignored", "--exact", *TESTS, "--test-threads=1"]
        subprocess.run(command, env=env, check=True, timeout=1800)
        return 0
    except (subprocess.SubprocessError, RuntimeError):
        for name in started:
            subprocess.run(["docker", "logs", "--tail", "60", name], check=False, timeout=10)
        raise
    finally:
        for name in started:
            subprocess.run(["docker", "stop", "--time", "5", name],
                           check=False, timeout=60, stdout=subprocess.DEVNULL)


if __name__ == "__main__":
    sys.exit(main())
