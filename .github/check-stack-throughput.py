#!/usr/bin/env python3
"""Check the throughput of generated applications' production stacks.

Blank starter: builds .github/stack-throughput (the router, layers and
release profile of a `cargo rullst new` application, against this checkout)
and serves `/json` two ways on one machine: `STACK=generated`
(`Server::new(router()).run()` with `RULLST_ENV=production`: the production
security baseline and the access log, writing to a pipe like a container log
driver) and `STACK=minimal` (the same routes through `axum::serve`, without
any layer). Each request has no cookie, so the CSRF layer mints a token.

SaaS starter: generates the SaaS starter with the CLI built from this
checkout (`--default`: SQLite, the strict profile it selects), builds it in
release mode with its generated `[profile.release]`, migrates it, starts it
with `RULLST_ENV=production`, registers one account and measures the
signed-in `GET /dashboard` with that session's cookies: the CSRF mask, the
session registry's lookup of the `rullst_session` cookie and the page.

The server runs on two CPUs and the pinned load generator (bombardier, as
Rullst/Benchmarks uses it) on two others. Rounds alternate the stacks; the
medians give each ratio to the minimal stack's requests per second.

Blocking: a ratio below its `min_ratio` in .github/stack-throughput/budget.toml
(`[throughput]` for the Blank stack, `[saas]` for the SaaS page); a non-2xx
response; a generated response without the production security headers, a
Blank one without the CSRF cookie, a SaaS page that is not the signed-in
dashboard (the ratio would not measure the stack); a server that does not
start; fewer than four usable CPUs. Absolute figures are reported, never
limited: they vary between shared runners.

Usage: check-stack-throughput.py --bombardier PATH --cli PATH [--work DIR]
                                 [--out DIR] [--report-only]
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
from pathlib import Path
import secrets
import shutil
import signal
import statistics
import subprocess
import sys
import threading
import time
import tomllib
import urllib.error
import urllib.parse
import urllib.request

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
APP = HERE / "stack-throughput"
BUDGET = APP / "budget.toml"
TARGET = ROOT / "target" / "stack-throughput"
BINARY = TARGET / "release" / "rullst-stack-throughput"
TITLE = "Generated stack throughput"
PORT = 18080
ROUNDS = 3
CONNECTIONS = 125
WARMUP_S = 3
DURATION_S = 10
# Headroom of the suggested limit below the measured ratio.
MARGIN = 0.15
# The stacks of a round, in order, and what each one serves.
STACKS = ("minimal", "generated", "saas")
LABELS = {"generated": "Blank generated/minimal", "saas": "SaaS signed-in dashboard/minimal"}
# The budget table of each stack's limit.
BUDGET_TABLES = {"generated": "throughput", "saas": "saas"}
DASHBOARD = "/dashboard"
ACCOUNT_NAME = "Throughput Check"
SAAS_PROJECT = "throughput-saas"
SAAS_TARGET = ROOT / "target" / "stack-throughput-saas"
SAAS_BUILD_DEADLINE = 45 * 60
# Production refuses mock billing: placeholders as in .github/check-vm-deploy.py.
# Stripe is never contacted; only the dashboard is requested.
SAAS_SETTINGS = {
    "BILLING_API_KEY": "sk_test_rullst_ci_placeholder",
    "BILLING_REDIRECT_URL": "https://localhost/dashboard",
    "BILLING_ALLOWED_PLAN_IDS": "price_starter,price_pro",
    "RULLST_PUBLIC_ORIGIN": "https://localhost",
}
# Headers the production stack must add to `/json` and to the dashboard.
SECURITY_HEADERS = (
    "content-security-policy",
    "strict-transport-security",
    "x-frame-options",
    "x-content-type-options",
    "cross-origin-opener-policy",
    "referrer-policy",
)


_SPEC = importlib.util.spec_from_file_location("dast_generated_app", HERE / "dast-generated-app.py")
if _SPEC is None or _SPEC.loader is None:
    raise RuntimeError("cannot load .github/dast-generated-app.py")
DAST = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(DAST)


class Failure(Exception):
    """A blocking result, with the text for the summary."""


def figures(result: dict, cpu_seconds: float) -> dict:
    """The figures of one bombardier `-o j` result."""
    body = result["result"]
    ok = body["req2xx"]
    errors = sum(body[key] for key in ("req1xx", "req3xx", "req4xx", "req5xx", "others"))
    taken = body["timeTakenSeconds"]
    return {
        "rps": ok / taken if taken > 0 else 0.0,
        "p99_ms": body["latency"]["percentiles"]["99"] / 1000,
        "errors": errors,
        "cpu_us_per_request": cpu_seconds * 1e6 / ok if ok else 0.0,
        "server_cpus": cpu_seconds / taken if taken > 0 else 0.0,
    }


def ratio(rounds: list[dict], stack: str = "generated") -> float:
    """Median requests per second of `stack` over median minimal ones."""
    measured = statistics.median(r[stack]["rps"] for r in rounds)
    minimal = statistics.median(r["minimal"]["rps"] for r in rounds)
    return measured / minimal if minimal > 0 else 0.0


def limits(budget: dict) -> dict[str, float]:
    """The `min_ratio` of each stack in a parsed budget.toml."""
    return {stack: budget[table]["min_ratio"] for stack, table in BUDGET_TABLES.items()}


def verdict(rounds: list[dict], min_ratios: dict[str, float] | None) -> list[str]:
    """The blocking problems of `rounds` under `min_ratios` (none: report only)."""
    problems = []
    for index, measured in enumerate(rounds, 1):
        for stack in STACKS:
            if stack in measured and measured[stack]["errors"]:
                problems.append(f"round {index}: {measured[stack]['errors']} non-2xx responses from the {stack} stack")
    for stack, limit in (min_ratios or {}).items():
        measured_ratio = ratio(rounds, stack)
        if measured_ratio < limit:
            problems.append(f"{LABELS[stack]} ratio {measured_ratio:.3f} is below the limit {limit:.3f}")
    return problems


def missing_security(headers: dict[str, str], cookieless: bool = True) -> list[str]:
    """What a generated response lacks of the production stack. A cookieless
    request must also receive the CSRF cookie."""
    names = {name.lower(): value for name, value in headers.items()}
    missing = [name for name in SECURITY_HEADERS if name not in names]
    if cookieless and not names.get("set-cookie", "").startswith("rullst_csrf="):
        missing.append("set-cookie: rullst_csrf")
    return missing


def cpu_model(cpuinfo: str) -> str:
    """The first `model name` of /proc/cpuinfo text: ratios differ between runner models."""
    for line in cpuinfo.splitlines():
        name, _, value = line.partition(":")
        if name.strip() == "model name":
            return value.strip()
    return "unknown CPU"


def summary(rounds: list[dict], min_ratios: dict[str, float] | None, problems: list[str],
            cpu: str = "unknown CPU") -> str:
    """The job summary in Markdown."""
    lines = [f"## {TITLE}", "", f"CPU: {cpu}", ""]
    lines.append("| Round | Stack | Requests/s | p99 (ms) | Server CPU µs/request | Server CPUs |")
    lines.append("| ---: | :--- | ---: | ---: | ---: | ---: |")
    for index, measured in enumerate(rounds, 1):
        for stack in STACKS:
            if stack not in measured:
                continue
            f = measured[stack]
            lines.append(f"| {index} | {stack} | {f['rps']:.0f} | {f['p99_ms']:.2f} | "
                         f"{f['cpu_us_per_request']:.2f} | {f['server_cpus']:.2f} |")
    lines.append("")
    for stack, label in LABELS.items():
        if not rounds or stack not in rounds[0]:
            continue
        measured_ratio = ratio(rounds, stack)
        line = f"{label} ratio (medians): **{measured_ratio:.3f}**."
        if min_ratios is not None and stack in min_ratios:
            line += f" Limit: {min_ratios[stack]:.3f}. Suggested limit: {measured_ratio * (1 - MARGIN):.3f}."
        lines.append(line)
    lines += ["", "**Failed:** " + "; ".join(problems) if problems else "Passed."]
    return "\n".join(lines) + "\n"


def cpus() -> tuple[str, str]:
    available = sorted(os.sched_getaffinity(0))
    if len(available) < 4:
        raise Failure(f"needs four usable CPUs, found {len(available)}")
    return ",".join(map(str, available[:2])), ",".join(map(str, available[2:4]))


def build() -> None:
    shutil.copyfile(ROOT / "Cargo.lock", APP / "Cargo.lock")
    command = ["cargo", "build", "--release", "--manifest-path", str(APP / "Cargo.toml"),
               "--target-dir", str(TARGET)]
    if subprocess.run(command, cwd=ROOT, check=False).returncode != 0:
        raise Failure("the stack-throughput application does not build")


def toolchain() -> str:
    """The channel of rust-toolchain.toml, which does not reach a project outside the checkout."""
    return tomllib.loads((ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))["toolchain"]["channel"]


def run(command: list[str], cwd: Path, env: dict[str, str], what: str, timeout: int = 600) -> None:
    try:
        done = subprocess.run(command, cwd=cwd, env=env, check=False, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        raise Failure(f"{what} did not finish within {timeout // 60} minutes") from error
    if done.returncode != 0:
        raise Failure(f"{what} failed with exit code {done.returncode}")


def prepare_saas(cli: Path, work: Path) -> tuple[Path, Path]:
    """Generates, builds (release) and migrates the SaaS starter; its project and binary."""
    env = dict(os.environ, RULLST_DISABLE_UPDATE_CHECK="true", CARGO_NET_OFFLINE="true",
               RUSTUP_TOOLCHAIN=toolchain())
    for name in ("PORT", "HOST", "RULLST_ENV", "CARGO_TARGET_DIR"):
        env.pop(name, None)
    project = work / SAAS_PROJECT
    if project.exists():
        shutil.rmtree(project)
    work.mkdir(parents=True, exist_ok=True)
    run([str(cli), "new", str(project), "--default", "--blueprint", "saas", "--skip-initial-migration"],
        cwd=work, env=env, what="generating the SaaS starter")
    shutil.copyfile(ROOT / "Cargo.lock", project / "Cargo.lock")
    DAST.command_registry_patch(project)
    run(["cargo", "build", "--release", "--offline", "--manifest-path", str(project / "Cargo.toml"),
         "--target-dir", str(SAAS_TARGET)], cwd=project, env=env, what="building the SaaS starter",
        timeout=SAAS_BUILD_DEADLINE)
    binary = SAAS_TARGET / "release" / SAAS_PROJECT
    run([str(binary), "db:migrate"], cwd=project, env=env, what="migrating the SaaS starter")
    return project, binary


def cpu_seconds(pid: int) -> float:
    fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
    return (int(fields[11]) + int(fields[12])) / os.sysconf("SC_CLK_TCK")


def get(path: str, cookie: str | None = None) -> tuple[int, dict[str, str], str]:
    request = urllib.request.Request(f"http://127.0.0.1:{PORT}{path}")
    if cookie:
        request.add_header("Cookie", cookie)
    try:
        with urllib.request.urlopen(request, timeout=2) as response:
            body = response.read(1 << 20).decode(errors="replace")
            return response.status, {name: value for name, value in response.getheaders()}, body
    except urllib.error.HTTPError as error:
        return error.code, dict(error.headers.items()), ""


class Served:
    """One server process on the server CPUs, its stdout read and discarded."""

    def __init__(self, stack: str, server_cpus: str, saas: tuple[Path, Path, dict[str, str]] | None = None):
        env = dict(os.environ, RULLST_ENV="production", HOST="127.0.0.1", PORT=str(PORT), STACK=stack)
        env.pop("RULLST_ACCESS_LOG", None)
        command, cwd, ready = [str(BINARY)], APP, "/json"
        if stack == "saas":
            if saas is None:
                raise Failure("the SaaS stack needs its prepared project")
            project, binary, settings = saas
            env.update(settings)
            command, cwd, ready = [str(binary)], project, "/health"
        self.process = subprocess.Popen(["taskset", "-c", server_cpus, *command], cwd=cwd, env=env,
                                        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        self.reader = threading.Thread(target=self._drain, daemon=True)
        self.reader.start()
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise Failure(f"the {stack} server exited with {self.process.returncode}")
            try:
                if get(ready)[0] == 200:
                    return
            except OSError:
                pass
            time.sleep(0.2)
        self.stop()
        raise Failure(f"the {stack} server did not answer within 60 s")

    def _drain(self) -> None:
        assert self.process.stdout is not None
        while self.process.stdout.read(65536):
            pass

    def stop(self) -> None:
        if self.process.poll() is None:
            self.process.send_signal(signal.SIGTERM)
            try:
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
        self.reader.join(timeout=5)


def sign_up() -> str:
    """Registers a disposable account on the running SaaS starter; the Cookie
    header of its session (with the CSRF cookie a browser also sends)."""
    base = urllib.parse.urlsplit(f"http://127.0.0.1:{PORT}")
    try:
        cookies: dict[str, str] = {}
        status, issued, _, _ = DAST.request(base, "GET", "/register", cookies)
        cookies.update(issued)
        if status != 200 or not cookies.get("rullst_csrf"):
            raise Failure(f"GET /register answered {status} without a CSRF cookie")
        form = {"_token": cookies["rullst_csrf"], "name": ACCOUNT_NAME,
                "email": f"throughput-{secrets.token_hex(6)}@example.com", "password": secrets.token_urlsafe(24)}
        status, issued, headers, _ = DAST.request(base, "POST", "/register", cookies, form)
    except DAST.Failure as error:
        raise Failure(str(error)) from error
    cookies.update(issued)
    if status not in (302, 303) or headers["location"] != DASHBOARD or not cookies.get("rullst_session"):
        raise Failure(f"POST /register answered {status} -> {headers['location']!r} without a session")
    return "; ".join(f"{name}={cookies[name]}" for name in ("rullst_session", "rullst_csrf"))


def dashboard_problems(status: int, headers: dict[str, str], body: str) -> list[str]:
    """Why a signed-in `GET /dashboard` response is not the measured page."""
    problems = [] if status == 200 else [f"status {status}"]
    if ACCOUNT_NAME not in body:
        problems.append("the account's dashboard")
    problems += missing_security(headers, cookieless=False)
    return problems


def load(bombardier: str, client_cpus: str, seconds: int, path: str = "/json", cookie: str | None = None) -> dict:
    command = ["taskset", "-c", client_cpus, bombardier, "--fasthttp", "-c", str(CONNECTIONS),
               "-d", f"{seconds}s", "-t", "5s", "-l", "-p", "r", "-o", "j"]
    if cookie:
        command += ["-H", f"Cookie: {cookie}"]
    done = subprocess.run(command + [f"http://127.0.0.1:{PORT}{path}"], capture_output=True, text=True,
                          timeout=seconds + 60, check=False)
    if done.returncode != 0:
        raise Failure(f"bombardier failed: {done.stderr.strip()[-300:]}")
    return json.loads(done.stdout)


class Saas:
    """The prepared SaaS starter and, once registered, its session cookie."""

    def __init__(self, project: Path, binary: Path):
        self.project, self.binary = project, binary
        # One key for every start, so the session stays valid between rounds.
        self.settings = dict(SAAS_SETTINGS, APP_KEY=secrets.token_hex(32),
                             BILLING_WEBHOOK_SECRET="whsec_" + secrets.token_hex(24))
        self.cookie: str | None = None


def measure(stack: str, bombardier: str, server_cpus: str, client_cpus: str, out: Path, index: int,
            saas: Saas | None = None) -> dict:
    served = Served(stack, server_cpus, None if saas is None else (saas.project, saas.binary, saas.settings))
    path, cookie = "/json", None
    try:
        if stack == "generated":
            missing = missing_security(get("/json")[1])
            if missing:
                raise Failure("the generated stack's /json lacks " + ", ".join(missing))
        elif stack == "saas" and saas is not None:
            if saas.cookie is None:
                saas.cookie = sign_up()
            path, cookie = DASHBOARD, saas.cookie
            problems = dashboard_problems(*get(DASHBOARD, cookie))
            if problems:
                raise Failure("the signed-in SaaS dashboard lacks " + ", ".join(problems))
        load(bombardier, client_cpus, WARMUP_S, path, cookie)
        before = cpu_seconds(served.process.pid)
        result = load(bombardier, client_cpus, DURATION_S, path, cookie)
        used = cpu_seconds(served.process.pid) - before
    finally:
        served.stop()
    (out / f"{stack}-{index}.json").write_text(json.dumps(result, indent=2))
    return figures(result, used)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--bombardier", default=os.environ.get("BOMBARDIER"), required=False)
    parser.add_argument("--cli", type=Path, required=True, help="cargo-rullst built from this checkout")
    parser.add_argument("--work", type=Path, default=ROOT / "target" / "stack-throughput-work")
    parser.add_argument("--out", type=Path, default=ROOT / "target" / "stack-throughput-results")
    parser.add_argument("--report-only", action="store_true")
    args = parser.parse_args()
    if not args.bombardier:
        parser.error("--bombardier (or BOMBARDIER) is required")
    args.out.mkdir(parents=True, exist_ok=True)
    min_ratios = None if args.report_only else limits(tomllib.loads(BUDGET.read_text()))
    try:
        server_cpus, client_cpus = cpus()
        build()
        saas = Saas(*prepare_saas(args.cli.resolve(), args.work.resolve()))
        rounds = []
        for index in range(1, ROUNDS + 1):
            rounds.append({stack: measure(stack, args.bombardier, server_cpus, client_cpus, args.out, index, saas)
                           for stack in STACKS})
        problems = verdict(rounds, min_ratios)
        cpuinfo = Path("/proc/cpuinfo")
        text = summary(rounds, min_ratios, problems, cpu_model(cpuinfo.read_text() if cpuinfo.exists() else ""))
    except (Failure, DAST.Failure, OSError) as failure:
        problems = [str(failure)]
        text = f"## {TITLE}\n\n**Failed:** {failure}\n"
    print(text)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write(text)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
