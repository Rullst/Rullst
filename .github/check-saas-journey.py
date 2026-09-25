#!/usr/bin/env python3
"""Source-installed CLI -> generated SQLite SaaS -> real HTTP -> process restart.

Manual, Linux-only diagnostic. This is neither release admission nor a benchmark.
The application receives only disposable configuration and no provider credentials.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import socket
import sqlite3
import subprocess
import tempfile
import time
import tomllib

from saas_journey_http import Client, exercise

ROOT = Path(__file__).resolve().parent.parent
GIB = 1024 ** 3


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def disk_check(paths, minimum):
    for path in paths:
        if shutil.disk_usage(path).free < minimum * GIB:
            raise RuntimeError(f"Journey requires at least {minimum} GiB available on {path}")


def stop(process):
    # Each owned child starts a new session. Do not signal other developer jobs.
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait(timeout=10)


class Journey:
    def __init__(self, work, report):
        self.work, self.report = work, report
        self.app = work / "pilot-saas"
        self.server = None
        self.server_log = None
        # Preserve toolchain discovery, not the user's application/provider environment.
        names = ("PATH", "HOME", "USER", "LOGNAME", "LANG", "LC_ALL", "RUSTUP_HOME",
                 "CARGO_HOME", "RUSTUP_TOOLCHAIN", "LD_LIBRARY_PATH", "PKG_CONFIG_PATH",
                 "CC", "CXX", "AR", "LIBCLANG_PATH", "SSL_CERT_FILE", "SSL_CERT_DIR")
        self.env = {key: os.environ[key] for key in names if key in os.environ}
        self.env.update(CARGO_TARGET_DIR=str(work / "target"), CARGO_NET_OFFLINE="true",
                        CARGO_BUILD_JOBS="2", CARGO_INCREMENTAL="0", CARGO_PROFILE_DEV_DEBUG="0",
                        CARGO_PROFILE_TEST_DEBUG="0", CARGO_TERM_COLOR="never", RUSTFLAGS="-D warnings")
        self.build_env = dict(self.env)
        for key in ("RUSTC_WRAPPER", "SCCACHE_GHA_ENABLED", "SCCACHE_GHA_VERSION",
                    "SCCACHE_GHA_RW_MODE", "ACTIONS_RESULTS_URL", "ACTIONS_RUNTIME_TOKEN"):
            if key in os.environ:
                self.build_env[key] = os.environ[key]
        self.runtime_env = dict(self.env)

    def sanitized_log(self, path):
        text = path.read_text(errors="replace")[-12000:]
        # Do not upload generated .env, database, raw output, cookies or runtime keys.
        env_path = self.app / ".env"
        if env_path.exists():
            for line in env_path.read_text().splitlines():
                if "=" in line and not line.startswith("#"):
                    key, value = line.split("=", 1)
                    if value and any(word in key for word in ("KEY", "TOKEN", "PASSWORD", "SECRET")):
                        text = text.replace(value, "[REDACTED]")
        for key, value in self.build_env.items():
            if value and ("TOKEN" in key or "SECRET" in key):
                text = text.replace(value, "[REDACTED]")
        return text

    def run(self, label, command, *, cwd=None, runtime=False, timeout=1800):
        disk_check((ROOT, self.work), 15)
        start = time.monotonic()
        phase = {"name": label, "passed": False}
        self.report["phases"].append(phase)
        print("Journey phase: " + label, flush=True)
        log = self.work / (label + ".log")
        with log.open("w") as output:
            process = subprocess.Popen(command, cwd=cwd or ROOT,
                                       env=self.runtime_env if runtime else self.build_env,
                                       stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                while process.poll() is None:
                    disk_check((ROOT, self.work), 12)
                    if time.monotonic() - start > timeout:
                        raise TimeoutError("Journey phase timed out: " + label)
                    time.sleep(2)
                if process.returncode != 0:
                    raise RuntimeError(f"Journey phase {label} exited {process.returncode}")
                phase["passed"] = True
            except BaseException:
                stop(process)
                print(self.sanitized_log(log), flush=True)
                raise
            finally:
                phase["seconds"] = round(time.monotonic() - start, 3)
        return log.read_text()

    def prepare(self):
        self.run("fetch", ["cargo", "--config", "net.offline=false", "fetch", "--locked"])
        prefix = self.work / "installed"
        self.run("install-cli", ["cargo", "install", "--path", str(ROOT / "cargo-rullst"),
                 "--locked", "--offline", "--debug", "--root", str(prefix), "--bin", "rullst"])
        self.cli = prefix / "bin/rullst"
        version = self.run("cli-version", [str(self.cli), "--version"], runtime=True).strip()
        expected = tomllib.loads((ROOT / "cargo-rullst/Cargo.toml").read_text())["package"]["version"]
        if expected not in version.split():
            raise RuntimeError("Installed CLI version does not match the candidate")
        self.report.update(cli_version=version, cli_sha256=digest(self.cli))
        self.run("generate", [str(self.cli), "new", "pilot-saas", "--default", "--blueprint", "saas",
                 "--database", "sqlite", "--skip-initial-migration"], cwd=self.work, runtime=True)
        manifest = tomllib.loads((self.app / "Cargo.toml").read_text())
        # A source-installed prerelease CLI must bind every generated local framework
        # dependency to this checkout, not an old registry version or another worktree.
        for name, dependency in manifest["dependencies"].items():
            if name == "rullst" or name.startswith("rullst-"):
                if not isinstance(dependency, dict) or Path(dependency.get("path", "")).resolve() != ROOT / name:
                    raise RuntimeError("Generated dependency is not bound to reviewed source: " + name)
        shutil.copyfile(ROOT / "Cargo.lock", self.app / "Cargo.lock")
        self.database = self.app / "journey.sqlite"
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        self.base = "http://127.0.0.1:" + str(port)
        env_path = self.app / ".env"
        values = dict(line.split("=", 1) for line in env_path.read_text().splitlines()
                      if "=" in line and not line.startswith("#"))
        values.update(DATABASE_URL="sqlite://" + str(self.database) + "?mode=rwc",
                      HOST="127.0.0.1", PORT=str(port), RULLST_ENV="development",
                      BILLING_PROVIDER="stripe", BILLING_API_KEY="", BILLING_ACCOUNT_ID="",
                      BILLING_WEBHOOK_SECRET="", BILLING_REDIRECT_URL=self.base + "/dashboard")
        env_path.write_text("".join(key + "=" + value + "\n" for key, value in values.items()))
        env_path.chmod(0o600)
        self.runtime_env.update(values)
        self.run("scaffold-migration", [str(self.cli), "make:migration", "create_journey_tables"],
                 cwd=self.app, runtime=True)
        migrations = list((self.app / "src/migrations").glob("*_create_journey_tables.rs"))
        if len(migrations) != 1:
            raise RuntimeError("CLI did not create exactly one journey migration")
        migration = ROOT / ".github/fixtures/saas-journey-migration.rs"
        migrations[0].write_text(migration.read_text().replace("__JOURNEY_MIGRATION_NAME__", migrations[0].stem))
        fixture = ROOT / ".github/fixtures/saas-journey.rs"
        shutil.copyfile(fixture, self.app / "src/journey.rs")
        main = self.app / "src/main.rs"
        source = main.read_text()
        marker = "    Server::new(router)"
        if source.count(marker) != 1:
            raise RuntimeError("Generated main changed; review application router composition")
        main.write_text("mod journey;\n" + source.replace(marker,
                        "    let router = router.merge_axum(journey::routes().into_axum());\n" + marker))
        self.report["application_changes"] = {
            "fixture_lines": len(fixture.read_text().splitlines()),
            "migration_lines": len(migration.read_text().splitlines()),
            "main_added_lines": 2,
            "configuration": "disposable .env; copied workspace lock; explicit membership operator",
        }
        # db:migrate invokes the actual generated binary through cargo run.
        self.run("migrate", [str(self.cli), "db:migrate"], cwd=self.app, runtime=True)
        self.run("migrate-again", [str(self.cli), "db:migrate"], cwd=self.app, runtime=True, timeout=120)
        self.run("generated-clippy", ["cargo", "clippy", "--offline", "--locked", "--all-targets",
                 "--", "-D", "warnings"], cwd=self.app)
        self.report["application_lock_sha256"] = digest(self.app / "Cargo.lock")

    def start(self):
        start = time.monotonic()
        self.server_log = (self.work / "server.log").open("a")
        self.server = subprocess.Popen([str(self.work / "target/debug/pilot-saas")], cwd=self.app,
                                      env=self.runtime_env, stdout=self.server_log,
                                      stderr=subprocess.STDOUT, start_new_session=True)
        client = Client(self.base)
        while time.monotonic() - start < 45:
            if self.server.poll() is not None:
                raise RuntimeError("Generated server exited before readiness")
            try:
                if client.request("GET", "/login")[0] == 200:
                    self.report.setdefault("startup_seconds", []).append(round(time.monotonic() - start, 3))
                    return
            except (OSError, TimeoutError):
                pass
            time.sleep(0.25)
        raise RuntimeError("Generated server was not ready within 45 seconds")

    def shutdown(self):
        if self.server is not None:
            stop(self.server)
            self.server = None
        if self.server_log is not None:
            self.server_log.close()
            self.server_log = None

    def restart(self):
        self.shutdown()
        self.start()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "saas-journey-result.json")
    args = parser.parse_args()
    report = {"schema": "rullst.saas-journey.v1", "passed": False, "cases": [], "phases": [],
              "profile": "Linux / SQLite / source-installed debug CLI / generated SaaS / loopback HTTP",
              "release_evidence": False, "live_provider_evidence": False,
              "platform": platform.platform(), "sqlite_python_version": sqlite3.sqlite_version,
              "limitations": ["No archive/registry installation, browser/TLS, load or competing framework trial",
                              "No billing, recovery, release upgrade or schema rollback journey",
                              "Application supplies membership, SQL scoping and resource authorization",
                              "Wall times include build/cache noise; no human productivity ranking"]}
    try:
        if platform.system() != "Linux":
            raise RuntimeError("This diagnostic is admitted only for Linux")
        # A cold build is not a small job. Leave >=12 GiB, with a conservative
        # 20 GiB initial build allowance. Monitor owned builds and stop at reserve.
        disk_check((ROOT, Path(tempfile.gettempdir())), 32)
        report["source_sha"] = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        report["workspace_lock_sha256"] = digest(ROOT / "Cargo.lock")
        report["rustc"] = subprocess.check_output(["rustc", "-Vv"], text=True).strip()
        with tempfile.TemporaryDirectory(prefix="rullst-saas-journey-") as directory:
            journey = Journey(Path(directory), report)
            try:
                journey.prepare()
                journey.start()
                start = time.monotonic()
                exercise(journey.base, journey.database, journey.restart, report)
                report["http_seconds"] = round(time.monotonic() - start, 3)
                report["passed"] = True
            finally:
                journey.shutdown()
                log = journey.work / "server.log"
                if not report["passed"] and log.exists():
                    print(journey.sanitized_log(log), flush=True)
    except Exception as error:
        report["error"] = str(error)
        raise
    finally:
        args.output.write_text(json.dumps(report, indent=2) + "\n")
        print(f"SaaS journey: passed={report['passed']}, cases={len(report['cases'])}", flush=True)


if __name__ == "__main__":
    main()
