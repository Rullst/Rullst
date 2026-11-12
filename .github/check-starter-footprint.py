#!/usr/bin/env python3
"""Check the footprint budgets of the generated Blank, SaaS and API starters.

With the CLI built from this checkout, each starter is generated as
.github/check-starter-browsers.py generates it (framework crates resolved from
the checkout by .github/dast-generated-app.py), built in release mode with its
generated `[profile.release]` and the toolchain of rust-toolchain.toml, and
migrated. `cargo rullst footprint --env staging --page-weight --bench` then
measures it against .github/starter-footprint/<starter>.toml (`--budget`).

Blocking: a limit exceeded; a limit skipped because its figure was not
measured (the budget would not hold); a footprint failure (build, start or
reach); a starter without a budget file. CPU time per request is reported in
the job summary and never budgeted: it varies between shared runners.

The summary lists each figure with its limit and the limit this check
suggests (the measured figure plus its margin in MARGINS, and at least
MIN_HEADROOM). --report-only
measures without budgets, for a first measurement or a new starter.

Usage: check-starter-footprint.py --cli PATH --work DIR [--out DIR]
                                  [--starter blank|saas|api ...] [--report-only]
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import math
import os
from pathlib import Path
import shlex
import shutil
import signal
import subprocess
import sys
import tomllib

from first_hour_checks import Failure, log
from first_hour_process import stream

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
BUDGETS = HERE / "starter-footprint"
TITLE = "Starter footprint budgets"
# The `cargo rullst new --default` arguments of each starter.
STARTERS = {
    "blank": ("--blueprint", "blank"),
    "saas": ("--blueprint", "saas"),
    "api": ("--blueprint", "blank", "--database", "sqlite", "--api"),
}
FOOTPRINT = ("footprint", "--env", "staging", "--page-weight", "--bench", "--duration", "5s", "--json")
BUILD_DEADLINE = 45 * 60
FOOTPRINT_DEADLINE = 15 * 60
# The headroom of a suggested limit over its measured figure. Counts of page
# requests and third-party origins get none: one more is a heavier page.
MARGINS = {
    "binary_size_bytes": 0.15,
    "idle_rss_bytes": 0.20,
    "dependency_count": 0.05,
    "bytes_per_request": 0.10,
    "page_first_visit_bytes": 0.10,
    "page_repeat_visit_bytes": 0.10,
    "page_requests": 0.0,
    "page_third_party_origins": 0.0,
}
# Response bodies of a few kilobytes get at least 1 KiB of headroom, so a
# sentence more in a starter's HTML does not need a new limit.
MIN_HEADROOM = {"bytes_per_request": 1024, "page_first_visit_bytes": 1024, "page_repeat_visit_bytes": 1024}

_SPEC = importlib.util.spec_from_file_location("dast_generated_app", HERE / "dast-generated-app.py")
if _SPEC is None or _SPEC.loader is None:
    raise RuntimeError("cannot load .github/dast-generated-app.py")
DAST = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(DAST)


def budget_path(starter: str) -> Path:
    return BUDGETS / f"{starter}.toml"


def load_budget(path: Path) -> dict[str, float]:
    """The `[footprint.budgets]` limits of `path`, each one this check suggests."""
    limits = tomllib.loads(path.read_text(encoding="utf-8")).get("footprint", {}).get("budgets", {})
    unknown = sorted(set(limits) - set(MARGINS))
    if not limits or unknown:
        raise Failure(f"{path.name}: [footprint.budgets] must set limits from {sorted(MARGINS)}"
                      + (f"; unknown: {unknown}" if unknown else ""))
    return limits


def suggest(name: str, measured: float) -> int:
    """The measured figure plus the limit's margin (at least its minimum headroom), rounded up."""
    return math.ceil(max(round(measured * (1 + MARGINS[name]), 6), measured + MIN_HEADROOM.get(name, 0)))


def figures(report: dict) -> dict[str, float | None]:
    """The budgetable figures of a `rullst.footprint.v1` report (None: not measured)."""
    measured, bench, page = report["measured"], report["benchmark"], report["page_weight"]
    weighed = page["status"] == "measured"
    return {
        "binary_size_bytes": measured["binary_size_bytes"]["value"],
        "idle_rss_bytes": measured["idle_rss_bytes"]["value"],
        "dependency_count": measured["dependency_count"]["value"],
        "bytes_per_request": bench["bytes_per_request"]["value"],
        "page_first_visit_bytes": page["first_visit"]["bytes"],
        "page_repeat_visit_bytes": page["repeat_visit"]["bytes"],
        "page_requests": page["first_visit"]["requests"],
        "page_third_party_origins": len(page["third_party_origins"]) if weighed else None,
    }


def evaluate(starter: str, report: dict, budgeted: bool) -> tuple[list[dict], list[str]]:
    """The summary rows of one starter and what blocks."""
    problems = []
    failure = report.get("failure")
    if failure:
        problems.append(f"{starter}: footprint could not measure the starter ({failure['stage']}): "
                        f"{failure['title']}. {failure['message']}")
    checked = {limit["name"]: limit for limit in report["budgets"]["limits"]}
    if budgeted and report["budgets"]["status"] == "not_checked":
        problems.append(f"{starter}: the budget was not checked")
    rows = []
    for name, measured in figures(report).items():
        limit = checked.get(name)
        row = {"starter": starter, "name": name, "measured": measured,
               "allowed": limit["allowed"] if limit else None,
               "status": limit["status"] if limit else "reported",
               "suggested": None if measured is None else suggest(name, measured)}
        if row["status"] == "exceeded":
            problems.append(f"{starter}: {name} is {show(measured)}, over its limit of {show(row['allowed'])}")
        elif row["status"] == "skipped":
            problems.append(f"{starter}: {name} has a limit but was not measured ({limit['reason']})")
        rows.append(row)
    return rows, problems


def show(value: float | None) -> str:
    if value is None:
        return "not measured"
    return f"{value:,.0f}" if float(value).is_integer() else f"{value:,.2f}"


def status_cell(row: dict) -> str:
    if row["status"] == "passed" and row["allowed"]:
        return f"passed ({row['measured'] / row['allowed']:.0%} of the limit)"
    return {"exceeded": "**EXCEEDED**", "skipped": "**not measured**"}.get(row["status"], row["status"])


def render(reports: dict[str, dict], rows: list[dict], problems: list[str]) -> str:
    first = next(iter(reports.values()), {})
    machine = first.get("machine", {})
    lines = [f"### {TITLE}", "",
             f"`cargo rullst footprint` {first.get('cli_version', '?')} on {machine.get('os', '?')} "
             f"{machine.get('arch', '?')}, {machine.get('logical_cpus', '?')} logical CPUs "
             f"({machine.get('cpu_model') or 'CPU model unknown'}); release builds with each starter's "
             "`[profile.release]`. Limits: `.github/starter-footprint/<starter>.toml`. A suggested limit "
             "is the measured figure plus its margin; copy it into the budget file only for an intended "
             "increase.", "",
             "| Starter | Figure | Measured | Limit | Status | Suggested limit |",
             "| :--- | :--- | ---: | ---: | :--- | ---: |"]
    for row in rows:
        allowed = "—" if row["allowed"] is None else show(row["allowed"])
        suggested = "—" if row["suggested"] is None else show(row["suggested"])
        lines.append(f"| {row['starter']} | `{row['name']}` | {show(row['measured'])} | {allowed} | "
                     f"{status_cell(row)} | {suggested} |")
    lines += ["", "Reported, never budgeted (CPU time varies between shared runners):", "",
              "| Starter | CPU ms per request | Requests in 5 s | HTTP errors | Content-Encoding |",
              "| :--- | ---: | ---: | ---: | :--- |"]
    for starter, report in reports.items():
        bench = report["benchmark"]
        encodings = ", ".join(bench.get("content_encodings") or []) or "—"
        lines.append(f"| {starter} | {show(bench['cpu_ms_per_request']['value'])} | "
                     f"{show(bench.get('requests'))} | {show(bench.get('http_errors'))} | {encodings} |")
    for starter in reports:
        suggested = [row for row in rows if row["starter"] == starter and row["suggested"] is not None]
        lines += ["", f"<details><summary>Suggested budget for {starter}</summary>", "", "```toml",
                  "[footprint.budgets]"]
        lines += [f"{row['name']} = {row['suggested']:_}" for row in suggested]
        lines += ["```", "", "</details>"]
    if problems:
        lines += ["", "#### Blocking", ""] + [f"- {problem}" for problem in problems]
    return "\n".join(lines) + "\n"


def toolchain() -> str:
    return tomllib.loads((ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))["toolchain"]["channel"]


def prepare(cli: Path, work: Path, starter: str, env: dict[str, str]) -> Path:
    """Generates, builds (release) and migrates one starter."""
    project = work / f"footprint-{starter}"
    stream([str(cli), "new", str(project), "--default", *STARTERS[starter], "--skip-initial-migration"],
           cwd=work, env=env)
    shutil.copyfile(ROOT / "Cargo.lock", project / "Cargo.lock")
    DAST.command_registry_patch(project)
    # From the project directory, as footprint builds: its .cargo/config.toml
    # applies, so footprint reuses this build instead of starting another.
    stream(["cargo", "build", "--release", "--offline"], cwd=project, env=env, timeout=BUILD_DEADLINE)
    binary = Path(env["CARGO_TARGET_DIR"]) / "release" / project.name
    stream([str(binary), "db:migrate"], cwd=project, env=env)
    return project


def measure(cli: Path, project: Path, budget: Path | None, env: dict[str, str]) -> tuple[int, dict]:
    """Runs footprint in `project`; its JSON report comes on stdout, progress on stderr."""
    tokens = [str(cli), *FOOTPRINT] + (["--budget", str(budget)] if budget else [])
    log(f"$ {shlex.join(tokens)}")
    process = subprocess.Popen(tokens, cwd=project, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                               text=True, start_new_session=True)
    try:
        stdout, _ = process.communicate(timeout=FOOTPRINT_DEADLINE)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.communicate()
        raise Failure(f"footprint did not finish within {FOOTPRINT_DEADLINE // 60} minutes in {project.name}")
    try:
        report = json.loads(stdout)
    except json.JSONDecodeError as error:
        raise Failure(f"footprint exited with {process.returncode} without a JSON report: {error}") from error
    if report.get("schema") != "rullst.footprint.v1":
        raise Failure(f"footprint wrote schema {report.get('schema')!r}, not rullst.footprint.v1")
    return process.returncode, report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--starter", action="append", choices=sorted(STARTERS))
    parser.add_argument("--report-only", action="store_true", help="measure without the budgets")
    args = parser.parse_args(argv)
    cli, work = args.cli.resolve(), args.work.resolve()
    out = (args.out or work / "reports").resolve()
    work.mkdir(parents=True, exist_ok=True)
    out.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, RULLST_DISABLE_UPDATE_CHECK="true", CARGO_NET_OFFLINE="true")
    for name in ("PORT", "HOST", "RULLST_ENV"):
        env.pop(name, None)
    env.setdefault("CARGO_TARGET_DIR", str(ROOT / "target"))
    reports: dict[str, dict] = {}
    rows: list[dict] = []
    problems: list[str] = []
    try:
        # The pinned toolchain, also outside the checkout (rust-toolchain.toml
        # does not reach the generated projects).
        env["RUSTUP_TOOLCHAIN"] = toolchain()
        for starter in args.starter or list(STARTERS):
            budget = None if args.report_only else budget_path(starter)
            if budget is not None:
                if not budget.is_file():
                    raise Failure(f"{starter} has no budget file {budget.relative_to(ROOT)}")
                load_budget(budget)
            log(f"::group::{starter}: generate, build in release mode and migrate")
            project = prepare(cli, work, starter, env)
            log("::endgroup::")
            code, report = measure(cli, project, budget, env)
            (out / f"{starter}.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
            reports[starter] = report
            starter_rows, starter_problems = evaluate(starter, report, budget is not None)
            rows += starter_rows
            problems += starter_problems
            if code != 0 and not starter_problems:
                problems.append(f"{starter}: footprint exited with {code}")
    except (Failure, DAST.Failure, OSError) as error:
        problems.append(str(error))
    summary = render(reports, rows, problems)
    print(summary)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write(summary)
    for problem in problems:
        log(f"::error title={TITLE}::{problem.splitlines()[0]}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
