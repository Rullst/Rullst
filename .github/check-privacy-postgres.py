#!/usr/bin/env python3
"""Exercise privacy state on an owned PostgreSQL server, including restart."""

import argparse
import os
import re
import secrets
import subprocess
import sys
import time
import uuid
from pathlib import Path

import mutation_contract


IMAGE = "postgres@sha256:29342cb52157b098821961d2c14eec3c019071f56a5d559e990cf07cf541ea9b"


def ready(name: str) -> None:
    for _ in range(60):
        result = subprocess.run(
            ["docker", "exec", name, "pg_isready", "--host", "127.0.0.1",
             "--username", "postgres", "--dbname", "rullst_privacy_contract"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10,
        )
        if result.returncode == 0:
            return
        time.sleep(0.5)
    raise RuntimeError("Owned PostgreSQL server did not become ready")


def database_url(name: str) -> str:
    binding = subprocess.check_output(
        ["docker", "port", name, "5432/tcp"], text=True, timeout=10,
    ).strip()
    if not re.fullmatch(r"127\.0\.0\.1:[0-9]+", binding):
        raise RuntimeError("Owned PostgreSQL must be bound only to loopback")
    return "postgres://postgres@" + binding + "/rullst_privacy_contract"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--coverage", action="store_true",
                        help="accumulate profiles with cargo llvm-cov --no-report")
    parser.add_argument("--suite", choices=("all", "age", "consent"), default="all")
    parser.add_argument("--manifest-path", type=Path,
                        help="external consent consumer, using only extracted packages")
    parser.add_argument("--mutants", metavar="REGEX",
                        help="mutate both PostgreSQL adapters and run their exercise-phase contracts")
    parser.add_argument("--shard", metavar="K/N", help="with --mutants, test only this cargo-mutants shard")
    args = parser.parse_args()
    if args.manifest_path is not None and (args.suite != "consent" or args.coverage):
        parser.error("external package consumers require --suite consent without --coverage")
    if args.mutants and (args.coverage or args.manifest_path is not None or args.suite != "all"):
        parser.error("mutation runs use both suites in the workspace, without coverage")
    if args.shard and not args.mutants:
        parser.error("--shard needs --mutants")
    name = "rullst-privacy-contract-" + uuid.uuid4().hex
    started = False
    try:
        subprocess.run(
            ["docker", "run", "--detach", "--rm", "--name", name,
             "--env", "POSTGRES_HOST_AUTH_METHOD=trust",
             "--env", "POSTGRES_DB=rullst_privacy_contract",
             "--publish", "127.0.0.1::5432", IMAGE],
            check=True, timeout=120, stdout=subprocess.DEVNULL,
        )
        started = True
        ready(name)
        env = dict(os.environ)
        env.update({
            "RULLST_PRIVACY_POSTGRES_DISPOSABLE": "1",
            "RULLST_PRIVACY_TEST_POSTGRES_URL": database_url(name),
            "RULLST_PRIVACY_RESTART_NONCE": secrets.token_hex(32),
            "RULLST_PRIVACY_POSTGRES_PHASE": "exercise",
        })
        if args.mutants:
            # Exercise phase only; the runner recreates the database, drops the
            # runtime roles and resets ALTER SYSTEM settings per binary.
            return mutation_contract.run(
                package="rullst-privacy", features="postgres,consent-postgres",
                files=["rullst-privacy/src/age_assurance/postgres.rs",
                       "rullst-privacy/src/consent/postgres.rs"],
                targets=["--test=age_assurance", "--test=consent"],
                tests=["postgres::shared_replay_contract", "postgres::shared_consent_contract"],
                regex=args.mutants, shard=args.shard, env=env, timeout=60,
                reset_spec=mutation_contract.postgres(name, "rullst_privacy_contract"))
        commands = []
        for suite, feature, target, contract in [
            ("age", "postgres", "age_assurance", "shared_replay_contract"),
            ("consent", "consent-postgres", "consent", "shared_consent_contract"),
        ]:
            if args.suite not in ("all", suite):
                continue
            command = (["cargo", "llvm-cov", "--no-report"] if args.coverage else ["cargo", "test"])
            if args.manifest_path is None:
                command += ["--locked", "-p", "rullst-privacy",
                            "--no-default-features", "--features", feature]
            else:
                command += ["--locked", "--offline", "--manifest-path", str(args.manifest_path)]
            command += ["--test", target, "--", "--ignored", "--exact",
                        "postgres::" + contract, "--nocapture"]
            commands.append(command)
            subprocess.run(command, env=env, check=True, timeout=1800)
        # Keep the owned container's storage, but interrupt the database process.
        # This is a server-restart contract, not a physical power-loss test.
        subprocess.run(["docker", "restart", "--time", "0", name],
                       check=True, timeout=60, stdout=subprocess.DEVNULL)
        ready(name)
        # Docker can reassign an ephemeral host port when restarting a container.
        env["RULLST_PRIVACY_TEST_POSTGRES_URL"] = database_url(name)
        env["RULLST_PRIVACY_POSTGRES_PHASE"] = "restart"
        for command in commands:
            subprocess.run(command, env=env, check=True, timeout=1800)
        return 0
    except (subprocess.SubprocessError, RuntimeError):
        if started:
            subprocess.run(["docker", "logs", "--tail", "60", name],
                           check=False, timeout=10)
        raise
    finally:
        if started:
            subprocess.run(["docker", "stop", "--time", "5", name],
                           check=True, timeout=30, stdout=subprocess.DEVNULL)


if __name__ == "__main__":
    sys.exit(main())
