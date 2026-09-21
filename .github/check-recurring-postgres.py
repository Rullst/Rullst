#!/usr/bin/env python3
"""Exercise recurring on an owned disposable PostgreSQL, including restart."""
import argparse
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time
import uuid

IMAGE = "postgres@sha256:29342cb52157b098821961d2c14eec3c019071f56a5d559e990cf07cf541ea9b"


def ready(name):
    for _ in range(40):
        result = subprocess.run(["docker", "exec", name, "pg_isready", "--host", "127.0.0.1",
                                 "--username", "postgres", "--dbname", "rullst_recurring_contract"],
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10)
        if result.returncode == 0:
            return
        time.sleep(0.5)
    raise RuntimeError("Owned recurring database did not become ready")


def url(name):
    binding = subprocess.check_output(["docker", "port", name, "5432/tcp"], text=True, timeout=10).strip()
    if not re.fullmatch(r"127\.0\.0\.1:[0-9]+", binding):
        raise RuntimeError("PostgreSQL port must be loopback-only")
    return "postgres://postgres@" + binding + "/rullst_recurring_contract"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--coverage", action="store_true")
    parser.add_argument("--manifest-path", type=Path)
    args = parser.parse_args()
    if args.manifest_path and args.coverage:
        parser.error("archive consumers do not support coverage")
    name = "rullst-recurring-" + uuid.uuid4().hex
    started = False
    try:
        subprocess.run(["docker", "run", "--detach", "--rm", "--name", name,
                        "--env", "POSTGRES_HOST_AUTH_METHOD=trust", "--env", "POSTGRES_DB=rullst_recurring_contract",
                        "--publish", "127.0.0.1::5432", IMAGE], check=True, timeout=120)
        started = True
        ready(name)
        env = dict(os.environ, RULLST_RECURRING_TEST_POSTGRES_URL=url(name))
        command = ["cargo", "llvm-cov", "--no-report"] if args.coverage else ["cargo", "test"]
        command += ["--locked"]
        if args.manifest_path:
            command += ["--manifest-path", str(args.manifest_path.resolve()), "--offline"]
        else:
            command += ["-p", "rullst-messaging", "--no-default-features", "--features", "schedules-postgres,sqlite"]
        subprocess.run(command + ["--test", "recurring_contract", "--", "--ignored", "--exact",
                                  "postgres_recurring_contract"], env=env, check=True, timeout=300)
        with tempfile.TemporaryDirectory(prefix="rullst-recurring-restart-") as directory:
            receipt = Path(directory, "receipt.json")
            receipt.touch(mode=0o600)
            env.update(RULLST_RECURRING_RESTART_RECEIPT=str(receipt), RULLST_RECURRING_RESTART_PHASE="exercise")
            journey = command + ["--test", "recurring_restart", "--", "--ignored", "--exact",
                                 "postgres_recurring_process_restart"]
            subprocess.run(journey, env=env, check=True, timeout=300)
            subprocess.run(["docker", "restart", "--timeout", "0", name], check=True, timeout=60)
            ready(name)
            env.update(RULLST_RECURRING_TEST_POSTGRES_URL=url(name), RULLST_RECURRING_RESTART_PHASE="restart")
            subprocess.run(journey, env=env, check=True, timeout=300)
    finally:
        if started:
            subprocess.run(["docker", "stop", "--timeout", "5", name], check=True, timeout=30)


if __name__ == "__main__":
    main()
