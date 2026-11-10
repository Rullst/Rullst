#!/usr/bin/env python3
"""Incremental compile time of `t!` calls on a synthetic catalog.

    python3 .github/i18n-compile-bench.py --work DIR [--rullst PATH]
        [--keys 250] [--locales 3] [--calls 400] [--runs 5]

Generates a package in DIR that depends on PATH/rullst-core (default: this
checkout) with --keys keys in each of --locales catalogs (one in five a
plural message, half of the others with a `{name}` placeholder) and --calls
`t!` calls spread over eight modules. It runs `cargo check` once, then edits
one module before each of --runs timed `cargo check`s and prints every time
and the median. The default size is the one an application reported for
13.0.0-alpha.4: 400 calls, about 250 keys and 3 locales (34 KB of TOML).

Set CARGO_TARGET_DIR to share one target directory between checkouts; the
package copies the checkout's Cargo.lock and builds offline.
"""

from __future__ import annotations

import argparse
import shutil
import statistics
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOCALES = ["pt-BR", "en", "es", "fr", "de", "it"]
MODULES = 8


def key(index: int) -> str:
    return f"section{index // 25}.message{index % 25}"


def plural(index: int) -> bool:
    return index % 5 == 0


def named(index: int) -> bool:
    return not plural(index) and index % 2 == 1


def catalog(tag: str, keys: int) -> str:
    sections: dict[str, list[str]] = {}
    for index in range(keys):
        section, name = key(index).split(".")
        filler = f"{tag} message number {index}"
        if plural(index):
            body = (f"[{section}.{name}]\none = \"one item: {filler}\"\n"
                    f"other = \"{{count}} items: {filler}\"\n")
            sections.setdefault(f"~{section}.{name}", []).append(body)
        elif named(index):
            sections.setdefault(section, []).append(f"{name} = \"Hello {{name}}, {filler}\"")
        else:
            sections.setdefault(section, []).append(f"{name} = \"{filler}\"")
    out = []
    for section, lines in sections.items():
        if section.startswith("~"):
            out.extend(lines)
        else:
            out.append(f"[{section}]\n" + "\n".join(lines) + "\n")
    return "\n".join(out)


def call(index: int) -> str:
    if plural(index):
        return f't!(locale, "{key(index)}", count = {index})'
    if named(index):
        return f't!(locale, "{key(index)}", name = "Ana")'
    return f't!(locale, "{key(index)}")'


def generate(work: Path, rullst: Path, keys: int, locales: int, calls: int) -> None:
    if work.exists():
        shutil.rmtree(work)
    (work / "src").mkdir(parents=True)
    (work / "locales").mkdir()
    core = (rullst / "rullst-core").as_posix()
    (work / "Cargo.toml").write_text(
        "[package]\nname = \"i18n-bench\"\nversion = \"0.0.0\"\nedition = \"2024\"\n"
        f"publish = false\n\n[dependencies]\nrullst-core = {{ path = '{core}' }}\n\n[workspace]\n")
    shutil.copy(rullst / "Cargo.lock", work / "Cargo.lock")
    for tag in LOCALES[:locales]:
        (work / "locales" / f"{tag}.toml").write_text(catalog(tag, keys))
    per_module = -(-calls // MODULES)
    for module in range(MODULES):
        indexes = range(module * per_module, min(calls, (module + 1) * per_module))
        body = "".join(f"        {call(i % keys)}.into_owned(),\n" for i in indexes)
        (work / "src" / f"page{module}.rs").write_text(
            "#[allow(unused_imports)]\nuse crate::i18n::t;\n\n#[allow(dead_code)]\npub const EDIT: u32 = 0;\n\n"
            "pub fn render(locale: rullst::i18n::Locale) -> Vec<String> {\n"
            f"    vec![\n{body}    ]\n}}\n")
    modules = "".join(f"mod page{m};\n" for m in range(MODULES))
    renders = " + ".join(f"page{m}::render(locale).len()" for m in range(MODULES))
    (work / "src" / "main.rs").write_text(
        "extern crate rullst_core as rullst;\n\nmod i18n {\n"
        "    rullst::i18n::catalog!(default = \"pt-BR\");\n}\n" + modules +
        "\nfn main() {\n    let locale = i18n::LOCALES.default_locale();\n"
        f"    println!(\"{{}}\", {renders});\n}}\n")


def check(work: Path) -> float:
    start = time.monotonic()
    subprocess.run(["cargo", "check", "--offline", "--quiet"], cwd=work, check=True)
    return time.monotonic() - start


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--rullst", type=Path, default=ROOT)
    parser.add_argument("--keys", type=int, default=250)
    parser.add_argument("--locales", type=int, default=3, choices=range(1, len(LOCALES) + 1))
    parser.add_argument("--calls", type=int, default=400)
    parser.add_argument("--runs", type=int, default=5)
    args = parser.parse_args()
    generate(args.work, args.rullst.resolve(), args.keys, args.locales, args.calls)
    toml = sum(p.stat().st_size for p in (args.work / "locales").iterdir())
    print(f"{args.keys} keys x {args.locales} locales ({toml // 1024} KiB of TOML), "
          f"{args.calls} t! calls, rullst at {args.rullst}")
    print(f"first check: {check(args.work):.2f} s")
    page = args.work / "src" / "page0.rs"
    times = []
    for run in range(1, args.runs + 1):
        page.write_text(page.read_text().replace(f"EDIT: u32 = {run - 1};", f"EDIT: u32 = {run};"))
        times.append(check(args.work))
    print("incremental checks: " + ", ".join(f"{t:.2f} s" for t in times))
    print(f"median: {statistics.median(times):.2f} s")


if __name__ == "__main__":
    main()
