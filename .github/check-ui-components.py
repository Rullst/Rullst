#!/usr/bin/env python3
"""Copy every `cargo rullst ui add` component into a fixture application and
drive it in Chromium, Firefox and WebKit.

.github/fixtures/ui-gallery is copied out of the repository, its `rullst`
dependency resolved from this checkout as .github/check-starter-browsers.py
resolves the starters', and the gallery page that cargo-rullst's tests
compile (cargo-rullst/src/generators/components/gallery_fixture.rs) placed
in its src/. Then, with the CLI built from this checkout:

1. `cargo rullst ui add --all` copies the components, registers them and
   declares `mod ui;`; a second run must change nothing;
2. `cargo rullst add tailwind` downloads the pinned, checksum-verified
   Tailwind CLI and builds static/app.css from src/**/*.rs, src/ui/ included;
3. the gallery is built, started on port 3000 and driven by
   .github/starter-browsers/ui-components.mjs (pinned Playwright and
   axe-core) in the light and the dark colour scheme.

A console error, a failed same-origin request, a Content-Security-Policy
violation, a keyboard defect, a failed flow or an axe-core violation of
`serious` or `critical` impact fails the run, with the rules and summary of
check-starter-browsers.py.

Usage: check-ui-components.py --cli PATH --work DIR [--out DIR] [--browsers LIST]
"""

from __future__ import annotations

import argparse
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
FIXTURE = HERE / "fixtures" / "ui-gallery"
GALLERY = ROOT / "cargo-rullst/src/generators/components/gallery_fixture.rs"
HTMX = ROOT / "cargo-rullst/src/blueprints/assets/htmx-2.0.11.min.js"
FAVICON = ROOT / "cargo-rullst/src/blueprints/blank/rullst.png"
DRIVER = HERE / "starter-browsers" / "ui-components.mjs"
NAME = "ui-components"


def load(name: str, file: str):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load .github/{file}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


STARTERS = load("check_starter_browsers", "check-starter-browsers.py")
DAST = STARTERS.DAST


def prepare(cli: Path, work: Path, env: dict[str, str]) -> tuple[Path, Path]:
    project = work / "ui-gallery"
    if project.exists():
        shutil.rmtree(project)
    shutil.copytree(FIXTURE, project)
    template = project / "Cargo.toml.template"
    manifest = template.read_text(encoding="utf-8").replace("__RULLST_PATH__", str(ROOT / "rullst"))
    (project / "Cargo.toml").write_text(manifest, encoding="utf-8")
    template.unlink()
    shutil.copyfile(GALLERY, project / "src" / "gallery.rs")
    shutil.copyfile(HTMX, project / "static" / HTMX.name)
    shutil.copyfile(FAVICON, project / "static" / "rullst.png")
    shutil.copyfile(ROOT / "Cargo.lock", project / "Cargo.lock")
    DAST.command_registry_patch(project)

    stream([str(cli), "ui", "add", "--all"], cwd=project, env=env)
    for path in ("src/ui/mod.rs", "src/ui/dialog.rs", "static/ui/dialog.js", "static/ui/toast.js"):
        checks.expect((project / path).is_file(), f"ui add --all wrote {path}")
    checks.expect("mod ui;" in (project / "src/main.rs").read_text(encoding="utf-8"),
                  "ui add --all declared `mod ui;` in src/main.rs")
    again = stream([str(cli), "ui", "add", "--all"], cwd=project, env=env)
    checks.expect("Nothing to change" in again.output, "a second ui add --all changes nothing")

    stream([str(cli), "add", "tailwind"], cwd=project, env=env)
    stylesheet = (project / "static" / "app.css").read_text(encoding="utf-8")
    for candidate in ("bg-sky-700", "aria-selected", "htmx-request", "backdrop"):
        checks.expect(candidate in stylesheet, f"static/app.css has the components' {candidate} rules")

    stream(["cargo", "build", "--offline", "--manifest-path", str(project / "Cargo.toml")], cwd=ROOT, env=env)
    return project, Path(env["CARGO_TARGET_DIR"]) / "debug" / "ui-gallery"


def drive(project: Path, binary: Path, out: Path, browsers: str, env: dict[str, str]) -> dict:
    out.mkdir(parents=True, exist_ok=True)
    server = Server([str(binary)], project, env, out / "server.log")
    try:
        server.wait_until_ready()
        stream(["node", str(DRIVER), "--base-url", checks.ORIGIN, "--out", str(out), "--browsers", browsers],
               cwd=ROOT, env=env)
    finally:
        server.stop()
    report = json.loads((out / "report.json").read_text(encoding="utf-8"))
    if "panicked at" in server.text():
        first = report["browsers"][0]
        first["findings"].append({"browser": first["browser"], "page": "(server)", "kind": "server",
                                  "key": "panic", "detail": server.tail(40)})
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--browsers", default=STARTERS.BROWSERS)
    args = parser.parse_args(argv)
    work, out = args.work.resolve(), (args.out or args.work / "artifacts").resolve()
    work.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, RULLST_DISABLE_UPDATE_CHECK="true")
    env.pop("PORT", None)
    env.setdefault("CARGO_TARGET_DIR", str(ROOT / "target"))
    try:
        log("::group::ui-gallery: copy the components, add Tailwind, build")
        project, binary = prepare(args.cli.resolve(), work, env)
        log("::endgroup::")
        report = drive(project, binary, out, args.browsers, env)
    except (Failure, DAST.Failure) as error:
        log(f"::error title=UI components in browsers::{error}")
        return 1
    result = STARTERS.evaluate([report], STARTERS.load_known_defects())
    summary = STARTERS.render([report], result).replace(
        "### Generated starters in browsers", "### UI components (cargo rullst ui add) in browsers", 1)
    print(summary)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write(summary)
    for item in STARTERS.dedupe(result["blocking"])[:20]:
        log(f"::error title=UI components in browsers::{STARTERS.describe(item)[2:]}")
    return 1 if result["blocking"] else 0


if __name__ == "__main__":
    sys.exit(main())
