#!/usr/bin/env python3
"""Exercise the remote Messaging contract against an owned AOF Redis process."""
import argparse
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time
import uuid

IMAGE = "redis:7.4-alpine@sha256:ff02b58f971e7d7d156a1267e283fcbbeee91773b6aa36c49dac28ecfe28eadf"
PASSWORD = "fixture-redis-messaging-only"


def ready(name):
    for _ in range(40):
        result = subprocess.run(["docker", "exec", "--env", "REDISCLI_AUTH=" + PASSWORD,
                                 name, "redis-cli", "ping"], capture_output=True,
                                text=True, timeout=10)
        if result.returncode == 0 and result.stdout.strip() == "PONG":
            return
        time.sleep(0.25)
    raise RuntimeError("Owned Redis did not become ready")


def endpoint(name, tls=False):
    binding = subprocess.check_output(["docker", "port", name, "6380/tcp" if tls else "6379/tcp"],
                                      text=True, timeout=10).strip()
    if not re.fullmatch(r"127\.0\.0\.1:[0-9]+", binding):
        raise RuntimeError("Owned Redis must expose only loopback")
    return ("rediss://" if tls else "redis://") + binding


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--coverage", action="store_true")
    args = parser.parse_args()
    name = "rullst-messaging-" + uuid.uuid4().hex
    volume = name + "-data"
    created = False
    certificates = tempfile.TemporaryDirectory(prefix="rullst-redis-certificates-")
    certificate_dir = Path(certificates.name, "fixture")
    certificate_dir.mkdir(mode=0o755)
    try:
        cert_commands = [
            ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes",
             "-keyout", "ca-key.pem", "-out", "ca.pem", "-days", "1", "-subj", "/CN=Disposable Fixture CA"],
            ["openssl", "req", "-new", "-newkey", "rsa:2048", "-nodes",
             "-keyout", "key.pem", "-out", "request.pem", "-subj", "/CN=localhost"],
            ["openssl", "x509", "-req", "-in", "request.pem", "-CA", "ca.pem",
             "-CAkey", "ca-key.pem", "-CAcreateserial", "-out", "cert.pem",
             "-days", "1", "-extfile", "extensions.cnf"],
        ]
        (certificate_dir / "extensions.cnf").write_text(
            "basicConstraints=critical,CA:FALSE\nsubjectAltName=DNS:localhost\nextendedKeyUsage=serverAuth\n")
        for certificate_command in cert_commands:
            subprocess.run(certificate_command, cwd=certificate_dir, check=True,
                           timeout=30, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        # Public disposable fixture identity, readable by the unprivileged service.
        (certificate_dir / "key.pem").chmod(0o644)
        subprocess.run(["docker", "volume", "create", volume], check=True, timeout=30)
        created = True
        subprocess.run(["docker", "run", "--detach", "--name", name,
                        "--user", "999:999", "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
                        "--read-only", "--mount", "type=volume,source=" + volume + ",target=/data",
                        "--mount", "type=bind,source=" + str(certificate_dir) + ",target=/certs,readonly",
                        "--memory", "384m", "--cpus", "1", "--pids-limit", "64",
                        "--publish", "127.0.0.1::6379", "--publish", "127.0.0.1::6380", IMAGE,
                        "redis-server", "--requirepass", PASSWORD, "--appendonly", "yes",
                        "--appendfsync", "always", "--maxmemory", "256mb",
                        "--tls-port", "6380", "--tls-cert-file", "/certs/cert.pem",
                        "--tls-key-file", "/certs/key.pem", "--tls-ca-cert-file", "/certs/ca.pem",
                        "--tls-auth-clients", "no",
                        "--maxmemory-policy", "noeviction"], check=True, timeout=120)
        ready(name)
        env = dict(os.environ, RULLST_MESSAGING_TEST_REDIS_URL=endpoint(name),
                   RULLST_MESSAGING_TEST_REDIS_TLS_URL=endpoint(name, True),
                   RULLST_MESSAGING_TEST_REDIS_CA=str(certificate_dir / "ca.pem"))
        command = ["cargo", "llvm-cov", "--no-report"] if args.coverage else ["cargo", "test"]
        command += ["--locked", "-p", "rullst-messaging", "--features", "redis-streams,orm-outbox",
                    "--test", "redis_streams"]
        for test in ["actual_redis_contract_concurrency_and_fenced_redelivery",
                     "actual_redis_latest_retry_purge_and_bounded_capacity",
                     "faults::missing_state_and_partial_mutations_fail_closed",
                     "faults::wrong_credentials_timeout_and_clock_regression_never_fall_back",
                     "tls::verified_tls_rejects_untrusted_roots_and_wrong_hostnames",
                     "outbox::remote_outbox_replay_converges_after_publish_before_ack",
                     "bounds::byte_retention_reply_batches_and_expired_attempts_are_bounded"]:
            subprocess.run(command + ["--", "--ignored", "--exact", test],
                           env=env, check=True, timeout=1800)
        with tempfile.TemporaryDirectory(prefix="rullst-redis-restart-") as temporary:
            receipt = Path(temporary, "receipt")
            receipt.touch(mode=0o600)
            env.update(RULLST_REDIS_RESTART_RECEIPT=str(receipt),
                       RULLST_REDIS_RESTART_PHASE="exercise")
            journey = command + ["--", "--ignored", "--exact",
                                 "actual_redis_restart_retains_receipts_groups_and_unacked_messages"]
            subprocess.run(journey, env=env, check=True, timeout=1800)
            # Actual abrupt server restart, retaining only this fixture's AOF.
            subprocess.run(["docker", "restart", "--time", "0", name], check=True, timeout=60)
            ready(name)
            env.update(RULLST_MESSAGING_TEST_REDIS_URL=endpoint(name),
                       RULLST_REDIS_RESTART_PHASE="restart")
            subprocess.run(journey, env=env, check=True, timeout=1800)
    finally:
        exists = subprocess.run(["docker", "container", "inspect", name],
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                timeout=10).returncode == 0
        if exists:
            subprocess.run(["docker", "rm", "--force", name], check=True, timeout=30)
        if created:
            subprocess.run(["docker", "volume", "rm", volume], check=True, timeout=30)
        certificates.cleanup()


if __name__ == "__main__":
    main()
