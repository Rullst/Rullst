#!/usr/bin/env python3
"""Assemble the https://rullst.win/ root site from this repository's sources.

    python3 .github/assemble-organization-site.py --out DIRECTORY [--benchmarks FILE]

Writes the English landing, privacy and comparison pages (/, /privacy.html,
/compare/), their Brazilian Portuguese versions under pt-br/ (/pt-br/comparar/
for the comparison) and the shared assets (stylesheet, script and
language-switcher flags) under assets/ into DIRECTORY, which must be missing
or empty.

--benchmarks names a downloaded copy (inside the working directory, at most
1 MiB) of the Rullst/Benchmarks latest-results JSON (schema "rullst.benchmarks.latest.v1", described in
docs/src/website-maintenance.md). Its Rullst, Loco and Topcoat rows are
rendered into the comparison pages; without the file, or when it is invalid,
the pages keep their "first measurement in progress" text and the script
prints a warning instead of failing. The pages also reference /Rullst/book/, /Rullst/images/ and
/Rullst/Rullst.png, which this repository's pages.yml deploys, so every path
is valid at the site root.

The Rullst/Rullst.github.io deployment workflow runs this script from a
read-only checkout of this repository's main branch; it reads no secret and
never commits, pushes or deploys anything itself. validate-site.py runs it too.
"""

from __future__ import annotations

import argparse
from datetime import date
from hashlib import sha256
from html import escape
import json
import math
import os
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"
SITE_URL = "https://rullst.win/"

# Published path -> source path, relative to the repository root.
FILES = {
    "index.html": "docs/home_template.html",
    "privacy.html": "docs/privacy_template.html",
    "pt-br/index.html": "docs/home_template.pt-BR.html",
    "pt-br/privacy.html": "docs/privacy_template.pt-BR.html",
    "compare/index.html": "docs/compare_template.html",
    "pt-br/comparar/index.html": "docs/compare_template.pt-BR.html",
    "assets/site.css": "docs/site.css",
    "assets/site.js": "docs/site.js",
    "assets/flags/us.svg": "docs/flags/us.svg",
    "assets/flags/br.svg": "docs/flags/br.svg",
}
CANONICAL = {
    "index.html": SITE_URL,
    "privacy.html": SITE_URL + "privacy.html",
    "pt-br/index.html": SITE_URL + "pt-br/",
    "pt-br/privacy.html": SITE_URL + "pt-br/privacy.html",
    "compare/index.html": SITE_URL + "compare/",
    "pt-br/comparar/index.html": SITE_URL + "pt-br/comparar/",
}
LANDINGS = ("index.html", "pt-br/index.html")
ASSET_REFERENCE = re.compile(r'(?:href|src)="/assets/(site\.(?:css|js))\?v=([0-9a-f]{16})"')
LOCAL_ASSET = re.compile(r'(?:href|src)="/(assets/[^"?#]+)')


COMPARE_PAGES = {"compare/index.html": "en", "pt-br/comparar/index.html": "pt-BR"}
BENCHMARK_START = "<!-- benchmark-results:start -->"
BENCHMARK_END = "<!-- benchmark-results:end -->"
BENCHMARK_SCHEMA = "rullst.benchmarks.latest.v1"
COMPARED = ("Rullst", "Loco", "Topcoat")
# Result key -> English label, Portuguese label, unit, decimals.
METRICS = {
    "json_rps": ("JSON responses per second", "Respostas JSON por segundo", "", 0),
    "db_single_rps": ("Responses with one database query per second", "Respostas com uma consulta ao banco por segundo", "", 0),
    "avg_latency_ms": ("Average latency", "Latência média", " ms", 2),
    "peak_ram_mib": ("Peak memory", "Pico de memória", " MiB", 1),
}
MONTHS = {
    "en": ("January", "February", "March", "April", "May", "June", "July", "August",
           "September", "October", "November", "December"),
    "pt-BR": ("janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto",
              "setembro", "outubro", "novembro", "dezembro"),
}
TEXT = {
    "en": {
        "measured": "Measured on {date} with the Rullst/Benchmarks suite. The rows update automatically after each new Rullst release; this page is rebuilt daily.",
        "missing": "Not in this measurement run.",
        "version": "version",
    },
    "pt-BR": {
        "measured": "Medido em {date} com a suíte Rullst/Benchmarks. As linhas se atualizam automaticamente a cada nova versão do Rullst; esta página é reconstruída diariamente.",
        "missing": "Fora desta rodada de medição.",
        "version": "versão",
    },
}
VERSION = re.compile(r"[0-9A-Za-z][0-9A-Za-z.+-]{0,39}")


class AssemblyError(Exception):
    """The sources cannot produce a consistent root site."""


BENCHMARK_LIMIT = 1024 * 1024


def read_benchmark_file(path: Path) -> object:
    """Parse a downloaded results file; it must be a small regular file in the working directory."""
    base = os.path.realpath(os.getcwd())
    resolved = os.path.realpath(path)
    if os.path.commonpath([base, resolved]) != base:
        raise ValueError("the results file must be inside the working directory")
    if not os.path.isfile(resolved) or os.path.getsize(resolved) > BENCHMARK_LIMIT:
        raise ValueError("the results file must be a regular file of at most 1 MiB")
    with open(resolved, encoding="utf-8") as handle:
        return json.load(handle)


def checked_metric(name: str, key: str, raw: object) -> float:
    value = float(raw)  # type: ignore[arg-type]
    if not math.isfinite(value) or value < 0:
        raise ValueError(f"{name}: {key} is not a finite, non-negative number")
    return value


def benchmark_row(result: dict) -> tuple[str, dict] | None:
    """The compared framework a result belongs to and its checked values, or None."""
    name = str(result["framework"])
    match = next((known for known in COMPARED if known.lower() == name.lower()), None)
    if match is None:
        return None
    version = str(result["version"])
    if not VERSION.fullmatch(version):
        raise ValueError(f"unexpected version for {name}: {version!r}")
    metrics = {key: checked_metric(name, key, result[key]) for key in METRICS if result.get(key) is not None}
    if not metrics:
        raise ValueError(f"{name}: no known metric")
    return match, {"version": version, "metrics": metrics}


def load_benchmarks(path: Path) -> dict | None:
    """Return the measured date and the compared frameworks' rows, or None.

    Every value is checked before it reaches a page: an unknown schema, a bad
    date, a non-finite or negative number or an unexpected version string
    discards the whole file.
    """
    try:
        data = read_benchmark_file(path)
        if not isinstance(data, dict) or data.get("schema") != BENCHMARK_SCHEMA:
            raise ValueError(f"schema is not {BENCHMARK_SCHEMA}")
        measured_on = date.fromisoformat(str(data["measured_on"]))
        rows = dict(row for row in map(benchmark_row, data["results"]) if row is not None)
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"warning: benchmark results not used ({error})", file=sys.stderr)
        return None
    if not rows:
        print("warning: benchmark results name none of the compared frameworks", file=sys.stderr)
        return None
    return {"measured_on": measured_on, "rows": rows}


def format_number(value: float, decimals: int, lang: str) -> str:
    text = f"{value:,.{decimals}f}"
    return text.translate(str.maketrans(",.", ".,")) if lang == "pt-BR" else text


def render_benchmarks(results: dict, lang: str) -> str:
    """HTML for the comparison page's results block, escaped throughout."""
    measured = results["measured_on"]
    month = MONTHS[lang][measured.month - 1]
    day = f"{measured.day} de {month} de {measured.year}" if lang == "pt-BR" else f"{measured.day} {month} {measured.year}"
    when = f'<time datetime="{measured.isoformat()}">{escape(day)}</time>'
    lines = [f'<p class="benchmark-meta">{TEXT[lang]["measured"].format(date=when)}</p>', '<ul class="benchmark-rows">']
    for name in COMPARED:
        row = results["rows"].get(name)
        if row is None:
            lines.append(f'<li><h3>{name}</h3><p>{TEXT[lang]["missing"]}</p></li>')
            continue
        metrics = "".join(
            f'<div><dt>{escape(METRICS[key][0 if lang == "en" else 1])}</dt>'
            f'<dd>{format_number(value, METRICS[key][3], lang)}{METRICS[key][2]}</dd></div>'
            for key, value in row["metrics"].items()
        )
        lines.append(
            f'<li><h3>{name} <span>{TEXT[lang]["version"]} {escape(row["version"])}</span></h3><dl>{metrics}</dl></li>'
        )
    lines.append("</ul>")
    return "\n".join(lines)


def check_page(page: str, text: str, digests: dict[str, str]) -> None:
    """One page's canonical URL, CSP and asset references."""
    source = FILES[page]
    if f'<link rel="canonical" href="{CANONICAL[page]}">' not in text:
        raise AssemblyError(f"{source} must declare the canonical URL {CANONICAL[page]}")
    if "Content-Security-Policy" not in text:
        raise AssemblyError(f"{source} must declare a Content-Security-Policy")
    unpublished = [asset for asset in LOCAL_ASSET.findall(text) if asset not in FILES]
    if unpublished:
        raise AssemblyError(f"{source} references /{unpublished[0]}, which the root site does not publish")
    references = ASSET_REFERENCE.findall(text)
    if not references:
        raise AssemblyError(f"{source} references no content-versioned asset")
    stale = [(name, version) for name, version in references if version != digests[name]]
    if stale:
        name, version = stale[0]
        raise AssemblyError(f"{source} references {name}?v={version}; the current content version is {digests[name]}")
    required = ("site.css", "site.js") if page in LANDINGS else ()
    missing = [name for name in required if f"/assets/{name}?v={digests[name]}" not in text]
    if missing:
        raise AssemblyError(f"{source} must load /assets/{missing[0]}?v={digests[missing[0]]}")
    if page in COMPARE_PAGES and not (
        text.count(BENCHMARK_START) == 1 == text.count(BENCHMARK_END) and text.index(BENCHMARK_START) < text.index(BENCHMARK_END)
    ):
        raise AssemblyError(f"{source} needs one benchmark-results block")


def check_sources(root: Path = ROOT) -> dict[str, bytes]:
    """Read every source and check the references the deployment relies on."""
    contents = {}
    for published, source in FILES.items():
        path = root / source
        if not path.is_file() or path.is_symlink():
            raise AssemblyError(f"missing regular source file: {source}")
        contents[published] = path.read_bytes()
    digests = {name: sha256(contents[f"assets/{name}"]).hexdigest()[:16] for name in ("site.css", "site.js")}
    for page in CANONICAL:
        check_page(page, contents[page].decode("utf-8"), digests)
    return contents


def assemble(out: Path, root: Path = ROOT, benchmarks: Path | None = None) -> list[str]:
    """Write the root site into OUT and return the published paths."""
    contents = check_sources(root)
    results = load_benchmarks(benchmarks) if benchmarks is not None else None
    if results is not None:
        for page, lang in COMPARE_PAGES.items():
            text = contents[page].decode("utf-8")
            start = text.index(BENCHMARK_START) + len(BENCHMARK_START)
            end = text.index(BENCHMARK_END)
            contents[page] = (text[:start] + "\n" + render_benchmarks(results, lang) + "\n" + text[end:]).encode("utf-8")
    if out.is_symlink() or (out.exists() and not out.is_dir()):
        raise AssemblyError(f"output must be a directory: {out}")
    if out.exists() and any(out.iterdir()):
        raise AssemblyError(f"output directory is not empty: {out}")
    for published, data in contents.items():
        target = out / published
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    return list(contents)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", required=True, type=Path, help="missing or empty output directory")
    parser.add_argument("--benchmarks", type=Path, help="downloaded latest-results JSON of Rullst/Benchmarks")
    args = parser.parse_args(argv)
    try:
        published = assemble(args.out, benchmarks=args.benchmarks)
    except AssemblyError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"assembled {len(published)} files into {args.out}: {', '.join(published)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
