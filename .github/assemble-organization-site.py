#!/usr/bin/env python3
"""Assemble the https://rullst.win/ root site from this repository's sources.

    python3 .github/assemble-organization-site.py --out DIRECTORY [--benchmarks [--benchmarks-max-age-days DAYS]]

Writes the English landing, privacy and comparison pages (/, /privacy.html,
/compare/), their Brazilian Portuguese versions under pt-br/ (/pt-br/comparar/
for the comparison) and the shared assets (stylesheet, script and
language-switcher flags) under assets/ into DIRECTORY, which must be missing
or empty. The pages also reference /Rullst/book/, /Rullst/images/ and
/Rullst/Rullst.png, which this repository's pages.yml deploys, so every path
is valid at the site root.

--benchmarks reads benchmarks.json from the working directory: a downloaded
copy (at most 1 MiB) of the site summary Rullst/Benchmarks publishes
(results/site.json, schema "rullst.benchmarks.site.v2"; see
docs/src/website-maintenance.md). Its overall and category scores, positions,
flags and memory for Rullst, Loco, Topcoat, Actix Web, Gin and Rullst
(minimal), and its Rullst reading, are rendered into the comparison pages.
Without the file, when it is invalid or uses anything the pages do not
explain, or when it was measured more than --benchmarks-max-age-days days
ago (default 90), the pages keep their "no current results" text and the
script prints a warning instead of failing.

The Rullst/Rullst.github.io deployment workflow runs this script from a
read-only checkout of this repository's main branch; it reads no secret and
never commits, pushes or deploys anything itself. validate-site.py runs it too.
"""

from __future__ import annotations

import argparse
from datetime import date, datetime, timezone
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
BENCHMARK_SCHEMA = "rullst.benchmarks.site.v2"
# Results measured longer ago than this are not shown (--benchmarks-max-age-days).
BENCHMARK_MAX_AGE_DAYS = 90
# The rows shown (Rullst/Benchmarks slug -> name). "rullst" is the application
# `cargo rullst new` generates, which has a position; "rullst-minimal" is the
# framework without that application's middleware, a diagnostic row without a
# position, shown only when the data has it.
MINIMAL = "rullst-minimal"
ROWS = {"rullst": "Rullst", "loco": "Loco", "topcoat": "Topcoat", "actix": "Actix Web", "gin": "Gin", MINIMAL: "Rullst (minimal)"}
OPTIONAL_ROWS = (MINIMAL,)
# The data must use exactly these categories, overall weights and flags; the
# page explains no other (a new one needs a new schema and a change here).
CATEGORIES = ("hello", "database", "application", "memory", "startup")
WEIGHTS = {"hello": 2, "database": 2, "application": 2, "memory": 1, "startup": 1}
FLAGS = {"cpu_cost": "*", "database_note": "†", "put_not_written": "‡", "within_reference_margin": "≈"}
STATUSES = ("ranked", "diagnostic", "errors", "incomplete", "not comparable", "not measured", "unstable")
STABILITY = ("pass", "warn", "fail")
MEMORY = ("peak_ram_mib", "axum_peak_ram_mib", "after_logins_peak_ram_mib", "axum_after_logins_peak_ram_mib")
MONTHS = {
    "en": ("January", "February", "March", "April", "May", "June", "July", "August",
           "September", "October", "November", "December"),
    "pt-BR": ("janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto",
              "setembro", "outubro", "novembro", "dezembro"),
}
TEXT = {
    "en": {
        "measured": "Measured on {date} with Rullst {version}{source}, against Axum {axum} on the same GitHub-hosted runners ({run}).",
        "run": "measurement run {id}",
        "source": {"crates.io": " from crates.io", "git": " from a git revision"},
        "full": 'The <a href="/Benchmarks/">full results</a> list all {ranked} ranked implementations, every endpoint and the method. They update automatically after each new Rullst release; this page is rebuilt daily.',
        "caption": "Score relative to Axum (1.00×; higher is better in every column), position among the {ranked} ranked implementations and tie margin (±).",
        "headers": ("Framework", "Overall", "Hello world", "Database", "Application", "Memory", "Startup", "Stability"),
        "names": {MINIMAL: "Rullst (minimal)"},
        "variants": {"rullst": "as <code>cargo rullst new</code> generates it", MINIMAL: "diagnostic, no position"},
        "missing": "Not in this measurement run.",
        "statuses": {"diagnostic": "diagnostic", "errors": "errors", "incomplete": "incomplete", "not comparable": "not comparable",
                     "not measured": "not measured", "unstable": "unstable"},
        "stability": {"pass": "pass", "warn": "warn", "fail": "fail"},
        "unpublished": "This run publishes no overall score; the categories are the result.",
        "legend": "Each cell shows the position (a range such as 3–7 is a tie: those implementations share positions 3 to 7 in no particular order), the score and its tie margin (±: the margin of its comparison with the score above; the first position has none).",
        "flags": {
            "cpu_cost": "* In at least one run of this score the runner's load generator and PostgreSQL were the limit (client-bound), so that run counts with its CPU-cost ratio, requests per CPU-second of the server and PostgreSQL together, instead of requests per second.",
            "database_note": "† This implementation's database setup differs from the others' in a way the full results explain.",
            "put_not_written": "‡ In this run most of this implementation's measured PUT /world/{id} requests did not write a row, so its Application score, and through it its overall score, are higher than they would be with a write. The full results explain the cause and its fix.",
            "within_reference_margin": "≈ Within the tie margin of Axum's 1.00×: not distinguishable from Axum.",
        },
        "memory": "Memory",
        "memory_caption": "RAM of the server in MiB, with Axum's on the same runners in parentheses. The Memory score compares the high-water mark under load; no score includes the peak after the {logins} logins of the application endpoints.",
        "memory_headers": ("Framework", "High-water mark under load", "Peak after the {logins} logins"),
        "reading": "What this means for Rullst",
    },
    "pt-BR": {
        "measured": "Medido em {date} com o Rullst {version}{source}, contra o Axum {axum} nas mesmas máquinas do GitHub ({run}).",
        "run": "execução da medição {id}",
        "source": {"crates.io": " do crates.io", "git": " de uma revisão git"},
        "full": 'Os <a href="/Benchmarks/">resultados completos</a> (em inglês) listam as {ranked} implementações classificadas, cada endpoint e o método. Eles se atualizam automaticamente a cada nova versão do Rullst; esta página é reconstruída diariamente.',
        "caption": "Pontuação em relação ao Axum (1,00×; mais é melhor em todas as colunas), posição entre as {ranked} implementações classificadas e margem de empate (±).",
        "headers": ("Framework", "Geral", "Hello world", "Banco de dados", "Aplicação", "Memória", "Inicialização", "Estabilidade"),
        "names": {MINIMAL: "Rullst (mínimo)"},
        "variants": {"rullst": "como o <code>cargo rullst new</code> gera", MINIMAL: "diagnóstico, sem posição"},
        "missing": "Fora desta rodada de medição.",
        "statuses": {"diagnostic": "diagnóstico", "errors": "erros", "incomplete": "incompleto", "not comparable": "não comparável",
                     "not measured": "não medido", "unstable": "instável"},
        "stability": {"pass": "passou", "warn": "alerta", "fail": "falhou"},
        "unpublished": "Esta rodada não publica pontuação geral; o resultado são as categorias.",
        "legend": "Cada célula mostra a posição (um intervalo como 3–7 é um empate: essas implementações dividem as posições 3 a 7, sem ordem entre elas), a pontuação e a margem de empate dela (±: a margem da comparação com a pontuação acima; a primeira posição não tem).",
        "flags": {
            "cpu_cost": "* Em pelo menos uma medição desta pontuação o gerador de carga e o PostgreSQL da máquina eram o limite (client-bound), então essa medição conta com a razão de custo de CPU, requisições por segundo de CPU do servidor e do PostgreSQL juntos, em vez de requisições por segundo.",
            "database_note": "† A configuração de banco de dados desta implementação difere da das outras de um jeito que os resultados completos explicam.",
            "put_not_written": "‡ Nesta rodada, a maioria das requisições PUT /world/{id} medidas desta implementação não gravou nenhuma linha, então a pontuação de Aplicação dela, e por meio dela a pontuação geral, são mais altas do que seriam com uma gravação. Os resultados completos explicam a causa e a correção.",
            "within_reference_margin": "≈ Dentro da margem de empate do 1,00× do Axum: não dá para distinguir do Axum.",
        },
        "memory": "Memória",
        "memory_caption": "RAM do servidor em MiB, com a do Axum nas mesmas máquinas entre parênteses. A pontuação de Memória compara o pico sob carga; nenhuma pontuação inclui o pico depois dos {logins} logins dos endpoints de aplicação.",
        "memory_headers": ("Framework", "Pico sob carga", "Pico depois dos {logins} logins"),
        "reading": "O que isto significa para o Rullst",
    },
}
# The data's note and reading, by page language.
NOTE_KEYS = {"en": "note", "pt-BR": "note_pt_br"}
READING_KEYS = {"en": "en", "pt-BR": "pt_br"}
NOTE_LIMIT = 1000
READING_LIMIT = 4000
# Backticks in a note pair up in order; a pair around 1 to 80 characters on
# one line, not all blank and without `<` or `>`, renders as <code>. Any other
# pair and a final unpaired backtick stay literal text.
CODE_TEXT = re.compile(r"[^<>\r\n]{1,80}")
RUN_URL = re.compile(r"https://github\.com/Rullst/Benchmarks/actions/runs/([0-9]{1,20})")
VERSION = re.compile(r"[0-9A-Za-z][0-9A-Za-z.+-]{0,39}")


class AssemblyError(Exception):
    """The sources cannot produce a consistent root site."""


BENCHMARK_LIMIT = 1024 * 1024
# The command line selects this fixed name; it never takes a results path.
BENCHMARK_FILE = "benchmarks.json"


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


def checked_metric(name: str, key: str, raw: object, limit: float = 1e9) -> float:
    if isinstance(raw, bool) or not isinstance(raw, (int, float)):
        raise ValueError(f"{name}: {key} is not a number")
    value = float(raw)
    if not math.isfinite(value) or not 0 <= value <= limit:
        raise ValueError(f"{name}: {key} is not a finite number from 0 to {limit:g}")
    return value


def checked_int(name: str, raw: object, low: int, high: int) -> int:
    if isinstance(raw, bool) or not isinstance(raw, int) or not low <= raw <= high:
        raise ValueError(f"{name} is not a whole number from {low} to {high}")
    return raw


def checked_text(key: str, raw: object, limit: int = NOTE_LIMIT) -> str:
    if not isinstance(raw, str) or len(raw) > limit:
        raise ValueError(f"{key} is not a string of at most {limit} characters")
    return raw.strip()


def checked_version(name: str, raw: object) -> str:
    if not isinstance(raw, str) or not VERSION.fullmatch(raw):
        raise ValueError(f"unexpected version for {name}: {raw!r}")
    return raw


def checked_cell(name: str, raw: object, ranked: int) -> dict:
    """One score: its status, value, position range, tie margin and known flags."""
    if not isinstance(raw, dict) or raw.get("status") not in STATUSES:
        raise ValueError(f"{name}: unknown status {raw.get('status') if isinstance(raw, dict) else raw!r}")
    score = None if raw.get("score") is None else checked_metric(name, "score", raw["score"], 1000)
    position = raw.get("position")
    if position is not None:
        if not isinstance(position, list) or len(position) != 2 or score is None:
            raise ValueError(f"{name}: a position is a [first, last] range of a score")
        first, last = (checked_int(f"{name} position", value, 1, ranked) for value in position)
        if first > last:
            raise ValueError(f"{name}: position range {position} is reversed")
        position = (first, last)
    elif raw["status"] == "ranked" and score is not None:
        raise ValueError(f"{name}: a ranked score needs a position")
    margin = None if raw.get("margin_percent") is None else checked_metric(name, "margin_percent", raw["margin_percent"], 1000)
    flags = raw.get("flags", [])
    if not isinstance(flags, list) or any(flag not in FLAGS for flag in flags):
        raise ValueError(f"{name}: unknown flag in {flags!r}")
    return {"status": raw["status"], "score": score, "position": position, "margin": margin, "flags": set(flags)}


def benchmark_row(raw: object, ranked: int, overall_published: bool) -> tuple[str, dict] | None:
    """The shown row a result belongs to and its checked values, or None for another implementation."""
    if not isinstance(raw, dict) or raw.get("slug") not in ROWS:
        return None
    slug = raw["slug"]
    name = ROWS[slug]
    role = "diagnostic" if slug in OPTIONAL_ROWS else "ranked"
    if raw.get("role") != role:
        raise ValueError(f"{name}: expected the role {role}, not {raw.get('role')!r}")
    categories = raw.get("categories")
    if not isinstance(categories, dict) or set(categories) != set(CATEGORIES):
        raise ValueError(f"{name}: the categories are not {CATEGORIES}")
    stability = raw.get("stability")
    if stability is not None and stability not in STABILITY:
        raise ValueError(f"{name}: unknown stability result {stability!r}")
    memory = raw.get("memory") or {}
    if not isinstance(memory, dict):
        raise ValueError(f"{name}: memory is not an object")
    return slug, {
        "version": checked_version(name, raw.get("version")),
        "overall": checked_cell(f"{name} overall", raw.get("overall"), ranked) if overall_published else None,
        "categories": {key: checked_cell(f"{name} {key}", categories[key], ranked) for key in CATEGORIES},
        "stability": stability,
        "memory": {key: checked_metric(name, key, memory[key]) for key in MEMORY if memory.get(key) is not None},
    }


def checked_results(data: object, today: date, max_age_days: int) -> dict:
    """Every value the page shows, checked; ValueError for anything the page does not know."""
    if not isinstance(data, dict) or data.get("schema") != BENCHMARK_SCHEMA:
        raise ValueError(f"schema is not {BENCHMARK_SCHEMA}")
    measured_on = date.fromisoformat(str(data["measured_on"]))
    if measured_on > today:
        raise ValueError(f"measured_on {measured_on} is after today ({today})")
    if (today - measured_on).days > max_age_days:
        raise ValueError(f"the results were measured on {measured_on}, more than {max_age_days} days ago")
    run = data.get("run")
    match = RUN_URL.fullmatch(str(run.get("url", ""))) if isinstance(run, dict) else None
    if match is None:
        raise ValueError(f"unexpected run url: {run!r}")
    reference = data.get("reference")
    if not isinstance(reference, dict) or reference.get("slug") != "axum" or reference.get("framework") != "Axum":
        raise ValueError("the reference is not Axum")
    ranked = checked_int("ranked", data.get("ranked"), 1, 1000)
    overall_published = data.get("overall_published")
    if not isinstance(overall_published, bool):
        raise ValueError("overall_published is not true or false")
    if overall_published and data.get("weights") != WEIGHTS:
        raise ValueError(f"the overall weights are not {WEIGHTS}")
    categories = data.get("categories")
    if not isinstance(categories, list) or [c.get("key") if isinstance(c, dict) else c for c in categories] != list(CATEGORIES):
        raise ValueError(f"the categories are not {CATEGORIES}")
    if data.get("flags") != FLAGS:
        raise ValueError(f"the flags are not {FLAGS}")
    if not isinstance(data.get("rows"), list):
        raise ValueError("rows is not a list")
    rows: dict[str, dict] = {}
    for slug, row in filter(None, (benchmark_row(raw, ranked, overall_published) for raw in data["rows"])):
        if slug in rows:
            raise ValueError(f"{ROWS[slug]} appears twice")
        rows[slug] = row
    source = data.get("rullst_source")
    source = source.get("kind") if isinstance(source, dict) else None
    source = source if isinstance(source, str) and source in TEXT["en"]["source"] else None
    reading = data.get("reading")
    if not isinstance(reading, dict):
        raise ValueError("reading is not an object")
    readings = {lang: checked_text(f"reading.{key}", reading.get(key), READING_LIMIT) for lang, key in READING_KEYS.items()}
    if not all(readings.values()):
        raise ValueError("the reading is empty")
    return {
        "measured_on": measured_on,
        "rullst_version": checked_version("Rullst", data.get("rullst_version")),
        "source": source,
        "run_url": match[0],
        "run_id": match[1],
        "axum_version": checked_version("Axum", reference.get("version")),
        "ranked": ranked,
        "overall_published": overall_published,
        "logins": checked_int("logins", data.get("logins"), 1, 100000) if data.get("logins") is not None else None,
        "rows": rows,
        "notes": {lang: checked_text(key, data[key]) for lang, key in NOTE_KEYS.items() if data.get(key)},
        "readings": readings,
    }


def load_benchmarks(path: Path, today: date | None = None, max_age_days: int = BENCHMARK_MAX_AGE_DAYS) -> dict | None:
    """Return the checked results, or None after a warning.

    An unknown schema, category, flag, status or weighting, a bad or future
    date, results older than max_age_days, a non-finite or negative number,
    an unexpected version string or another run URL discards the whole file,
    so the page shows no numbers rather than older or partial ones.
    """
    today = today or datetime.now(timezone.utc).date()
    try:
        results = checked_results(read_benchmark_file(path), today, max_age_days)
    except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
        print(f"warning: benchmark results not used ({error})", file=sys.stderr)
        return None
    if not results["rows"]:
        print("warning: benchmark results name none of the compared frameworks", file=sys.stderr)
        return None
    return results


def render_note(note: str) -> str:
    """A note's text, escaped, with its short backtick spans as <code>."""
    pieces = note.split("`")
    parts = []
    for index, piece in enumerate(pieces):
        if index % 2 == 0:
            parts.append(escape(piece))
            continue
        closed = index + 1 < len(pieces)
        if closed and piece.strip() and CODE_TEXT.fullmatch(piece):
            parts.append(f"<code>{escape(piece)}</code>")
        else:
            parts.append(escape("`" + piece + ("`" if closed else "")))
    return "".join(parts)


def format_number(value: float, decimals: int, lang: str) -> str:
    text = f"{value:,.{decimals}f}"
    return text.translate(str.maketrans(",.", ".,")) if lang == "pt-BR" else text


def render_cell(cell: dict, lang: str) -> str:
    """Position, score with its flags, and tie margin; the status when there is no score."""
    if cell["score"] is None:
        return f'<td class="benchmark-status">{escape(TEXT[lang]["statuses"].get(cell["status"], cell["status"]))}</td>'
    first, last = cell["position"] or (None, None)
    position = "–" if first is None else (f"{first}" if first == last else f"{first}–{last}")
    score = format_number(cell["score"], 2 if cell["score"] >= 0.1 else 3, lang) + "×"
    score += "*" if "cpu_cost" in cell["flags"] else ""
    score += "".join(f" {sign}" for flag, sign in FLAGS.items() if flag != "cpu_cost" and flag in cell["flags"])
    margin = f'<span class="benchmark-margin">±{format_number(cell["margin"], 1, lang)}%</span>' if cell["margin"] is not None else ""
    return f'<td><span class="benchmark-position">{position}</span><span class="benchmark-score">{escape(score)}</span>{margin}</td>'


def row_header(slug: str, row: dict | None, lang: str) -> str:
    name = TEXT[lang]["names"].get(slug, ROWS[slug])
    details = [escape(row["version"])] if row else []
    if variant := TEXT[lang]["variants"].get(slug):
        details.append(variant)
    detail = f'<span class="benchmark-variant">{" · ".join(details)}</span>' if details else ""
    return f'<th scope="row">{name}{detail}</th>'


def shown_rows(results: dict) -> list[str]:
    """Measured rows by overall position (the fixed order without a published overall), then missing rows."""
    rows = results["rows"]
    order = [slug for slug in ROWS if slug in rows]
    if results["overall_published"]:
        order.sort(key=lambda slug: (slug in OPTIONAL_ROWS, (rows[slug]["overall"]["position"] or (10**6,))[0],
                                     -(rows[slug]["overall"]["score"] or 0)))
    missing = [slug for slug in ROWS if slug not in rows and slug not in OPTIONAL_ROWS]
    return [slug for slug in order if slug not in OPTIONAL_ROWS] + missing + [slug for slug in order if slug in OPTIONAL_ROWS]


def render_table(results: dict, order: list[str], lang: str) -> str:
    text = TEXT[lang]
    headers = [name for index, name in enumerate(text["headers"]) if index != 1 or results["overall_published"]]
    head = "".join(f'<th scope="col">{escape(name)}</th>' for name in headers)
    lines = []
    for slug in order:
        row = results["rows"].get(slug)
        if row is None:
            lines.append(f'<tr>{row_header(slug, None, lang)}<td colspan="{len(headers) - 1}">{text["missing"]}</td></tr>')
            continue
        cells = [row["overall"]] if results["overall_published"] else []
        cells += [row["categories"][key] for key in CATEGORIES]
        stability = escape(text["stability"].get(row["stability"], "–")) if row["stability"] else "–"
        css = ' class="benchmark-diagnostic"' if slug in OPTIONAL_ROWS else ""
        lines.append(f'<tr{css}>{row_header(slug, row, lang)}{"".join(render_cell(cell, lang) for cell in cells)}<td>{stability}</td></tr>')
    # The caption stays outside the horizontally scrolling region, readable at any width.
    caption = escape(text["caption"].format(ranked=results["ranked"]))
    return (f'<p class="benchmark-caption" id="benchmark-caption">{caption}</p>'
            f'<div class="benchmark-table" role="region" aria-labelledby="benchmark-caption" tabindex="0">'
            f'<table aria-labelledby="benchmark-caption"><thead><tr>{head}</tr></thead>'
            f'<tbody>{"".join(lines)}</tbody></table></div>')


def render_memory(results: dict, order: list[str], lang: str) -> str:
    text = TEXT[lang]
    logins = results["logins"] or "–"

    def pair(memory: dict, own: str, axum: str) -> str:
        if own not in memory:
            return "–"
        value = format_number(memory[own], 0, lang)
        return value + (f' <span class="benchmark-axum">({format_number(memory[axum], 0, lang)})</span>' if axum in memory else "")
    head = "".join(f'<th scope="col">{escape(name.format(logins=logins))}</th>' for name in text["memory_headers"])
    lines = [
        f'<tr>{row_header(slug, row, lang)}<td>{pair(row["memory"], "peak_ram_mib", "axum_peak_ram_mib")}</td>'
        f'<td>{pair(row["memory"], "after_logins_peak_ram_mib", "axum_after_logins_peak_ram_mib")}</td></tr>'
        for slug in order if (row := results["rows"].get(slug)) is not None
    ]
    caption = escape(text["memory_caption"].format(logins=logins))
    return (f'<h3>{text["memory"]}</h3><p class="benchmark-caption" id="benchmark-memory-caption">{caption}</p>'
            f'<div class="benchmark-table benchmark-memory" role="region" aria-labelledby="benchmark-memory-caption" tabindex="0">'
            f'<table aria-labelledby="benchmark-memory-caption"><thead><tr>{head}</tr></thead>'
            f'<tbody>{"".join(lines)}</tbody></table></div>')


def render_benchmarks(results: dict, lang: str) -> str:
    """HTML for the comparison page's results block, escaped throughout."""
    text = TEXT[lang]
    measured = results["measured_on"]
    month = MONTHS[lang][measured.month - 1]
    day = f"{measured.day} de {month} de {measured.year}" if lang == "pt-BR" else f"{measured.day} {month} {measured.year}"
    run = f'<a href="{escape(results["run_url"])}">{text["run"].format(id=escape(results["run_id"]))}</a>'
    meta = text["measured"].format(date=f'<time datetime="{measured.isoformat()}">{escape(day)}</time>',
                                   version=escape(results["rullst_version"]), source=text["source"].get(results["source"], ""),
                                   axum=escape(results["axum_version"]), run=run)
    lines = [f'<p class="benchmark-meta">{meta} {text["full"].format(ranked=results["ranked"])}</p>']
    note, note_lang = results["notes"].get(lang), lang
    if note is None and results["notes"].get("en"):
        note, note_lang = results["notes"]["en"], "en"
    if note:
        attribute = "" if note_lang == lang else ' lang="en"'
        lines.append(f'<p class="benchmark-note"{attribute}>{render_note(note)}</p>')
    order = shown_rows(results)
    lines.append(render_table(results, order, lang))
    shown = {flag for row in results["rows"].values() for cell in [row["overall"], *row["categories"].values()] if cell for flag in cell["flags"]}
    legend = [text["legend"]] + ([] if results["overall_published"] else [text["unpublished"]])
    legend += [escape(text["flags"][flag]) for flag in FLAGS if flag in shown]
    lines.append('<ul class="benchmark-legend">' + "".join(f"<li>{item}</li>" for item in legend) + "</ul>")
    lines.append(render_memory(results, order, lang))
    lines.append(f'<h3>{text["reading"]}</h3>')
    lines.append(f'<p class="benchmark-reading">{render_note(results["readings"][lang])}</p>')
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


def assemble(out: Path, root: Path = ROOT, benchmarks: Path | None = None, today: date | None = None,
             max_age_days: int = BENCHMARK_MAX_AGE_DAYS) -> list[str]:
    """Write the root site into OUT and return the published paths."""
    contents = check_sources(root)
    results = load_benchmarks(benchmarks, today, max_age_days) if benchmarks is not None else None
    try:
        # Both languages or neither: a failure keeps the pending text and still
        # deploys, rather than failing the deployment and leaving older pages up.
        blocks = {page: render_benchmarks(results, lang) for page, lang in COMPARE_PAGES.items()} if results else {}
    except (KeyError, TypeError, ValueError) as error:
        print(f"warning: benchmark results not rendered ({error!r})", file=sys.stderr)
        blocks = {}
    for page, block in blocks.items():
        text = contents[page].decode("utf-8")
        start = text.index(BENCHMARK_START) + len(BENCHMARK_START)
        end = text.index(BENCHMARK_END)
        contents[page] = (text[:start] + "\n" + block + "\n" + text[end:]).encode("utf-8")
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
    parser.add_argument(
        "--benchmarks", action="store_true",
        help=f"render {BENCHMARK_FILE} (the Rullst/Benchmarks site summary) from the working directory",
    )
    parser.add_argument(
        "--benchmarks-max-age-days", type=int, default=BENCHMARK_MAX_AGE_DAYS, metavar="DAYS",
        help=f"do not show results measured more than DAYS days ago (default {BENCHMARK_MAX_AGE_DAYS})",
    )
    args = parser.parse_args(argv)
    if args.benchmarks_max_age_days < 0:
        parser.error("--benchmarks-max-age-days must not be negative")
    try:
        published = assemble(args.out, benchmarks=Path(BENCHMARK_FILE) if args.benchmarks else None,
                             max_age_days=args.benchmarks_max_age_days)
    except AssemblyError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"assembled {len(published)} files into {args.out}: {', '.join(published)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
