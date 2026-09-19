#!/usr/bin/env python3
"""Run the atomic account-recovery contract against an owned disposable database."""
import os
import subprocess
import time
import uuid

IMAGE = "postgres@sha256:29342cb52157b098821961d2c14eec3c019071f56a5d559e990cf07cf541ea9b"
name = "rullst-auth-recovery-" + uuid.uuid4().hex
started = False
try:
    subprocess.run(["docker", "run", "--detach", "--rm", "--name", name,
                    "--env", "POSTGRES_HOST_AUTH_METHOD=trust",
                    "--env", "POSTGRES_DB=rullst_recovery_contract",
                    "--publish", "127.0.0.1::5432", IMAGE], check=True)
    started = True
    for attempt in range(40):
        if subprocess.run(["docker", "exec", name, "pg_isready", "--username", "postgres"],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
            break
        time.sleep(0.5)
    else:
        raise RuntimeError("Disposable PostgreSQL did not become ready")
    binding = subprocess.check_output(["docker", "port", name, "5432/tcp"], text=True).strip()
    if not binding.startswith("127.0.0.1:") or not binding.rsplit(":", 1)[1].isdigit():
        raise RuntimeError("PostgreSQL port must be loopback-only")
    env = dict(os.environ)
    env["RULLST_RECOVERY_TEST_POSTGRES_URL"] = (
        "postgres://postgres@" + binding + "/rullst_recovery_contract")
    subprocess.run(["cargo", "test", "--locked", "-p", "rullst-auth",
                    "--no-default-features", "--features", "recovery-postgres",
                    "--test", "recovery_contract", "postgres_recovery_contract",
                    "--", "--ignored", "--exact"], env=env, check=True)
finally:
    if started:
        subprocess.run(["docker", "stop", "--time", "5", name], check=True)
