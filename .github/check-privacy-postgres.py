#!/usr/bin/env python3
"""Exercise age replay state on an owned PostgreSQL server, including restart."""

import argparse
import os
import re
import secrets
import subprocess
import time
import uuid


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


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--coverage", action="store_true",
                        help="accumulate profiles with cargo llvm-cov --no-report")
    args = parser.parse_args()
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
        command = (["cargo", "llvm-cov", "--no-report"] if args.coverage else ["cargo", "test"])
        command += ["--locked", "-p", "rullst-privacy",
                   "--no-default-features", "--features", "postgres",
                   "--test", "age_assurance", "--", "--ignored", "--exact",
                   "postgres::shared_replay_contract", "--nocapture"]
        subprocess.run(command, env=env, check=True, timeout=1800)
        # Keep the owned container's storage, but interrupt the database process.
        # This is a server-restart contract, not a physical power-loss test.
        subprocess.run(["docker", "restart", "--time", "0", name],
                       check=True, timeout=60, stdout=subprocess.DEVNULL)
        ready(name)
        # Docker can reassign an ephemeral host port when restarting a container.
        env["RULLST_PRIVACY_TEST_POSTGRES_URL"] = database_url(name)
        env["RULLST_PRIVACY_POSTGRES_PHASE"] = "restart"
        subprocess.run(command, env=env, check=True, timeout=1800)
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
    main()
