#!/usr/bin/env python3
"""Open the Scalar `/docs` page of a generated application in Chromium, in production.

With the CLI built from this checkout, a Blank starter is generated (framework
crates resolved from the checkout by .github/dast-generated-app.py), given
`cargo rullst make:scalar` with its controller mounted on the router and an
`openapi.json` with one operation, built, migrated and started with
`RULLST_ENV=production`: the security baseline and its strict default
Content-Security-Policy are in force, and `/docs` answers with a policy of its
own, which the baseline keeps. .github/starter-browsers/scalar-docs.mjs then
loads the page with the pinned Playwright's Chromium, which fetches the pinned
Scalar file from jsDelivr. It fails on a `securitypolicyviolation` event, a
console error, an uncaught page error or a failed same-origin response, and
unless the document's operation is visible.

The policy admits the style elements Scalar inserts by their hashes, measured
with Scalar 1.67.0, so a Scalar upgrade whose styles differ fails here until
rullst-core/src/scalar.rs is updated.

Usage: check-scalar-docs-browser.py --cli PATH --work DIR [--out DIR]
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
from pathlib import Path
import secrets
import shutil
import sys

import first_hour_checks as checks
from first_hour_checks import Failure, log
from first_hour_process import Server, stream

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
DRIVER = HERE / "starter-browsers" / "scalar-docs.mjs"
PROJECT = "scalar-docs"
# The Blank router's last line, after which the docs controller is mounted.
ROUTER_ANCHOR = ".merge_axum(rullst::health::health_router())"
OPERATION = "List the smoke widgets"
# `scalar_docs_router` serves this file from the working directory.
OPENAPI = {
    "openapi": "3.1.0",
    "info": {"title": "Scalar smoke API", "version": "1.0.0"},
    "paths": {
        "/widgets": {
            "get": {
                "operationId": "listSmokeWidgets",
                "summary": OPERATION,
                "responses": {"200": {"description": "The widgets"}},
            }
        }
    },
}

_SPEC = importlib.util.spec_from_file_location("dast_generated_app", HERE / "dast-generated-app.py")
if _SPEC is None or _SPEC.loader is None:
    raise RuntimeError("cannot load .github/dast-generated-app.py")
DAST = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(DAST)


def mount_docs(project: Path) -> None:
    """Mounts `controllers::docs_controller::router()`, as make:scalar asks,
    on the router of src/main.rs (src/lib.rs with hot reload)."""
    sources = [project / "src" / name for name in ("main.rs", "lib.rs")]
    holders = [path for path in sources if path.is_file() and ROUTER_ANCHOR in path.read_text()]
    if len(holders) != 1 or holders[0].read_text().count(ROUTER_ANCHOR) != 1:
        raise Failure(f"the Blank router no longer ends with `{ROUTER_ANCHOR}` once")
    source = holders[0]
    text = source.read_text().replace(
        ROUTER_ANCHOR, f"{ROUTER_ANCHOR}\n    .merge_axum(controllers::docs_controller::router())")
    if not any(line.strip() in ("mod controllers;", "pub mod controllers;") for line in text.splitlines()):
        text = "mod controllers;\n" + text
    source.write_text(text)
    log(f"mounted docs_controller::router() in {source.relative_to(project)}")


def prepare(cli: Path, work: Path, env: dict[str, str]) -> tuple[Path, Path]:
    project = work / PROJECT
    if project.exists():
        shutil.rmtree(project)
    stream([str(cli), "new", str(project), "--default", "--blueprint", "blank", "--skip-initial-migration"],
           cwd=work, env=env)
    shutil.copyfile(ROOT / "Cargo.lock", project / "Cargo.lock")
    DAST.command_registry_patch(project)
    stream([str(cli), "make:scalar"], cwd=project, env=env)
    mount_docs(project)
    (project / "openapi.json").write_text(json.dumps(OPENAPI, indent=2) + "\n", encoding="utf-8")
    stream(["cargo", "build", "--offline", "--manifest-path", str(project / "Cargo.toml")], cwd=ROOT, env=env)
    binary = Path(env["CARGO_TARGET_DIR"]) / "debug" / project.name
    stream([str(binary), "db:migrate"], cwd=project, env=env)
    return project, binary


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    args = parser.parse_args(argv)
    work, out = args.work.resolve(), (args.out or args.work / "artifacts").resolve()
    work.mkdir(parents=True, exist_ok=True)
    out.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, RULLST_DISABLE_UPDATE_CHECK="true")
    env.pop("PORT", None)
    env.setdefault("CARGO_TARGET_DIR", str(ROOT / "target"))
    try:
        log("::group::Generate a Blank starter with make:scalar, build and migrate")
        project, binary = prepare(args.cli.resolve(), work, env)
        log("::endgroup::")
        production = dict(env, RULLST_ENV="production", APP_KEY=secrets.token_hex(32))
        server = Server([str(binary)], project, production, out / "server.log")
        try:
            server.wait_until_ready()
            stream(["node", str(DRIVER), "--base-url", checks.ORIGIN, "--operation", OPERATION,
                    "--out", str(out)], cwd=ROOT, env=env)
        finally:
            server.stop()
        if "panicked at" in server.text():
            raise Failure(f"the application panicked:\n{server.tail(40)}")
    except Failure as error:
        log(f"::error title=Scalar /docs in Chromium::{error}")
        return 1
    report = json.loads((out / "report.json").read_text(encoding="utf-8"))
    summary = (f"### Scalar `/docs` in Chromium (production)\n\nChromium {report.get('chromium')}, "
               f"Playwright {report.get('playwright')}: no CSP violation, no console error, "
               f"and `{OPERATION}` visible.\n")
    print(summary)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write(summary)
    return 0


if __name__ == "__main__":
    sys.exit(main())
