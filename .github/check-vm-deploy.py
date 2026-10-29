#!/usr/bin/env python3
"""Start the files `cargo rullst deploy` writes, on a real engine (Linux CI only).

Installs the CLI from the packaged release archives, creates a Blank starter,
runs `cargo rullst deploy`, copies the project the way the VM guide does
(docs/src/vm-deployment.md, step 4A), builds the image with the generated
Dockerfile and starts the generated compose.yaml and Caddyfile with Docker or
rootless Podman. It then checks /health through Caddy over HTTPS (Caddy's
internal CA for localhost), the HTTP redirect, that the app port is not
published, the generated hardening, the SQLite database in its volume across
`compose restart`, and the logs.

The generated Dockerfile, .dockerignore, compose.yaml and Caddyfile are never
modified. The release crates are not on crates.io before publication, so the
harness copies their packaged archives into the build context and appends a
`[patch.crates-io]` section to Cargo.toml before resolving Cargo.lock: the
substitution .github/test-packaged-distribution.sh makes outside a container.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import socket
import subprocess
import sys
import tarfile
import time
import tomllib

ROOT = Path(__file__).resolve().parent.parent
PROJECT = "deploy-smoke"
CADDY_ADDRESS = "172.31.250.10"
# Already pulled by compose.yaml; used read-only to list the data volume.
CADDY_IMAGE = "docker.io/library/caddy:2-alpine"
NET_BIND_SERVICE = 1 << 10
GENERATED = ("Dockerfile", ".dockerignore", "compose.yaml", "Caddyfile")
CLI_PACKAGES = ("cargo-rullst", "cargo-rullst-extras")
PATCH_DIRECTORY = "rullst-ci-packages"
DATABASE_SUFFIXES = (".db", ".sqlite", ".sqlite3")
TIMINGS: list[tuple[str, float]] = []


class Failure(Exception):
    """A deployment check failed; the message says which."""


def log(message: str) -> None:
    print(message, flush=True)


def expect(condition: bool, message: str) -> None:
    if not condition:
        raise Failure(message)
    log(f"ok: {message}")


class Step:
    """Collapses a step's output in the Actions log and records its duration."""

    def __init__(self, title: str):
        self.title = title
        self.started = 0.0

    def __enter__(self) -> "Step":
        log(f"::group::{self.title}")
        self.started = time.monotonic()
        return self

    def __exit__(self, *_exception: object) -> None:
        TIMINGS.append((self.title, time.monotonic() - self.started))
        log("::endgroup::")


def run(command: list[str], *, cwd: Path | None = None, env: dict[str, str] | None = None,
        timeout: int = 600, capture: bool = False, check: bool = True) -> subprocess.CompletedProcess:
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
        return json.loads(self.output("inspect", container))[0]

    def containers(self, service: str | None = None) -> list[str]:
        filters = ["--filter", f"label=com.docker.compose.project={PROJECT}"]
        if service:
            filters += ["--filter", f"label=com.docker.compose.service={service}"]
        return self.output("ps", "-aq", *filters).split()


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
    lines = [
        "",
        "# Added by .github/check-vm-deploy.py: the release crates are not on",
        "# crates.io before publication, so these packaged archives replace them.",
        "[patch.crates-io]",
    ]
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


def create_project(env: dict[str, str], version: str, work: Path) -> tuple[Path, dict[str, str]]:
    projects = work / "projects"
    projects.mkdir()
    run(["cargo", "rullst", "new", PROJECT, "--default", "--blueprint", "blank",
         "--skip-initial-migration"], cwd=projects, env=env, timeout=300)
    project = projects / PROJECT
    manifest = tomllib.loads((project / "Cargo.toml").read_text())
    dependencies = manifest["dependencies"]
    expect(not any(isinstance(spec, dict) and "path" in spec for spec in dependencies.values()),
           "the generated manifest uses registry dependencies only")
    expect(dependencies["rullst"]["version"] == f"={version}",
           f"the generated manifest pins rullst ={version}")

    deploy = run(["cargo", "rullst", "deploy"], cwd=project, env=env, capture=True, timeout=120)
    log(deploy.stdout)
    for name in GENERATED:
        expect((project / name).is_file(), f"`cargo rullst deploy` wrote {name}")
    config = tomllib.loads((project / "Rullst.toml").read_text())
    expect(config.get("security", {}).get("trusted_proxies") == [CADDY_ADDRESS],
           "Rullst.toml trusts only the generated Caddy address")
    return project, {name: sha256(project / name) for name in GENERATED}


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


def copy_to_vm(project: Path, work: Path) -> Path:
    """`rsync -av --exclude target --exclude .git --exclude '.env*' ./ vm:~/app/`."""
    vm = work / "vm" / PROJECT
    shutil.copytree(project, vm, ignore=shutil.ignore_patterns("target", ".git", ".env*"))
    (vm / ".env").write_text("DOMAIN=localhost\n")
    production = vm / ".env.production"
    production.write_text(f"APP_KEY={secrets.token_hex(32)}\n")
    production.chmod(0o600)
    return vm


def proc_status(pid: int) -> dict[str, str]:
    fields = {}
    for line in Path(f"/proc/{pid}/status").read_text().splitlines():
        key, _, value = line.partition(":")
        fields[key] = value.strip()
    return fields


def fetch(url: str, ca: Path, work: Path) -> tuple[int, str, str, str]:
    headers, body = work / "response-headers.txt", work / "response-body.txt"
    result = run(["curl", "-sS", "--max-time", "20", "--cacert", str(ca),
                  "--resolve", "localhost:443:127.0.0.1", "--resolve", "localhost:80:127.0.0.1",
                  "-D", str(headers), "-o", str(body), "-w", "%{http_code} %{redirect_url}", url],
                 capture=True, check=False)
    if result.returncode != 0:
        raise Failure(f"curl {url} failed: {result.stdout.strip()}")
    code, _, redirect = result.stdout.strip().partition(" ")
    return int(code), headers.read_text().lower(), body.read_text(errors="replace"), redirect


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


def check_ports(engine: Engine, app: dict, caddy: dict) -> None:
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
    ca = work / "caddy-root.crt"
    for _ in range(30):
        if engine("cp", f"{caddy}:/data/caddy/pki/authorities/local/root.crt", str(ca),
                  check=False).returncode == 0:
            return ca
        time.sleep(2)
    raise Failure("Caddy's local CA certificate never appeared")


def check_https(ca: Path, work: Path) -> None:
    code, headers, body, _ = fetch("https://localhost/health", ca, work)
    expect(code == 200, f"GET https://localhost/health through Caddy answers {code}")
    try:
        json.loads(body)
    except json.JSONDecodeError as error:
        raise Failure(f"/health did not answer JSON: {error}") from error
    expect("x-content-type-options: nosniff" in headers, "the app's security headers pass through Caddy")
    code, _, _, redirect = fetch("http://localhost/health", ca, work)
    expect(code in (301, 308) and redirect == "https://localhost/health",
           f"HTTP redirects to HTTPS ({code} -> {redirect})")


def check_database(engine: Engine, project: Path, app: dict, binary: str,
                   ca: Path, work: Path, command: str) -> None:
    output = engine.compose(project, "run", "--rm", "app", f"/app/{binary}", command,
                            capture=True, timeout=300).stdout
    log(output)
    if command == "db:status":
        expect("Applied" in output and "Pending" not in output,
               "the migration applied before the restart is still recorded")
    code, _, body, _ = fetch("https://localhost/", ca, work)
    expect(code == 200 and "Database connected." in body, f"the home page reaches the database ({code})")
    volume = next(mount["Name"] for mount in app["Mounts"] if mount.get("Destination") == "/app/data")
    listing = engine.output("run", "--rm", "-v", f"{volume}:/data:ro", "--entrypoint", "ls",
                            CADDY_IMAGE, "-ln", "/data")
    log(listing)
    databases = [line.split() for line in listing.splitlines()
                 if line.startswith("-") and line.endswith(DATABASE_SUFFIXES)]
    expect(any(fields[2] == "10001" and int(fields[4]) > 0 for fields in databases),
           f"the SQLite database is a non-empty file owned by 10001 in volume {volume}")


def check_logs(engine: Engine) -> None:
    for container in engine.containers():
        logs = engine.output("logs", container)
        expect("panicked" not in logs, f"container {container[:12]} logged no panic")


def diagnostics(engine: Engine, project: Path) -> None:
    log("::group::Container diagnostics")
    engine("ps", "-a", "--filter", f"label=com.docker.compose.project={PROJECT}", check=False)
    for container in engine.containers():
        engine("inspect", "--format", "{{.Name}} {{.State.Status}} exit={{.State.ExitCode}} "
               "restarts={{.RestartCount}}", container, check=False)
        engine("logs", "--tail", "200", container, check=False)
    log("::endgroup::")


def deploy(engine: Engine, vm: Path, binary: str, work: Path) -> None:
    with Step(f"Build the image and start compose.yaml with {engine.name}"):
        engine.compose(vm, "up", "-d", "--build", timeout=50 * 60)
    with Step("Wait for the health check and check the deployment"):
        app_id, caddy_id = wait_until_healthy(engine, first_start=True)
        app = check_hardening(engine, app_id, "app", 0)
        caddy = check_hardening(engine, caddy_id, "caddy", NET_BIND_SERVICE)
        check_ports(engine, app, caddy)
        ca = caddy_root(engine, caddy_id, work)
        check_https(ca, work)
    with Step("Migrate once, restart, and check the database volume"):
        check_database(engine, vm, app, binary, ca, work, "db:migrate")
        engine.compose(vm, "restart", timeout=300)
        app_id, caddy_id = wait_until_healthy(engine, first_start=False)
        check_https(ca, work)
        check_database(engine, vm, engine.inspect(app_id), binary, ca, work, "db:status")
        check_logs(engine)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--engine", choices=("docker", "podman"), required=True)
    parser.add_argument("--packages", type=Path, default=ROOT / "target/package")
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

    with Step("Install the CLI from the packaged archives"):
        extracted, crates = extract_archives(args.packages.resolve(), version, work)
        env = install_cli(extracted, crates, version, work)
    with Step("Create a Blank project and run `cargo rullst deploy`"):
        project, digests = create_project(env, version, work)
        patch_release_crates(project, extracted, crates, version)
        vm = copy_to_vm(project, work)
    binary = tomllib.loads((vm / "Cargo.toml").read_text())["package"]["name"]

    failed = True
    try:
        deploy(engine, vm, binary, work)
        expect({name: sha256(vm / name) for name in GENERATED} == digests,
               "the generated deployment files ran unmodified")
        failed = False
    finally:
        if failed:
            diagnostics(engine, vm)
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
