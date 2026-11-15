#!/usr/bin/env python3
"""Check the throughput of a generated application's production stack.

Builds .github/stack-throughput (the router, layers and release profile of a
`cargo rullst new` application, against this checkout) and serves `/json`
two ways on one machine: `STACK=generated` (`Server::new(router()).run()`
with `RULLST_ENV=production`: the generated router's CSRF and header layers,
the production security baseline and the access log, writing to a pipe like
a container log driver) and `STACK=minimal` (the same routes through
`axum::serve`, without any layer). The server runs on two CPUs and the pinned
load generator (bombardier, as Rullst/Benchmarks uses it) on two others.
Rounds alternate the two stacks; the medians give the ratio.

Blocking: a generated/minimal ratio of requests per second below `min_ratio`
in .github/stack-throughput/budget.toml; a non-2xx response; a generated
response without the production security headers or the CSRF cookie (the
ratio would not measure the stack); a server that does not start; fewer than
four usable CPUs. Absolute figures are reported, never limited: they vary
between shared runners.

Usage: check-stack-throughput.py --bombardier PATH [--out DIR] [--report-only]
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import statistics
import subprocess
import sys
import threading
import time
import tomllib
import urllib.error
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
# Headers the production stack must add to `/json`.
SECURITY_HEADERS = (
    "content-security-policy",
    "strict-transport-security",
    "x-frame-options",
    "x-content-type-options",
    "cross-origin-opener-policy",
    "referrer-policy",
)


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


def ratio(rounds: list[dict]) -> float:
    """Median generated requests per second over median minimal ones."""
    generated = statistics.median(r["generated"]["rps"] for r in rounds)
    minimal = statistics.median(r["minimal"]["rps"] for r in rounds)
    return generated / minimal if minimal > 0 else 0.0


def verdict(rounds: list[dict], min_ratio: float | None) -> list[str]:
    """The blocking problems of `rounds` under `min_ratio` (none: report only)."""
    problems = []
    for index, measured in enumerate(rounds, 1):
        for stack in ("minimal", "generated"):
            if measured[stack]["errors"]:
                problems.append(f"round {index}: {measured[stack]['errors']} non-2xx responses from the {stack} stack")
    if min_ratio is not None and ratio(rounds) < min_ratio:
        problems.append(f"generated/minimal ratio {ratio(rounds):.3f} is below the limit {min_ratio:.3f}")
    return problems


def missing_security(headers: dict[str, str]) -> list[str]:
    """What a generated `/json` response lacks of the production stack."""
    names = {name.lower(): value for name, value in headers.items()}
    missing = [name for name in SECURITY_HEADERS if name not in names]
    if not names.get("set-cookie", "").startswith("rullst_csrf="):
        missing.append("set-cookie: rullst_csrf")
    return missing


def cpu_model(cpuinfo: str) -> str:
    """The first `model name` of /proc/cpuinfo text: ratios differ between runner models."""
    for line in cpuinfo.splitlines():
        name, _, value = line.partition(":")
        if name.strip() == "model name":
            return value.strip()
    return "unknown CPU"


def summary(rounds: list[dict], min_ratio: float | None, problems: list[str], cpu: str = "unknown CPU") -> str:
    """The job summary in Markdown."""
    lines = [f"## {TITLE}", "", f"CPU: {cpu}", ""]
    lines.append("| Round | Stack | Requests/s | p99 (ms) | Server CPU µs/request | Server CPUs |")
    lines.append("| ---: | :--- | ---: | ---: | ---: | ---: |")
    for index, measured in enumerate(rounds, 1):
        for stack in ("minimal", "generated"):
            f = measured[stack]
            lines.append(f"| {index} | {stack} | {f['rps']:.0f} | {f['p99_ms']:.2f} | "
                         f"{f['cpu_us_per_request']:.2f} | {f['server_cpus']:.2f} |")
    measured_ratio = ratio(rounds)
    lines += ["", f"Generated/minimal ratio (medians): **{measured_ratio:.3f}**"]
    if min_ratio is not None:
        lines.append(f"Limit: {min_ratio:.3f}. Suggested limit: {measured_ratio * (1 - MARGIN):.3f}.")
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


def cpu_seconds(pid: int) -> float:
    fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
    return (int(fields[11]) + int(fields[12])) / os.sysconf("SC_CLK_TCK")


def get(path: str) -> tuple[int, dict[str, str]]:
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{PORT}{path}", timeout=2) as response:
            return response.status, {name: value for name, value in response.getheaders()}
    except urllib.error.HTTPError as error:
        return error.code, dict(error.headers.items())


class Served:
    """One server process on the server CPUs, its stdout read and discarded."""

    def __init__(self, stack: str, server_cpus: str):
        env = dict(os.environ, RULLST_ENV="production", HOST="127.0.0.1", PORT=str(PORT), STACK=stack)
        env.pop("RULLST_ACCESS_LOG", None)
        self.process = subprocess.Popen(["taskset", "-c", server_cpus, str(BINARY)], cwd=APP, env=env,
                                        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        self.reader = threading.Thread(target=self._drain, daemon=True)
        self.reader.start()
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise Failure(f"the {stack} server exited with {self.process.returncode}")
            try:
                if get("/json")[0] == 200:
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


def load(bombardier: str, client_cpus: str, seconds: int) -> dict:
    command = ["taskset", "-c", client_cpus, bombardier, "--fasthttp", "-c", str(CONNECTIONS),
               "-d", f"{seconds}s", "-t", "5s", "-l", "-p", "r", "-o", "j", f"http://127.0.0.1:{PORT}/json"]
    done = subprocess.run(command, capture_output=True, text=True, timeout=seconds + 60, check=False)
    if done.returncode != 0:
        raise Failure(f"bombardier failed: {done.stderr.strip()[-300:]}")
    return json.loads(done.stdout)


def measure(stack: str, bombardier: str, server_cpus: str, client_cpus: str, out: Path, index: int) -> dict:
    served = Served(stack, server_cpus)
    try:
        if stack == "generated":
            missing = missing_security(get("/json")[1])
            if missing:
                raise Failure("the generated stack's /json lacks " + ", ".join(missing))
        load(bombardier, client_cpus, WARMUP_S)
        before = cpu_seconds(served.process.pid)
        result = load(bombardier, client_cpus, DURATION_S)
        used = cpu_seconds(served.process.pid) - before
    finally:
        served.stop()
    (out / f"{stack}-{index}.json").write_text(json.dumps(result, indent=2))
    return figures(result, used)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--bombardier", default=os.environ.get("BOMBARDIER"), required=False)
    parser.add_argument("--out", type=Path, default=ROOT / "target" / "stack-throughput-results")
    parser.add_argument("--report-only", action="store_true")
    args = parser.parse_args()
    if not args.bombardier:
        parser.error("--bombardier (or BOMBARDIER) is required")
    args.out.mkdir(parents=True, exist_ok=True)
    min_ratio = None if args.report_only else tomllib.loads(BUDGET.read_text())["throughput"]["min_ratio"]
    try:
        server_cpus, client_cpus = cpus()
        build()
        rounds = []
        for index in range(1, ROUNDS + 1):
            rounds.append({stack: measure(stack, args.bombardier, server_cpus, client_cpus, args.out, index)
                           for stack in ("minimal", "generated")})
        problems = verdict(rounds, min_ratio)
        cpuinfo = Path("/proc/cpuinfo")
        text = summary(rounds, min_ratio, problems, cpu_model(cpuinfo.read_text() if cpuinfo.exists() else ""))
    except Failure as failure:
        problems = [str(failure)]
        text = f"## {TITLE}\n\n**Failed:** {failure}\n"
    print(text)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write(text)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
