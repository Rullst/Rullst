#!/usr/bin/env python3
"""Start the files `cargo rullst deploy` writes, on a real engine (Linux CI only).

Installs the CLI from the packaged archives; for the Blank and SaaS starters,
runs `cargo rullst deploy`, copies the project as docs/src/vm-deployment.md
(step 4A) does, builds the generated Dockerfile and starts compose.yaml and the
Caddyfile with Docker or rootless Podman. Checks /health over HTTPS through
Caddy (its local CA), the HTTP redirect, the unpublished app port, the
hardening, a migrated SQLite database (SaaS: an account) in its volume across
`compose restart`, and the logs. The generated deployment files are never
modified: the unpublished release crates enter the build context as packaged
archives through `[patch.crates-io]`, as in .github/test-packaged-distribution.sh.
"""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import socket
import sqlite3
import subprocess
import sys
import tarfile
import time
import tomllib

from vm_deploy_http import Failure, check_application, check_https, expect, log

ROOT = Path(__file__).resolve().parent.parent
STARTERS = ("blank", "saas")
CADDY_ADDRESS = "172.31.250.10"
# Already pulled by compose.yaml; used read-only to list the data volume.
CADDY_IMAGE = "docker.io/library/caddy:2-alpine"
NET_BIND_SERVICE = 1 << 10
GENERATED = ("Dockerfile", ".dockerignore", "compose.yaml", "Caddyfile")
CLI_PACKAGES = ("cargo-rullst", "cargo-rullst-extras")
PATCH_DIRECTORY = "rullst-ci-packages"
DATABASE_SUFFIXES = (".db", ".sqlite", ".sqlite3")
TIMINGS: list[tuple[str, float]] = []


@contextlib.contextmanager
def step(title: str):
    """Collapses a step's output in the Actions log and records its duration."""
    log(f"::group::{title}")
    started = time.monotonic()
    try:
        yield
    finally:
        TIMINGS.append((title, time.monotonic() - started))
        log("::endgroup::")


def run(command: list[str], *, cwd: Path | None = None, env: dict[str, str] | None = None,
        timeout: int = 600, capture: bool = False, check: bool = True,
        quiet: bool = False) -> subprocess.CompletedProcess:
    if not quiet:
        log("$ " + " ".join(command))
    result = subprocess.run(
        command, cwd=cwd, env=env, timeout=timeout, text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.STDOUT if capture else None,
    )
    if check and result.returncode != 0:
        if capture:
            log(result.stdout)
        raise Failure(f"`{' '.join(command[:3])}` exited with {result.returncode}")
    return result


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class Engine:
    """`docker` or rootless `podman`, with the compose command the guide uses."""

    def __init__(self, name: str):
        self.name = name
        self.project = ""
        self.env = dict(os.environ)
        if name == "docker":
            self.env["BUILDKIT_PROGRESS"] = "plain"
        else:
            # Debian and Ubuntu: `apt install podman podman-compose` (VM guide).
            self.env["PODMAN_COMPOSE_PROVIDER"] = "podman-compose"

    def __call__(self, *args: str, **options) -> subprocess.CompletedProcess:
        return run([self.name, *args], env=self.env, **options)

    def output(self, *args: str, **options) -> str:
        return self(*args, capture=True, **options).stdout.strip()

    def compose(self, project: Path, *args: str, **options) -> subprocess.CompletedProcess:
        return run([self.name, "compose", *args], cwd=project, env=self.env, **options)

    def inspect(self, container: str) -> dict:
        return json.loads(self.output("inspect", container, quiet=True))[0]

    def containers(self, service: str | None = None) -> list[str]:
        filters = ["--filter", f"label=com.docker.compose.project={self.project}"]
        if service:
            filters += ["--filter", f"label=com.docker.compose.service={service}"]
        return self.output("ps", "-aq", *filters, quiet=True).split()


def extract_archives(packages: Path, version: str, work: Path) -> tuple[Path, list[str]]:
    order = json.loads((ROOT / ".github/release-order.json").read_text())
    extracted = work / "packages"
    extracted.mkdir(parents=True)
    for crate in order:
        archive = packages / f"{crate}-{version}.crate"
        if not archive.is_file():
            raise Failure(f"missing package archive {archive}")
        with tarfile.open(archive) as handle:
            handle.extractall(extracted, filter="data")
    return extracted, [crate for crate in order if crate not in CLI_PACKAGES]


def patch_section(paths: dict[str, str]) -> str:
    lines = ["", "# Added by .github/check-vm-deploy.py: the release crates are not on",
             "# crates.io before publication, so these packaged archives replace them.",
             "[patch.crates-io]"]
    lines += [f'"{crate}" = {{ path = {json.dumps(path)} }}' for crate, path in paths.items()]
    return "\n".join(lines) + "\n"


def install_cli(extracted: Path, crates: list[str], version: str, work: Path) -> dict[str, str]:
    """`cargo install` from the archive, as users get it; it writes registry pins."""
    package = extracted / f"cargo-rullst-{version}"
    manifest = package / "Cargo.toml"
    paths = {crate: str(extracted / f"{crate}-{version}") for crate in crates}
    manifest.write_text(manifest.read_text() + patch_section(paths))
    root = work / "install"
    run(["cargo", "install", "--path", str(package), "--root", str(root), "--offline",
         "--locked", "--debug", "--force"], timeout=45 * 60)
    env = dict(os.environ)
    env["PATH"] = f"{root / 'bin'}{os.pathsep}{env['PATH']}"
    env["RULLST_DISABLE_UPDATE_CHECK"] = "true"
    run(["cargo", "rullst", "--version"], env=env)
    return env


def create_project(starter: str, env: dict[str, str], version: str,
                   work: Path) -> tuple[Path, dict[str, str]]:
    projects = work / "projects"
    projects.mkdir(exist_ok=True)
    name = f"deploy-{starter}"
    run(["cargo", "rullst", "new", name, "--default", "--blueprint", starter,
         "--skip-initial-migration"], cwd=projects, env=env, timeout=300)
    project = projects / name
    dependencies = tomllib.loads((project / "Cargo.toml").read_text())["dependencies"]
    expect(not any(isinstance(spec, dict) and "path" in spec for spec in dependencies.values()),
           "the generated manifest uses registry dependencies only")
    expect(dependencies["rullst"]["version"] == f"={version}",
           f"the generated manifest pins rullst ={version}")

    deploy = run(["cargo", "rullst", "deploy"], cwd=project, env=env, capture=True, timeout=120)
    log(deploy.stdout)
    for file in GENERATED:
        expect((project / file).is_file(), f"`cargo rullst deploy` wrote {file}")
    config = tomllib.loads((project / "Rullst.toml").read_text())
    expect(config.get("security", {}).get("trusted_proxies") == [CADDY_ADDRESS],
           "Rullst.toml trusts only the generated Caddy address")
    return project, {file: sha256(project / file) for file in GENERATED}


def patch_release_crates(project: Path, extracted: Path, crates: list[str], version: str) -> None:
    """The only harness change to the project: archives inside the build context."""
    target = project / PATCH_DIRECTORY
    target.mkdir()
    paths = {}
    for crate in crates:
        shutil.copytree(extracted / f"{crate}-{version}", target / f"{crate}-{version}")
        paths[crate] = f"{PATCH_DIRECTORY}/{crate}-{version}"
    manifest = project / "Cargo.toml"
    manifest.write_text(manifest.read_text() + patch_section(paths))
    # The Dockerfile builds with --locked: "Generate and commit Cargo.lock".
    run(["cargo", "generate-lockfile"], cwd=project, timeout=600)


def production_settings(starter: str) -> str:
    """The minimal `.env.production` of the guide; SaaS adds its billing settings."""
    # RULLST_ENV is also set by the image; `deploy:doctor` checks it in this file.
    settings = {"RULLST_ENV": "production", "APP_KEY": secrets.token_hex(32)}
    if starter == "saas":
        # Production refuses mock billing; Stripe is never contacted here.
        settings.update({
            "BILLING_API_KEY": "sk_test_rullst_ci_placeholder",
            "BILLING_WEBHOOK_SECRET": "whsec_" + secrets.token_hex(24),
            "BILLING_REDIRECT_URL": "https://localhost/dashboard",
            "BILLING_ALLOWED_PLAN_IDS": "price_starter,price_pro",
            "RULLST_PUBLIC_ORIGIN": "https://localhost",
        })
    return "".join(f"{key}={value}\n" for key, value in settings.items())


def copy_to_vm(project: Path, starter: str, work: Path) -> Path:
    """`rsync -av --exclude target --exclude .git --exclude '.env*' ./ vm:~/app/`."""
    vm = work / "vm" / project.name
    shutil.copytree(project, vm, ignore=shutil.ignore_patterns("target", ".git", ".env*"))
    (vm / ".env").write_text("DOMAIN=localhost\n")
    production = vm / ".env.production"
    production.write_text(production_settings(starter))
    production.chmod(0o600)
    return vm


def proc_status(pid: int) -> dict[str, str]:
    lines = Path(f"/proc/{pid}/status").read_text().splitlines()
    return {key: value.strip() for key, _, value in (line.partition(":") for line in lines)}


def wait_until_healthy(engine: Engine, first_start: bool, deadline: float = 300) -> tuple[str, str]:
    """Waits for the Caddy health check (it probes the app's /health)."""
    started = time.monotonic()
    while time.monotonic() - started < deadline:
        app, caddy = engine.containers("app"), engine.containers("caddy")
        if len(app) == 1 and len(caddy) == 1:
            state = engine.inspect(app[0])
            # A crash loop shows as policy restarts; `compose restart` itself
            # may count as one on some engines, so only the first start checks.
            if state["State"].get("Status") in ("exited", "dead") or (
                    first_start and (state.get("RestartCount") or state["State"].get("Restarting"))):
                raise Failure(f"the app container stopped (state {state['State'].get('Status')}, "
                              f"exit {state['State'].get('ExitCode')}, "
                              f"restarts {state.get('RestartCount')})")
            state = engine.inspect(caddy[0])["State"]
            health = (state.get("Health") or state.get("Healthcheck") or {}).get("Status")
            if health == "healthy":
                log(f"ok: compose reports Caddy healthy after {time.monotonic() - started:.0f}s")
                return app[0], caddy[0]
            # Rootless Podman runs health checks from systemd user timers; when
            # the runner has none, run the generated check explicitly.
            if engine.name == "podman" and time.monotonic() - started > 90:
                if engine("healthcheck", "run", caddy[0], check=False).returncode == 0:
                    log("ok: `podman healthcheck run` passed (no systemd timer ran it)")
                    return app[0], caddy[0]
        time.sleep(5)
    raise Failure(f"Caddy did not report healthy within {deadline:.0f}s")


def check_hardening(engine: Engine, container: str, service: str, capabilities: int) -> dict:
    info = engine.inspect(container)
    host = info["HostConfig"]
    log(f"{service}: SecurityOpt={host.get('SecurityOpt')} CapDrop={host.get('CapDrop')} "
        f"CapAdd={host.get('CapAdd')} Tmpfs={host.get('Tmpfs')}")
    expect(host.get("ReadonlyRootfs") is True, f"{service} has a read-only root filesystem")
    expect(not host.get("Privileged"), f"{service} is not privileged")
    expect("/tmp" in (host.get("Tmpfs") or {}), f"{service} has a tmpfs on /tmp")
    status = proc_status(int(info["State"]["Pid"]))
    expect(status.get("NoNewPrivs") == "1", f"{service} runs with no-new-privileges")
    for field in ("CapEff", "CapPrm", "CapBnd"):
        expect(int(status[field], 16) == capabilities,
               f"{service} {field} is {capabilities:#x} (cap_drop ALL"
               + (", cap_add NET_BIND_SERVICE)" if capabilities else ")"))
    return info


def check_ports(app: dict, caddy: dict) -> None:
    for name, info in (("app", app), ("caddy", caddy)):
        log(f"{name}: PortBindings={info['HostConfig'].get('PortBindings')} "
            f"Ports={info['NetworkSettings'].get('Ports')}")
    expect(not any((app["HostConfig"].get("PortBindings") or {}).values())
           and not any((app["NetworkSettings"].get("Ports") or {}).values()),
           "the app port is not published on the host")
    with socket.socket() as probe:
        probe.settimeout(3)
        expect(probe.connect_ex(("127.0.0.1", 3000)) != 0, "nothing answers on host port 3000")
    published = set(caddy["HostConfig"].get("PortBindings") or {})
    expect({"80/tcp", "443/tcp", "443/udp"} <= published, "Caddy publishes 80, 443 and 443/udp")
    addresses = [network.get("IPAddress") for network in caddy["NetworkSettings"]["Networks"].values()]
    expect(CADDY_ADDRESS in addresses, f"Caddy holds the trusted proxy address {CADDY_ADDRESS}")
    expect(app["Config"].get("User") == "10001:10001", "the app runs as 10001:10001")


def caddy_root(engine: Engine, caddy: str, work: Path) -> Path:
    ca = work / f"{engine.project}-caddy-root.crt"
    for _ in range(30):
        if engine("cp", f"{caddy}:/data/caddy/pki/authorities/local/root.crt", str(ca),
                  check=False).returncode == 0:
            return ca
        time.sleep(2)
    raise Failure("Caddy's local CA certificate never appeared")


def check_database(engine: Engine, project: Path, app: dict, binary: str, command: str) -> str:
    output = engine.compose(project, "run", "--rm", "app", f"/app/{binary}", command,
                            capture=True, timeout=300).stdout
    log(output)
    if command == "db:status":
        expect("Applied" in output and "Pending" not in output,
               "the migrations applied before the restart are still recorded")
    volume = next(mount["Name"] for mount in app["Mounts"] if mount.get("Destination") == "/app/data")
    listing = engine.output("run", "--rm", "-v", f"{volume}:/data:ro", "--entrypoint", "ls",
                            CADDY_IMAGE, "-ln", "/data")
    log(listing)
    databases = [line.split() for line in listing.splitlines()
                 if line.startswith("-") and line.endswith(DATABASE_SUFFIXES)]
    expect(any(fields[2] == "10001" and int(fields[4]) > 0 for fields in databases),
           f"the SQLite database is a non-empty file owned by 10001 in volume {volume}")
    return volume


def check_backup(engine: Engine, volume: str, work: Path, account: bool) -> None:
    """The guide's SQLite backup command (section 8), then the copy is opened."""
    backup = work / f"{engine.project}-backup"
    backup.mkdir()
    script = ('apk add --no-cache sqlite >/dev/null && sqlite3 /data/rullst.db '
              '".backup /backup/rullst-$(date +%F).db"')
    engine("run", "--rm", "-v", f"{volume}:/data", "-v", f"{backup}:/backup",
           "docker.io/library/alpine:3", "sh", "-c", script, timeout=300)
    copies = list(backup.glob("rullst-*.db"))
    expect(len(copies) == 1, "the backup command wrote one database copy")
    with sqlite3.connect(f"file:{copies[0]}?mode=ro", uri=True) as database:
        users = database.execute("SELECT COUNT(*) FROM users").fetchone()[0]
    expect(users == (1 if account else 0), f"the backup holds the users table ({users} rows)")


def check_logs(engine: Engine) -> None:
    for container in engine.containers():
        logs = engine.output("logs", container, quiet=True)
        expect("panicked" not in logs, f"container {container[:12]} logged no panic")
        if container in engine.containers("app"):
            log("\n".join(logs.splitlines()[:25]))
        else:
            expect("Caddyfile input is not formatted" not in logs, "Caddy accepts the Caddyfile as formatted")


def diagnostics(engine: Engine) -> None:
    log("::group::Container diagnostics")
    engine("ps", "-a", "--filter", f"label=com.docker.compose.project={engine.project}", check=False)
    for container in engine.containers():
        engine("inspect", "--format", "{{.Name}} {{.State.Status}} exit={{.State.ExitCode}} "
               "restarts={{.RestartCount}}", container, check=False)
        engine("logs", "--tail", "200", container, check=False)
    log("::endgroup::")


def restart(engine: Engine, vm: Path) -> None:
    """`compose restart`. Rootless Podman on the Ubuntu runner may fail to tear
    down its pasta network namespace; that is reported as a warning (with the
    kernel's AppArmor messages) and `compose up -d` starts what stopped."""
    result = engine.compose(vm, "restart", capture=True, check=False, timeout=300)
    log(result.stdout)
    if result.returncode == 0:
        return
    if engine.name != "podman" or "rootless netns" not in result.stdout:
        raise Failure(f"`{engine.name} compose restart` exited with {result.returncode}")
    log("::warning title=Rootless Podman restart::`podman compose restart` failed to stop "
        "the rootless network namespace (see the log); ran `podman compose up -d`")
    denials = run(["sudo", "dmesg"], capture=True, check=False, quiet=True).stdout
    log("\n".join(line for line in denials.splitlines() if "apparmor" in line.lower())[-4000:])
    engine("info", "--format", "network={{.Host.RootlessNetworkCmd}}", check=False)
    engine.compose(vm, "up", "-d", timeout=600)


def deploy(engine: Engine, starter: str, vm: Path, work: Path) -> None:
    binary = tomllib.loads((vm / "Cargo.toml").read_text())["package"]["name"]
    password = secrets.token_urlsafe(18)
    with step(f"{starter}: build the image and start compose.yaml with {engine.name}"):
        engine.compose(vm, "up", "-d", "--build", timeout=25 * 60)
    with step(f"{starter}: wait for the health check and check the deployment"):
        app_id, caddy_id = wait_until_healthy(engine, first_start=True)
        app = check_hardening(engine, app_id, "app", 0)
        caddy = check_hardening(engine, caddy_id, "caddy", NET_BIND_SERVICE)
        check_ports(app, caddy)
        ca = caddy_root(engine, caddy_id, work)
        check_https(ca)
    with step(f"{starter}: migrate once, restart, and check the database volume"):
        check_database(engine, vm, app, binary, "db:migrate")
        check_application(starter, ca, password, registered=False)
        restart(engine, vm)
        app_id, _ = wait_until_healthy(engine, first_start=False)
        check_https(ca)
        volume = check_database(engine, vm, engine.inspect(app_id), binary, "db:status")
        check_application(starter, ca, password, registered=True)
        check_backup(engine, volume, work, account=starter == "saas")
        check_logs(engine)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--engine", choices=("docker", "podman"), required=True)
    parser.add_argument("--packages", type=Path, default=ROOT / "target/package")
    parser.add_argument("--starter", choices=STARTERS, action="append")
    args = parser.parse_args()
    if os.environ.get("GITHUB_ACTIONS") != "true" and os.environ.get("RULLST_VM_DEPLOY_SMOKE") != "1":
        log("This check binds ports 80/443 and builds container images; it runs in CI "
            "(or set RULLST_VM_DEPLOY_SMOKE=1 on a disposable machine).")
        return 2
    version = tomllib.loads((ROOT / "cargo-rullst/Cargo.toml").read_text())["package"]["version"]
    work = Path(os.environ.get("RUNNER_TEMP") or "/tmp") / "rullst-vm-deploy"
    if work.exists():
        shutil.rmtree(work)
    engine = Engine(args.engine)
    run(["df", "-h", "/"])
    with step("Install the CLI from the packaged archives"):
        extracted, crates = extract_archives(args.packages.resolve(), version, work)
        env = install_cli(extracted, crates, version, work)

    for starter in args.starter or STARTERS:
        with step(f"{starter}: create the project and run `cargo rullst deploy`"):
            project, digests = create_project(starter, env, version, work)
            patch_release_crates(project, extracted, crates, version)
            vm = copy_to_vm(project, starter, work)
            # The guide's offline check of the production settings file.
            run(["cargo", "rullst", "deploy:doctor", "--env-file", ".env.production"],
                cwd=vm, env=env, timeout=120)
        engine.project = vm.name
        failed = True
        try:
            deploy(engine, starter, vm, work)
            expect({file: sha256(vm / file) for file in GENERATED} == digests,
                   "the generated deployment files ran unmodified")
            failed = False
        finally:
            if failed:
                diagnostics(engine)
            engine.compose(vm, "down", "-v", check=False, timeout=300)
            run(["df", "-h", "/"], check=False)
        expect(not engine.containers(), "`compose down -v` removed every container")

    summary = [f"| {title} | {seconds / 60:.1f} min |" for title, seconds in TIMINGS]
    report = "\n".join([f"### Generated VM deployment ({engine.name})", "", "| Step | Time |",
                        "| :--- | ---: |", *summary, ""])
    log(report)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as handle:
            handle.write(report + "\n")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (Failure, subprocess.TimeoutExpired) as error:
        log(f"::error title=Generated VM deployment::{error}")
        sys.exit(1)
