#!/usr/bin/env python3
"""Drive the generated Blank and SaaS starters in Chromium, Firefox and WebKit.

With the CLI built from this checkout, the starters are generated as
.github/workflows/dast-zap.yml generates them (framework crates resolved from
the checkout by .github/dast-generated-app.py), built, migrated and started in
turn on port 3000 with their generated .env (development, offline billing).
.github/starter-browsers/starter-browsers.mjs drives each one with the pinned
Playwright and axe-core and records what it observes; this script decides
what blocks:

- a console error, an uncaught page error, a failed same-origin request or a
  Content-Security-Policy violation;
- a keyboard defect (an element Tab never reaches, focus without a visible
  change, focus lost after an HTMX swap) or a failed flow (the HTMX counter,
  and sign-up, sign-in and sign-out through the real forms, submitted with
  Enter and the CSRF double-submit cookie);
- an axe-core violation of `serious` or `critical` impact, or a server panic.

`moderate` and `minor` axe findings are listed in the job summary without
failing. A defect in framework code that a starter cannot fix is versioned in
.github/starter-browsers/known-defects.json with its owner and reason: it is
reported instead of failing, and an entry that nothing matches any more fails
the run, so it is removed together with the fix.

Usage: check-starter-browsers.py --cli PATH --work DIR [--out DIR]
                                 [--starter blank|saas ...] [--browsers LIST]
"""

from __future__ import annotations

import argparse
from collections import Counter
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys

import first_hour_checks as checks
from first_hour_checks import Failure, log
from first_hour_process import Server, stream

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
DRIVER = HERE / "starter-browsers" / "starter-browsers.mjs"
KNOWN_DEFECTS = HERE / "starter-browsers" / "known-defects.json"
STARTERS = ("blank", "saas")
BROWSERS = "chromium,firefox,webkit"
BLOCKING_IMPACTS = {"serious", "critical"}
KNOWN_FIELDS = {"starter", "kind", "key", "owner", "reason"}
COLUMNS = (("Console and requests", {"console", "pageerror", "http", "server"}), ("CSP", {"csp"}),
           ("Keyboard", {"keyboard"}), ("Flows", {"flow"}), ("axe serious/critical", {"axe"}))

_SPEC = importlib.util.spec_from_file_location("dast_generated_app", HERE / "dast-generated-app.py")
if _SPEC is None or _SPEC.loader is None:
    raise RuntimeError("cannot load .github/dast-generated-app.py")
DAST = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(DAST)


def load_known_defects(path: Path = KNOWN_DEFECTS) -> list[dict]:
    entries = json.loads(path.read_text(encoding="utf-8"))
    for entry in entries:
        missing = KNOWN_FIELDS - set(entry)
        if missing or not all(isinstance(entry[field], str) and entry[field] for field in KNOWN_FIELDS):
            raise Failure(f"{path.name}: an entry lacks {sorted(missing) or 'non-empty strings'}: {entry}")
    return entries


def matches(entry: dict, starter: str, finding: dict) -> bool:
    return (entry["starter"] == starter and entry["kind"] == finding["kind"] and entry["key"] == finding["key"]
            and entry.get("page", finding["page"]) == finding["page"]
            and finding["browser"] in entry.get("browsers", [finding["browser"]]))


def classify(finding: dict, csp_pages: set[tuple[str, str]]) -> str:
    """`blocking`, `recorded` (moderate/minor axe) or `covered` (a console line
    for a CSP violation the page's violation event already reports)."""
    if finding["kind"] == "axe":
        return "blocking" if finding.get("impact") in BLOCKING_IMPACTS else "recorded"
    if finding["kind"] == "console" and finding.get("csp") and (finding["browser"], finding["page"]) in csp_pages:
        return "covered"
    return "blocking"


def evaluate_run(starter: str, run: dict, known: list[dict], used: set[int], result: dict) -> None:
    """Sorts one browser's findings into `result` and adds its summary row."""
    csp_pages = {(f["browser"], f["page"]) for f in run["findings"] if f["kind"] == "csp"}
    counts: Counter = Counter()
    for finding in run["findings"]:
        verdict = classify(finding, csp_pages)
        if verdict == "covered":
            continue
        item = {"starter": starter, **finding, "owner": None}
        hit = next((index for index, entry in enumerate(known) if matches(entry, starter, finding)), None)
        if verdict == "blocking" and hit is not None:
            used.add(hit)
            verdict, item["owner"] = "known", known[hit]["owner"]
        result[verdict].append(item)
        counts[(finding["kind"], verdict)] += 1
    result["rows"].append({"starter": starter, "browser": run["browser"], "version": run.get("version"),
                           "steps": run["steps"], "counts": counts,
                           "capture_reports": run.get("capture_reports", 0)})


def evaluate(reports: list[dict], known: list[dict]) -> dict:
    result: dict = {"blocking": [], "known": [], "recorded": [], "stale": [], "rows": []}
    used: set[int] = set()
    for report in reports:
        for run in report["browsers"]:
            evaluate_run(report["starter"], run, known, used, result)
    evaluated = {report["starter"] for report in reports}
    result["stale"] = [entry for index, entry in enumerate(known)
                       if index not in used and entry["starter"] in evaluated]
    return result


def cell(counts: Counter, kinds: set[str]) -> str:
    failed = sum(counts[(kind, "blocking")] for kind in kinds)
    known = sum(counts[(kind, "known")] for kind in kinds)
    if failed:
        return f"FAIL ({failed})"
    return f"known defect ({known})" if known else "pass"


def render(reports: list[dict], result: dict) -> str:
    versions = reports[0]["versions"] if reports else {}
    lines = ["### Generated starters in browsers", "",
             f"Playwright {versions.get('playwright', '?')}, axe-core {versions.get('axe-core', '?')}; "
             "axe `serious`/`critical` violations fail, `moderate`/`minor` are listed below.", "",
             "| Starter | Browser | " + " | ".join(name for name, _ in COLUMNS) + " | axe moderate/minor | Steps |",
             "| :--- | :--- |" + " :--- |" * (len(COLUMNS) + 2)]
    for row in result["rows"]:
        recorded = row["counts"][("axe", "recorded")]
        cells = [cell(row["counts"], kinds) for _, kinds in COLUMNS]
        lines.append(f"| {row['starter']} | {row['browser']} {row['version'] or '(not started)'} | "
                     + " | ".join(cells) + f" | {recorded} | {len(row['steps'])} |")
    sections = (("Blocking findings", result["blocking"]), ("Known framework defects (reported, not failed)", result["known"]),
                ("axe moderate/minor findings (recorded)", result["recorded"]))
    for title, items in sections:
        if items:
            lines += ["", f"#### {title}", ""] + [describe(item) for item in dedupe(items)]
    ignored = [f"{row['starter']}/{row['browser']} {row['capture_reports']}"
               for row in result["rows"] if row["capture_reports"]]
    if ignored:
        lines += ["", "CSP reports raised by Playwright's own screenshot styles (not the starter) were ignored: "
                  + ", ".join(ignored) + "."]
    if result["stale"]:
        lines += ["", "#### Stale known defects (fixed? remove them from known-defects.json)", ""]
        lines += [f"- {entry['starter']} {entry['kind']} `{entry['key']}` ({entry['owner']})" for entry in result["stale"]]
    return "\n".join(lines) + "\n"


def dedupe(items: list[dict]) -> list[dict]:
    """One line per starter/page/kind/key, naming every browser that saw it."""
    merged: dict[tuple, dict] = {}
    for item in items:
        key = (item["starter"], item["page"], item["kind"], item["key"])
        entry = merged.setdefault(key, {**item, "browsers": []})
        if item["browser"] not in entry["browsers"]:
            entry["browsers"].append(item["browser"])
    return list(merged.values())


def describe(item: dict) -> str:
    impact = f" ({item['impact']})" if item.get("impact") else ""
    owner = f" [{item['owner']}]" if item.get("owner") else ""
    detail = item["detail"] if isinstance(item["detail"], str) else json.dumps(item["detail"], sort_keys=True)
    targets = "; ".join(node["target"] for node in item.get("nodes", []))
    where = f" at `{targets}`" if targets else ""
    return (f"- {item['starter']} `{item['page']}` {item['kind']}{impact}{owner}: `{item['key']}` — "
            f"{detail[:300]}{where} ({', '.join(item['browsers'])})")


def prepare(cli: Path, work: Path, starter: str, env: dict[str, str]) -> tuple[Path, Path]:
    project = work / f"browser-{starter}"
    stream([str(cli), "new", str(project), "--default", "--blueprint", starter, "--skip-initial-migration"],
           cwd=work, env=env)
    shutil.copyfile(ROOT / "Cargo.lock", project / "Cargo.lock")
    DAST.command_registry_patch(project)
    stream(["cargo", "build", "--offline", "--manifest-path", str(project / "Cargo.toml")], cwd=ROOT, env=env)
    binary = Path(env["CARGO_TARGET_DIR"]) / "debug" / project.name
    stream([str(binary), "db:migrate"], cwd=project, env=env)
    return project, binary


def drive(project: Path, binary: Path, starter: str, out: Path, browsers: str, env: dict[str, str]) -> dict:
    out.mkdir(parents=True, exist_ok=True)
    server = Server([str(binary)], project, env, out / "server.log")
    try:
        server.wait_until_ready()
        stream(["node", str(DRIVER), "--starter", starter, "--base-url", checks.ORIGIN, "--out", str(out),
                "--browsers", browsers], cwd=ROOT, env=env)
    finally:
        server.stop()
    report = json.loads((out / "report.json").read_text(encoding="utf-8"))
    if "panicked at" in server.text():
        report["browsers"][0]["findings"].append({"browser": report["browsers"][0]["browser"], "page": "(server)",
                                                  "kind": "server", "key": "panic", "detail": server.tail(40)})
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--starter", action="append", choices=STARTERS)
    parser.add_argument("--browsers", default=BROWSERS)
    args = parser.parse_args(argv)
    work, out = args.work.resolve(), (args.out or args.work / "artifacts").resolve()
    work.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, RULLST_DISABLE_UPDATE_CHECK="true")
    env.pop("PORT", None)
    env.setdefault("CARGO_TARGET_DIR", str(ROOT / "target"))
    reports = []
    try:
        known = load_known_defects()
        for starter in args.starter or STARTERS:
            log(f"::group::{starter}: generate, build and migrate")
            project, binary = prepare(args.cli.resolve(), work, starter, env)
            log("::endgroup::")
            reports.append(drive(project, binary, starter, out / starter, args.browsers, env))
    except (Failure, DAST.Failure) as error:
        log(f"::error title=Generated starters in browsers::{error}")
        return 1
    result = evaluate(reports, known)
    summary = render(reports, result)
    print(summary)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write(summary)
    for item in dedupe(result["blocking"])[:20]:
        log(f"::error title=Generated starters in browsers::{describe(item)[2:]}")
    for entry in result["stale"]:
        log(f"::error title=Stale known defect::{entry['starter']} {entry['kind']} {entry['key']} no longer occurs")
    return 1 if result["blocking"] or result["stale"] else 0


if __name__ == "__main__":
    sys.exit(main())
