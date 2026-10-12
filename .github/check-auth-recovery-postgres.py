#!/usr/bin/env python3
"""Run account recovery and session contracts on an owned disposable PostgreSQL."""
import argparse
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
import uuid

import mutation_contract

IMAGE = "postgres@sha256:29342cb52157b098821961d2c14eec3c019071f56a5d559e990cf07cf541ea9b"


def ready(name):
    for _ in range(40):
        if subprocess.run(["docker", "exec", name, "pg_isready", "--host", "127.0.0.1",
                           "--username", "postgres", "--dbname", "rullst_recovery_contract"],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                          timeout=10).returncode == 0:
            return
        time.sleep(0.5)
    raise RuntimeError("Disposable PostgreSQL did not become ready")


def url(name):
    binding = subprocess.check_output(["docker", "port", name, "5432/tcp"],
                                      text=True, timeout=10).strip()
    if not re.fullmatch(r"127\.0\.0\.1:[0-9]+", binding):
        raise RuntimeError("PostgreSQL port must be loopback-only")
    return "postgres://postgres@" + binding + "/rullst_recovery_contract"


# Files with PostgreSQL-only branches; the shared SQL is also tested on SQLite.
MUTATED_FILES = [
    "rullst-auth/src/recovery/connection.rs",
    "rullst-auth/src/recovery/store.rs",
    "rullst-auth/src/recovery/email_login/guard.rs",
    "rullst-auth/src/recovery/email_login/storage.rs",
    "rullst-auth/src/recovery/api_tokens/storage.rs",
]
# Every contract below except the browser journey, in its exercise phase.
CONTRACT_TARGETS = ["--lib", "--test=recovery_contract", "--test=email_login_contract",
                    "--test=api_token_contract", "--test=api_token_http", "--test=session_process",
                    "--test=email_login_restart", "--test=api_token_restart"]
CONTRACT_TESTS = [
    "postgres_recovery_contract",
    "recovery::sessions::tests::postgres_inventory_and_revocation_are_account_scoped",
    "recovery::sessions::tests::postgres::contention_and_locked_database_preserve_revocation_and_deadlines",
    "postgres_login_contract",
    "postgres_api_token_contract",
    "postgres_api_tokens_http_authorization_and_revocation",
    "postgres_requests_observe_revocation_across_processes_and_restart",
    "postgres_email_login_process_restart",
    "postgres_api_token_process_restart",
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--coverage", action="store_true")
    parser.add_argument("--suite", choices=["all", "email-login", "api-tokens"], default="all")
    parser.add_argument("--manifest-path", type=Path)
    parser.add_argument("--mutants", metavar="REGEX",
                        help="mutate the PostgreSQL branches and run the exercise-phase contracts")
    parser.add_argument("--shard", metavar="K/N", help="with --mutants, test only this cargo-mutants shard")
    args = parser.parse_args()
    if args.manifest_path and (args.suite == "all" or args.coverage):
        parser.error("archive consumers require a specific suite without coverage")
    if args.mutants and (args.coverage or args.manifest_path or args.suite != "all"):
        parser.error("mutation runs use every suite in the workspace, without coverage")
    if args.shard and not args.mutants:
        parser.error("--shard needs --mutants")
    email_login = args.suite in ("all", "email-login")
    api_tokens = args.suite in ("all", "api-tokens")
    name = "rullst-auth-recovery-" + uuid.uuid4().hex
    started = False
    try:
        subprocess.run(["docker", "run", "--detach", "--rm", "--name", name,
                        "--env", "POSTGRES_HOST_AUTH_METHOD=trust",
                        "--env", "POSTGRES_DB=rullst_recovery_contract",
                        "--publish", "127.0.0.1::5432", IMAGE], check=True, timeout=120)
        started = True
        ready(name)
        env = dict(os.environ, RULLST_RECOVERY_TEST_POSTGRES_URL=url(name))
        if args.mutants:
            with tempfile.TemporaryDirectory(prefix="rullst-session-mutants-") as temporary:
                # Exercise phase only: it rewrites each receipt, and the runner
                # recreates the database and drops the runtime roles per binary.
                for variable, file in [("RULLST_SESSION_RESTART_RECEIPT", "receipt.json"),
                                       ("RULLST_EMAIL_LOGIN_RESTART_RECEIPT", "email-login.json"),
                                       ("RULLST_API_TOKEN_RESTART_RECEIPT", "api-tokens.json")]:
                    receipt = Path(temporary, file)
                    receipt.touch(mode=0o600)
                    env[variable] = str(receipt)
                env["RULLST_SESSION_RESTART_PHASE"] = "exercise"
                return mutation_contract.run(
                    package="rullst-auth",
                    features="recovery-postgres,email-login-postgres,api-tokens-postgres",
                    files=MUTATED_FILES, targets=CONTRACT_TARGETS, tests=CONTRACT_TESTS,
                    regex=args.mutants, shard=args.shard, env=env, timeout=240,
                    reset_spec=mutation_contract.postgres(name, "rullst_recovery_contract"))
        command = ["cargo", "llvm-cov", "--no-report"] if args.coverage else ["cargo", "test"]
        command += ["--locked"]
        if args.manifest_path:
            command += ["--manifest-path", str(args.manifest_path.resolve()), "--offline"]
        else:
            command += ["-p", "rullst-auth", "--no-default-features",
                        "--features", "recovery-postgres,email-login-postgres,api-tokens-postgres"]
        if args.suite == "all":
            subprocess.run(command + ["--test", "recovery_contract", "--", "--ignored", "--exact",
                                      "postgres_recovery_contract"], env=env, check=True, timeout=1800)
            subprocess.run(command + ["--lib", "--", "--ignored", "--exact",
                                      "recovery::sessions::tests::postgres_inventory_and_revocation_are_account_scoped"],
                           env=env, check=True, timeout=1800)
            subprocess.run(command + ["--lib", "--", "--ignored", "--exact",
                                      "recovery::sessions::tests::postgres::contention_and_locked_database_preserve_revocation_and_deadlines"],
                           env=env, check=True, timeout=1800)
        if email_login:
            subprocess.run(command + ["--test", "email_login_contract", "--", "--ignored", "--exact",
                                      "postgres_login_contract"], env=env, check=True, timeout=1800)
        if email_login and os.environ.get("RULLST_EMAIL_LOGIN_BROWSER_TESTS") == "1":
            subprocess.run(command + ["--test", "email_login_http", "--", "--ignored", "--exact",
                                      "postgres_email_login_browser", "--nocapture"], env=env, check=True, timeout=180)
        if api_tokens:
            subprocess.run(command + ["--test", "api_token_contract", "--", "--ignored", "--exact",
                                      "postgres_api_token_contract"], env=env, check=True, timeout=1800)
            subprocess.run(command + ["--test", "api_token_http", "--", "--ignored", "--exact",
                                      "postgres_api_tokens_http_authorization_and_revocation"], env=env, check=True, timeout=180)
        with tempfile.TemporaryDirectory(prefix="rullst-session-restart-") as temporary:
            receipt = Path(temporary, "receipt.json")
            receipt.touch(mode=0o600)
            login_receipt = Path(temporary, "email-login.json")
            login_receipt.touch(mode=0o600)
            api_receipt = Path(temporary, "api-tokens.json")
            api_receipt.touch(mode=0o600)
            env.update(RULLST_EMAIL_LOGIN_RESTART_RECEIPT=str(login_receipt),
                       RULLST_API_TOKEN_RESTART_RECEIPT=str(api_receipt),
                       RULLST_SESSION_RESTART_RECEIPT=str(receipt),
                       RULLST_SESSION_RESTART_PHASE="exercise")
            journey = command + ["--test", "session_process", "--", "--ignored", "--exact",
                                 "postgres_requests_observe_revocation_across_processes_and_restart"]
            login_journey = command + ["--test", "email_login_restart", "--", "--ignored", "--exact",
                                       "postgres_email_login_process_restart"]
            api_journey = command + ["--test", "api_token_restart", "--", "--ignored", "--exact",
                                     "postgres_api_token_process_restart"]
            if args.suite == "all":
                subprocess.run(journey, env=env, check=True, timeout=1800)
            if email_login:
                subprocess.run(login_journey, env=env, check=True, timeout=1800)
            if api_tokens:
                subprocess.run(api_journey, env=env, check=True, timeout=1800)
            subprocess.run(["docker", "restart", "--time", "0", name], check=True, timeout=60)
            ready(name)
            env.update(RULLST_RECOVERY_TEST_POSTGRES_URL=url(name),
                       RULLST_SESSION_RESTART_PHASE="restart")
            if args.suite == "all":
                subprocess.run(journey, env=env, check=True, timeout=1800)
            if email_login:
                subprocess.run(login_journey, env=env, check=True, timeout=1800)
            if api_tokens:
                subprocess.run(api_journey, env=env, check=True, timeout=1800)
        return 0
    finally:
        if started:
            subprocess.run(["docker", "stop", "--time", "5", name], check=True, timeout=30)


if __name__ == "__main__":
    sys.exit(main())
