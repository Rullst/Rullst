#!/usr/bin/env python3
"""Validate the generated root site and the benchmark templates without network I/O.

The landing, privacy and comparison pages of https://rullst.win/, in English
and Brazilian Portuguese, are written by the Rust program in site/ with
Rullst's html! macro; Rullst/Rullst.github.io runs it to deploy them, and
/Rullst/ only serves docs/redirect_template.html, pointing there. This script
runs the built generator (RULLST_SITE_BIN, default
site/target/debug/rullst-site; build it with
`cargo build --locked --manifest-path site/Cargo.toml`) and checks what it
writes: the two languages must keep the same section ids, external links and
integration names, and the benchmark and page-weight data must render only
when every check passes.
"""

from __future__ import annotations

import copy
from datetime import date
from html import unescape
import json
import os
from html.parser import HTMLParser
from hashlib import sha256
import importlib.util
import re
import shutil
import subprocess
import tempfile
from pathlib import Path
from urllib.parse import urlparse


ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"
# The generated pages, written by main() into a temporary directory.
SITE = Path()
HOME = SITE / "index.html"
PRIVACY = SITE / "privacy.html"
HOME_PT_BR = SITE / "pt-br/index.html"
PRIVACY_PT_BR = SITE / "pt-br/privacy.html"
COMPARE = SITE / "compare/index.html"
COMPARE_PT_BR = SITE / "pt-br/comparar/index.html"
GENERATOR = Path(os.environ.get("RULLST_SITE_BIN") or ROOT / "site/target/debug/rullst-site")
# Published path -> the file it comes from, for the generated assets.
PUBLISHED = (
    "index.html", "privacy.html", "pt-br/index.html", "pt-br/privacy.html", "compare/index.html",
    "pt-br/comparar/index.html", "assets/site.css", "assets/site.js", "assets/flags/us.svg", "assets/flags/br.svg",
)
ASSET_SOURCES = {
    "assets/site.css": DOCS / "site.css", "assets/site.js": DOCS / "site.js",
    "assets/flags/us.svg": DOCS / "flags/us.svg", "assets/flags/br.svg": DOCS / "flags/br.svg",
}
BENCHMARK_FIXTURE = ROOT / ".github" / "fixtures" / "site-benchmarks.json"
FOOTPRINT_FIXTURE = ROOT / ".github" / "fixtures" / "site-footprint.json"
# The day the footprint fixture was measured (its measured_at).
FOOTPRINT_DAY = "2026-11-22"
# The day the fixture is rendered as of (its measured_on is the day before).
FIXTURE_DAY = date(2026, 11, 21)
REDIRECT = DOCS / "redirect_template.html"
SITE_URL = "https://rullst.win/"
# English external link -> its Brazilian Portuguese counterpart. Every other
# external link (mailto: included) must appear identically on both pages.
PT_BR_COUNTERPARTS = {
    SITE_URL: SITE_URL + "pt-br/",
    "https://academy.rullst.win/": "https://academy.rullst.win/pt-BR",
    "https://academy.rullst.win/en/paths": "https://academy.rullst.win/pt-BR/paths",
    "https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement":
        "https://docs.github.com/pt/site-policy/privacy-policies/github-general-privacy-statement",
    "https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement#international-data-transfers":
        "https://docs.github.com/pt/site-policy/privacy-policies/github-general-privacy-statement#international-data-transfers",
    "https://policies.google.com/privacy": "https://policies.google.com/privacy?hl=pt-BR",
    "mailto:officialrullst@gmail.com?subject=Privacy%20request": "mailto:officialrullst@gmail.com?subject=Pedido%20de%20privacidade",
}
BENCH_TEMPLATES = sorted(DOCS.glob("benches*_template.html"))
README = ROOT / "README.md"
# The blocks the generator fills on each landing page: from its container to the
# markup that follows it.
GENERATOR_BLOCKS = {
    "proof": ("<div class=\"proof-data\" data-landing-proof>", "</div>"),
    "summary": ("<div class=\"landing-bench\" data-landing-benchmarks>", "<div class=\"performance-links\">"),
}
RESULTS_BLOCK = ("<div class=\"benchmark-results\" data-benchmark-results>", "</section>")
MINIMAL_NAMES = {"en": "Rullst (minimal)", "pt-BR": "Rullst (mínimo)"}
FLAG_SIGNS = ("*", "†", "‡", "≈")


LLMS_INDEX_LIMIT = 128 * 1024
LLMS_FULL_LIMIT = 8 * 1024 * 1024
SUMMARY_PAGE = re.compile(r"\[[^\]]+\]\(([^)]+\.md)\)")
CLI_VERSION = re.compile(r'^version = "([^"]+)"$', re.MULTILINE)
INSTALL_PIN = re.compile(r"cargo install cargo-rullst --version =(\S+) --locked")


def validate_llms() -> int:
    """Generate llms.txt and llms-full.txt and check they cover SUMMARY.md."""
    spec = importlib.util.spec_from_file_location("generate_llms_txt", ROOT / ".github/generate-llms-txt.py")
    assert spec is not None and spec.loader is not None, "cannot load generate-llms-txt.py"
    generator = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(generator)
    with tempfile.TemporaryDirectory(prefix="rullst-site-llms-") as temp:
        assert generator.main(["--docs", str(DOCS), "--out", temp]) == 0, "llms generator failed"
        index = (Path(temp) / "llms.txt").read_text(encoding="utf-8")
        full = (Path(temp) / "llms-full.txt").read_text(encoding="utf-8")
    assert 0 < len(index.encode()) < LLMS_INDEX_LIMIT, "llms.txt size is out of bounds"
    assert 0 < len(full.encode()) < LLMS_FULL_LIMIT, "llms-full.txt size is out of bounds"
    pages = list(dict.fromkeys(SUMMARY_PAGE.findall((DOCS / "src/SUMMARY.md").read_text(encoding="utf-8"))))
    for page in pages:
        url = generator.page_url(page)
        assert f"]({url})" in index, f"llms.txt is missing {page}"
        assert f"Source: {url}\n" in full, f"llms-full.txt is missing {page}"
    return len(pages)


README_INTEGRATIONS = re.compile(r"^## 🔌 Integrations\n(?P<body>.*?)(?=^## )", re.MULTILINE | re.DOTALL)
README_INTEGRATION_ROW = re.compile(r"^\| \*\*(?P<level>.+?)\*\* \| (?P<items>.+?) \|$", re.MULTILINE)
LANDING_INTEGRATION_LEVEL = re.compile(
    r'<article class="integration-level" data-integration-level="[a-z-]+">(?P<body>.*?)</article>', re.DOTALL
)
RECIPES = ("MongoDB", "DuckDB", "Qdrant")


def plain_markdown(text: str) -> str:
    text = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", text)
    return " ".join(text.replace("**", "").replace("`", "").split())


def plain_html(fragment: str) -> str:
    return " ".join(unescape(re.sub(r"<[^>]+>", "", fragment)).split())


def sentence_case(text: str) -> str:
    return text[:1].upper() + text[1:]


def integration_names(source: str) -> list[tuple[str, list[list[str]]]]:
    """Per evidence level: its id and, per entry, the bold service names."""
    levels = re.findall(
        r'<article class="integration-level" data-integration-level="([a-z-]+)">(.*?)</article>', source, re.DOTALL
    )
    return [
        (level, [re.findall(r"<b>(.*?)</b>", item) for item in re.findall(r"<li>(.*?)</li>", body, re.DOTALL)])
        for level, body in levels
    ]


def external_links(document: "Document") -> list[str]:
    return sorted(link for link in document.links if urlparse(link).scheme)


def validate_translation(english: "Document", english_source: str, translated: "Document", source: str, label: str) -> None:
    """Fail when one language changes without the other."""
    assert translated.ids == english.ids, (
        f"{label}: section ids differ from the English page:\n"
        f"English: {english.ids}\n{label}: {translated.ids}"
    )
    expected_links = sorted(PT_BR_COUNTERPARTS.get(link, link) for link in external_links(english))
    assert external_links(translated) == expected_links, (
        f"{label}: external links differ from the English page: "
        f"missing {sorted(set(expected_links) - set(external_links(translated)))}, "
        f"extra {sorted(set(external_links(translated)) - set(expected_links))}"
    )
    assert integration_names(source) == integration_names(english_source), (
        f"{label}: integration names differ from the English page:\n"
        f"English: {integration_names(english_source)}\n{label}: {integration_names(source)}"
    )
    for name in RECIPES:
        recipes = re.search(r'<p class="integration-recipes" data-integration-recipes>(.*?)</p>', source, re.DOTALL)
        if "integration-level" in source:
            assert recipes and name in recipes[1], f"{label}: recipes line must name {name}"


def validate_integrations(source: str, readme: str) -> int:
    """Keep the landing's Integrations section on README's table, item by item.

    Each README cell lists items separated by "; "; the landing shows the same
    items, in the same order, one per list entry (only the first letter's case
    may differ). MongoDB, DuckDB and Qdrant are recipes, never integrations.
    """
    section = README_INTEGRATIONS.search(readme)
    assert section, "README is missing the Integrations section"
    expected = [
        (plain_markdown(row["level"]), [sentence_case(plain_markdown(item)) for item in row["items"].split("; ")])
        for row in README_INTEGRATION_ROW.finditer(section["body"])
    ]
    assert len(expected) == 3, f"README Integrations table must have three evidence levels: {expected}"
    landing = []
    for level in LANDING_INTEGRATION_LEVEL.finditer(source):
        title = re.search(r"<h3>(.*?)</h3>", level["body"], re.DOTALL)
        assert title, "landing integration level is missing its title"
        items = [sentence_case(plain_html(item)) for item in re.findall(r"<li>(.*?)</li>", level["body"], re.DOTALL)]
        landing.append((plain_html(title[1]), items))
    assert landing == expected, (
        "landing Integrations section differs from README's table:\n"
        f"landing: {landing}\nREADME:  {expected}"
    )
    recipes = re.search(r"<p class=\"integration-recipes\" data-integration-recipes>(.*?)</p>", source, re.DOTALL)
    assert recipes, "landing must name the recipes separately from the integrations"
    readme_after_table = section["body"][section["body"].rindex("|"):]
    for name in RECIPES:
        assert name in plain_html(recipes[1]), f"landing recipes line must name {name}"
        assert name in readme_after_table, f"README must name {name} as a recipe"
        assert not any(name in item for _level, items in landing for item in items), (
            f"{name} is a recipe, not a built-in integration"
        )
    assert "/Rullst/book/recipes/index.html" in recipes[1], "landing recipes line must link the recipes"
    assert "/Rullst/book/provider-conformance.html" in source, "landing must link the provider conformance kits"
    return sum(len(items) for _level, items in landing)


class Document(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: list[str] = []
        self.links: list[str] = []
        self.scripts: list[str] = []
        self.stylesheets: list[str] = []
        self.images_without_alt: list[str] = []
        self.images: list[str] = []
        self.embeds: list[str] = []
        self.remote_scripts_without_integrity: list[str] = []
        self.inline_behavior: list[str] = []
        self.h1_count = 0
        self.has_title = False
        self.has_language = False
        self.has_main = False
        self.has_nav = False
        self.has_footer = False

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        values = dict(attrs)
        if tag == "html" and values.get("lang"):
            self.has_language = True
        if tag == "title":
            self.has_title = True
        if tag == "h1":
            self.h1_count += 1
        if tag == "main":
            self.has_main = True
        if tag == "nav":
            self.has_nav = True
        if tag == "footer":
            self.has_footer = True
        if identifier := values.get("id"):
            self.ids.append(identifier)
        if tag == "a" and (href := values.get("href")):
            self.links.append(href)
        if tag == "script" and (src := values.get("src")):
            self.scripts.append(src)
            if src.startswith("https://") and not values.get("integrity"):
                self.remote_scripts_without_integrity.append(src)
        if tag == "link" and values.get("rel") == "stylesheet" and (href := values.get("href")):
            self.stylesheets.append(href)
        if tag == "img" and "alt" not in values:
            self.images_without_alt.append(values.get("src", "<unknown>"))
        if tag == "img" and (src := values.get("src")):
            self.images.append(src)
        if tag in ("iframe", "embed", "object"):
            self.embeds.append(tag)
        for name, _value in attrs:
            if name == "style" or name.startswith("on"):
                self.inline_behavior.append(f"{tag}[{name}]")


def parse(path: Path) -> tuple[Document, str]:
    source = path.read_text(encoding="utf-8")
    document = Document()
    document.feed(source)
    document.close()
    return document, source


def local_home_target(reference: str) -> Path | None:
    parsed = urlparse(reference)
    if parsed.scheme or parsed.netloc or reference.startswith("mailto:"):
        return None
    path = parsed.path
    if not path or path == "/" or path == "/Benchmarks/" or path.startswith("/Rullst/benches/"):
        # The root landing itself, the separate Rullst/Benchmarks site and the
        # dashboards assembled from gh-pages data are checked elsewhere.
        return None
    if path == "/Rullst/":
        return REDIRECT
    if path in ("/privacy.html", "./privacy.html"):
        return PRIVACY
    if path == "/pt-br/":
        return HOME_PT_BR
    if path == "/pt-br/privacy.html":
        return PRIVACY_PT_BR
    if path == "/compare/":
        return COMPARE
    if path == "/pt-br/comparar/":
        return COMPARE_PT_BR
    if path == "/Rullst/book/":
        return DOCS / "book" / "index.html"
    if path.startswith("/Rullst/book/"):
        return DOCS / "book" / path.removeprefix("/Rullst/book/")
    if path == "/Rullst/Rullst.png":
        return DOCS / "Rullst.png"
    if path.startswith("/Rullst/images/"):
        return ROOT / path.removeprefix("/Rullst/")
    if path in ("/assets/site.css", "/assets/site.js"):
        return DOCS / path.removeprefix("/assets/")
    if path.startswith("/assets/flags/"):
        return DOCS / "flags" / path.removeprefix("/assets/flags/")
    raise AssertionError(f"unclassified local home reference: {reference}")


def validate_document(path: Path, document: Document) -> None:
    assert document.has_language, f"{path}: missing html language"
    assert document.has_title, f"{path}: missing title"
    assert document.h1_count == 1, f"{path}: expected exactly one h1"
    assert len(document.ids) == len(set(document.ids)), f"{path}: duplicate id"
    assert not document.images_without_alt, f"{path}: images missing alt: {document.images_without_alt}"


WORKFLOW_REFERENCE = re.compile(r"(?:actions/workflows/|\.github/workflows/)([A-Za-z0-9_.-]+\.ya?ml)")
INVENTORY_ROW = re.compile(r"^\| \[`([A-Za-z0-9_.-]+\.ya?ml)`\]", re.MULTILINE)


def validate_workflow_references(readme: str) -> int:
    """Keep the WORKFLOWS.md inventory and every badge or link on real files.

    A renamed, merged or removed workflow must leave no stale row, badge or
    link behind. Links into the frozen v12 branch name that branch's files.
    """
    workflows = {
        path.name
        for path in (ROOT / ".github" / "workflows").iterdir()
        if path.suffix in {".yml", ".yaml"}
    }
    inventory_text = (ROOT / "WORKFLOWS.md").read_text(encoding="utf-8")
    assert (
        f"## Workflow inventory ({len(workflows)} definitions)" in inventory_text
    ), "WORKFLOWS.md inventory count is stale"
    rows = INVENTORY_ROW.findall(inventory_text)
    assert len(rows) == len(set(rows)), "WORKFLOWS.md inventory lists a workflow twice"
    assert set(rows) == workflows, (
        "WORKFLOWS.md inventory differs from .github/workflows: "
        f"missing {sorted(workflows - set(rows))}, stale {sorted(set(rows) - workflows)}"
    )
    documents = [
        README, ROOT / "WORKFLOWS.md", ROOT / "MAINTAINING.md",
        *sorted(ROOT.glob("*/README.md")), *sorted((DOCS / "src").rglob("*.md")),
    ]
    for document in documents:
        text = readme if document == README else document.read_text(encoding="utf-8")
        for match in WORKFLOW_REFERENCE.finditer(text):
            if re.search(r"/(?:blob|tree)/v12/[^\s)]*$", text[max(0, match.start() - 200):match.start()]):
                continue
            assert match[1] in workflows, (
                f"{document.relative_to(ROOT)} names a missing workflow: {match[1]}"
            )
    return len(workflows)


LANGUAGES = {
    "en": {"home": HOME, "privacy": PRIVACY, "url": SITE_URL, "locale": "en_US",
           "dedication": "All glory and honor to God", "status": "Rullst v13 prerelease",
           "slogan": "Intelligent, Security-Conscious, and Designed for Effortless Productivity",
           "performance": "measures each new Rullst release",
           "lede": "A full-stack web framework for Rust, built on <strong>Axum</strong> and <strong>SQLx</strong>",
           "illustration": "Not live telemetry.",
           "around": "…and the pieces around it, each an optional package:",
           "made_with": "Made with ",
           "page_weight": "This page: 484 kB in 7 requests on a first visit",
           "disclosure": "Rullst's maintainer also maintains that benchmark",
           "no_audit": "no external audit has been completed",
           "maintainer": "Rullst has one maintainer",
           # Marketing words and absolute claims the landing never makes.
           "banned": ("blazing", "revolutionary", "best-in-class", "the best", "no other framework", "the only framework",
                      "unique", "more secure", "experimental", "world-class")},
    "pt-BR": {"home": HOME_PT_BR, "privacy": PRIVACY_PT_BR, "url": SITE_URL + "pt-br/", "locale": "pt_BR",
              "dedication": "Toda glória e honra a Deus", "status": "Pré-lançamento do Rullst v13",
              "slogan": "Inteligente, consciente da segurança e feito para uma produtividade sem esforço",
              "performance": "mede cada nova versão do Rullst",
              "lede": "Um framework web full-stack para Rust, construído sobre <strong>Axum</strong> e <strong>SQLx</strong>",
              "illustration": "Não é telemetria ao vivo.",
              "around": "…e as peças ao redor dele, cada uma um pacote opcional:",
              "made_with": "Feito com ",
              "page_weight": "Esta página: 484 kB em 7 requisições na primeira visita",
              "disclosure": "O mantenedor do Rullst também mantém esse benchmark",
              "no_audit": "nenhuma auditoria externa foi concluída",
              "maintainer": "O Rullst tem um único mantenedor",
              "banned": ("revolucionári", "o melhor", "nenhum outro framework", "o único framework", "framework único",
                         "mais seguro", "experimental", "de classe mundial")},
}
# The landing sections, in page order.
LANDING_SECTIONS = ("performance", "security", "included", "integrations", "examples", "academy", "community")
LANDING_CHECKED = re.compile(r'<p class="(?:landing-checked|security-compare)" id="(checked-[a-z]+)">(.*?)</p>', re.DOTALL)


def validate_language_links(path: Path, source: str, page: str, lang: str, pt_page: str | None = None) -> None:
    """Language attribute, hreflang alternates and the flag switcher.

    PAGE is the English path below the site root; PT_PAGE the Portuguese one
    (default: the same path under pt-br/).
    """
    english, portuguese = f"/{page}", f"/pt-br/{page}" if pt_page is None else f"/{pt_page}"
    assert f'<html lang="{lang}">' in source, f"{path.name}: <html lang> must be {lang}"
    for hreflang, href in (("en", english), ("pt-BR", portuguese), ("x-default", english)):
        alternate = f'<link rel="alternate" hreflang="{hreflang}" href="{SITE_URL}{href[1:]}" />'
        assert alternate in source, f"{path.name}: missing {alternate}"
    switcher = re.search(r'<div class="lang-switch" role="group" aria-label="[^"]+">(.*?)</div>', source, re.DOTALL)
    assert switcher, f"{path.name}: missing the language switcher"
    entries = re.findall(r'<a href="([^"]+)" hreflang="([^"]+)" lang="([^"]+)"([^>]*)><img src="([^"]+)" alt="" [^>]*>([^<]+)</a>', switcher[1])
    assert [(href, hreflang, flag, text) for href, hreflang, _lang, _rest, flag, text in entries] == [
        (english, "en", "/assets/flags/us.svg", "English"),
        (portuguese, "pt-BR", "/assets/flags/br.svg", "Português"),
    ], f"{path.name}: language switcher entries: {entries}"
    current = [hreflang for _href, hreflang, _lang, rest, _flag, _text in entries if 'aria-current="page"' in rest]
    assert current == [lang], f"{path.name}: aria-current must mark only the {lang} entry"


def validate_landing(lang: str) -> tuple[Document, str]:
    facts = LANGUAGES[lang]
    path = facts["home"]
    home, source = parse(path)
    validate_document(path, home)
    validate_language_links(path, source, "", lang)
    assert home.has_main and home.has_nav and home.has_footer, f"{path.name}: needs nav, main and footer"
    assert source.count(facts["dedication"]) == 1, f"{path.name}: needs the footer dedication"
    assert facts["status"] in source, f"{path.name}: must identify the v13 prerelease line"
    assert facts["slogan"] in source, f"{path.name}: must preserve the project slogan"
    assert "Rullst v12 stable" not in source, f"{path.name}: still presents end-of-life v12 as current"
    assert "evaluate before stable v12" not in source, f"{path.name}: retains obsolete RC copy"
    assert "/Rullst/book/v12.html" not in home.links, f"{path.name}: release-status links must not point to the v12 record"
    cli_version = CLI_VERSION.search((ROOT / "cargo-rullst" / "Cargo.toml").read_text(encoding="utf-8"))
    assert cli_version, "cannot read the cargo-rullst version"
    pins = INSTALL_PIN.findall(source)
    assert pins and set(pins) == {cli_version[1]}, (
        f"{path.name}: install command must pin the current CLI version ={cli_version[1]}: {pins}"
    )
    assert "Content-Security-Policy" in source, f"{path.name}: must declare a CSP"
    assert f'<link rel="canonical" href="{facts["url"]}" />' in source, f"{path.name}: canonical must be {facts['url']}"
    assert f'<meta property="og:url" content="{facts["url"]}" />' in source, f"{path.name}: og:url must be {facts['url']}"
    assert f'<meta property="og:locale" content="{facts["locale"]}" />' in source, f"{path.name}: og:locale"
    assert "academy" in home.ids and any(link.startswith("https://academy.rullst.win/") for link in home.links), (
        f"{path.name}: must keep the Academy section"
    )
    validate_landing_sections(path, source, home, facts)
    validate_made_with(path, source, facts)
    assert source.count('class="demo-host-note"') == 4, f"{path.name}: every Azure-hosted demo needs its startup note"
    assert not home.inline_behavior, f"{path.name}: inline behavior/style: {home.inline_behavior}"
    for references, filename in ((home.stylesheets, "site.css"), (home.scripts, "site.js")):
        digest = sha256((DOCS / filename).read_bytes()).hexdigest()[:16]
        expected = f"/assets/{filename}?v={digest}"
        assert references == [expected], f"{path.name}: use the current content-versioned asset {expected}"
    assert not home.embeds, f"{path.name}: privacy contract forbids third-party embeds"
    assert "privacy" in home.ids, f"{path.name}: must expose its privacy notice"
    assert 'name="referrer" content="no-referrer"' in source, f"{path.name}: must suppress outgoing referrers"
    assert not any(urlparse(src).netloc for src in home.images), f"{path.name}: images must be same-origin"
    assert "examples" in home.ids, f"{path.name}: must expose the demo directory"
    assert DEMO_LINKS <= set(home.links), f"{path.name}: missing a published example link"
    assert SOCIAL_LINKS <= set(home.links), f"{path.name}: missing requested community links"
    for reference in [*home.links, *home.stylesheets, *home.scripts, *home.images]:
        if reference.startswith("#"):
            assert reference[1:] in home.ids, f"{path.name}: missing anchor {reference}"
            continue
        target = local_home_target(reference)
        if target is not None:
            assert target.is_file(), f"{path.name}: missing local target {reference} -> {target}"
    return home, source


def section_text(source: str, ident: str) -> str:
    match = re.search(rf'<section id="{ident}" .*?</section>', source, re.DOTALL)
    assert match, f"landing is missing the #{ident} section"
    return match[0]


def validate_landing_sections(path: Path, source: str, home: Document, facts: dict) -> None:
    """The first screen, Performance, Security and Included sections and their evidence rules."""
    order = [source.index(f'<section id="{ident}" ') for ident in LANDING_SECTIONS]
    assert order == sorted(order), f"{path.name}: sections must come in the order {LANDING_SECTIONS}"
    assert "verification" in home.ids, f"{path.name}: Security needs the #verification list the first screen links"
    hero = source[source.index('<header class="hero'):source.index("</header>")]
    assert facts["lede"] in hero, f"{path.name}: the first screen must say what Rullst is in one sentence"
    assert hero.count('<li class="proof-card ') == 3, f"{path.name}: the first screen needs three proof points"
    assert INSTALL_PIN.search(hero), f"{path.name}: the first screen must show the install command"
    engine = re.search(r'<div class="engine" aria-label="([^"]+)">', hero)
    assert engine and facts["illustration"] in engine[1] and hero.count('class="engine-module ') == 3, (
        f"{path.name}: the first screen keeps the animated concept illustration, labelled as not live telemetry"
    )
    for name in ("proof", "summary"):
        start = GENERATOR_BLOCKS[name][0]
        assert source.count(start) == 1, f"{path.name}: needs one {start} block"
        assert landing_pending(source, name), f"{path.name}: {start} needs its pending text"
    performance = section_text(source, "performance")
    assert {"/Rullst/benches/", "/Benchmarks/"} <= set(re.findall(r'href="([^"]+)"', performance)), (
        f"{path.name}: Performance must link the microbenchmarks and the full results"
    )
    assert facts["performance"] in performance, f"{path.name}: Performance must say when the cross-framework comparison is measured"
    assert facts["disclosure"] in performance and performance.index(facts["disclosure"]) < performance.index(GENERATOR_BLOCKS["summary"][0]), (
        f"{path.name}: Performance must say who maintains the benchmark before the results"
    )
    security = section_text(source, "security")
    for statement in ("no_audit", "maintainer"):
        assert facts[statement] in security, f"{path.name}: Security must say: {facts[statement]}"
    text = plain_html(source[source.index("<body"):]).lower()
    banned = [word for word in facts["banned"] if word in text]
    assert not banned, f"{path.name}: marketing or absolute wording on the landing: {banned}"
    # A statement about another framework names the version and the day it was checked.
    for ident in ("security", "included"):
        body = section_text(source, ident)
        if "Loco" in plain_html(body) or "Topcoat" in plain_html(body):
            checked = LANDING_CHECKED.search(body)
            assert checked and re.search(r'<time datetime="\d{4}-\d{2}-\d{2}">', checked[2]), (
                f"{path.name}: #{ident} mentions other frameworks without the date it checked them"
            )
            versions = re.findall(r"<code>((?:loco-rs|topcoat) [0-9][^<]*)</code>", checked[2])
            assert {version.split()[0] for version in versions} == {"loco-rs", "topcoat"}, (
                f"{path.name}: #{ident} must name the loco-rs and topcoat versions it checked: {versions}"
            )


def landing_checked_lines(source: str) -> list[tuple[str, list]]:
    """Each landing checked line with its versions and date, for the translation check."""
    return [(ident, re.findall(r"<code>(.*?)</code>|<time datetime=\"([0-9-]+)\">", body))
            for ident, body in LANDING_CHECKED.findall(source)]


def validate_privacy(lang: str, home: Document) -> tuple[Document, str]:
    facts = LANGUAGES[lang]
    privacy, source = validate_static_page(facts["privacy"], lang, "privacy.html", None, home)
    assert "Venelouis" in source, f"{facts['privacy'].name}: must name the responsible person"
    return privacy, source


def validate_static_page(path: Path, lang: str, page: str, pt_page: str | None, home: Document) -> tuple[Document, str]:
    """Checks shared by the script-free pages (privacy notice, comparison)."""
    privacy, source = parse(path)
    validate_document(path, privacy)
    validate_language_links(path, source, page, lang, pt_page)
    assert privacy.has_main and privacy.has_nav and privacy.has_footer, f"{path.name}: needs nav, main and footer"
    assert "script-src 'none'" in source, f"{path.name}: must forbid scripts"
    assert not privacy.scripts and not privacy.inline_behavior and not privacy.embeds, f"{path.name}: must stay static"
    url = SITE_URL + (page if lang == "en" else (pt_page or f"pt-br/{page}"))
    assert f'<link rel="canonical" href="{url}" />' in source, f"{path.name}: canonical URL must be {url}"
    assert "connect-src 'none'" in source, f"{path.name}: must forbid connections"
    assert 'name="referrer" content="no-referrer"' in source, f"{path.name}: must suppress outgoing referrers"
    css_version = sha256((DOCS / "site.css").read_bytes()).hexdigest()[:16]
    assert privacy.stylesheets == [f"/assets/site.css?v={css_version}"], f"{path.name}: must load the current stylesheet"
    landing_prefix = "/" if lang == "en" else "/pt-br/"
    for reference in [*privacy.links, *privacy.stylesheets, *privacy.images]:
        if reference.startswith("#"):
            assert reference[1:] in privacy.ids, f"{path.name}: missing anchor {reference}"
        elif reference.startswith(landing_prefix + "#"):
            assert reference.removeprefix(landing_prefix + "#") in home.ids, f"{path.name}: missing landing anchor {reference}"
        elif (target := local_home_target(reference)) is not None:
            assert target.is_file(), f"{path.name}: missing local target {reference} -> {target}"
    return privacy, source


COMPARE_PAGE = "compare/index.html"
CHECKED_CODE = re.compile(r'<p class="compare-checked" id="([a-z-]+)">(.*?)</p>', re.DOTALL)
# Each "checked" line, in page order, and the crates.io packages whose
# versions it must name: the topic-by-topic comparison, then the
# "Why not Leptos or Dioxus?" section.
CHECKED_PACKAGES = {"checked": ("loco-rs", "topcoat"), "checked-ui": ("leptos", "dioxus")}
CHECKED_PACKAGE_VERSION = re.compile(r"([a-z0-9_-]+) [0-9][0-9A-Za-z.+-]*")
UI_FRAMEWORKS = re.compile(r'<section id="ui-frameworks" [^>]*>(.*?)</section>', re.DOTALL)


def validate_comparison(home: Document, home_pt: Document) -> None:
    """The comparison pages: same facts in both languages, data-driven benchmarks."""
    compare, source = validate_static_page(COMPARE, "en", "compare/", "pt-br/comparar/", home)
    compare_pt, source_pt = validate_static_page(COMPARE_PT_BR, "pt-BR", "compare/", "pt-br/comparar/", home_pt)
    validate_translation(compare, source, compare_pt, source_pt, COMPARE_PT_BR.name)
    checked = [{match[1]: match[2] for match in CHECKED_CODE.finditer(text)} for text in (source, source_pt)]
    assert all(list(lines) == list(CHECKED_PACKAGES) for lines in checked), (
        f"comparison pages must state what each fact was checked against, in this order: {list(CHECKED_PACKAGES)}; "
        f"found {[list(lines) for lines in checked]}"
    )
    for identifier, packages in CHECKED_PACKAGES.items():
        versions = [re.findall(r"<code>(.*?)</code>|<time datetime=\"([0-9-]+)\">", lines[identifier]) for lines in checked]
        assert versions[0] == versions[1], f"comparison pages checked different versions or dates in #{identifier}: {versions}"
        assert any(date for _code, date in versions[0]), f"comparison #{identifier} must give the date it was checked"
        named = [match[1] for code, _date in versions[0] if (match := CHECKED_PACKAGE_VERSION.fullmatch(code))]
        assert all(package in named for package in packages), f"comparison #{identifier} must name the {packages} versions it checked"
        for package in named:
            for document, name in ((compare, COMPARE.name), (compare_pt, COMPARE_PT_BR.name)):
                assert f"https://crates.io/crates/{package}" in document.links, f"{name}: Sources must link {package} on crates.io"
    # "Why not Leptos or Dioxus?": its own checked line, between the topics and Performance.
    for text, name in ((source, COMPARE.name), (source_pt, COMPARE_PT_BR.name)):
        section = UI_FRAMEWORKS.search(text)
        assert section and 'id="checked-ui"' in section[1], f"{name}: needs the ui-frameworks section with its checked line"
        heading = re.search(r"<h2 [^>]*>(.*?)</h2>", section[1])
        assert heading and "Leptos" in heading[1] and "Dioxus" in heading[1], f"{name}: ui-frameworks heading must name Leptos and Dioxus"
        assert text.index('id="topics"') < section.start() < text.index('id="benchmarks"'), (
            f"{name}: the ui-frameworks section goes between the topics and Performance"
        )
    topics = [re.findall(r'<div class="compare-topic" id="([a-z-]+)">', text) for text in (source, source_pt)]
    assert len(topics[0]) >= 11 and topics[0] == topics[1], f"comparison topics differ: {topics}"
    for text, name in ((source, COMPARE.name), (source_pt, COMPARE_PT_BR.name)):
        for topic in re.findall(r'<div class="compare-topic" id="[a-z-]+">(.*?)</dl>', text, re.DOTALL):
            assert re.findall(r"<dt>(.*?)</dt>", topic) == ["Rullst", "Loco", "Topcoat"], f"{name}: each topic covers the three"
        assert text.count(RESULTS_BLOCK[0]) == 1 and 'class="benchmark-pending"' in text, (
            f"{name}: needs the benchmark-results block with its pending text"
        )
    # "How to read these numbers": the same six points in both languages, naming
    # the minimal row as the generator labels it, before the results block.
    for text, name, lang in ((source, COMPARE.name, "en"), (source_pt, COMPARE_PT_BR.name, "pt-BR")):
        howto = re.search(r'<div class="benchmark-howto">(.*?)</div>', text, re.DOTALL)
        assert howto and len(re.findall(r"<li>", howto[1])) == 6, f"{name}: needs the six-point benchmark-howto block"
        assert text.index('class="benchmark-howto"') < text.index(RESULTS_BLOCK[0]), f"{name}: benchmark-howto goes before the results"
        minimal = MINIMAL_NAMES[lang]
        assert f'"{minimal}"' in howto[1] and "cargo rullst new" in howto[1], f"{name}: benchmark-howto must explain {minimal}"
        assert all(sign in howto[1] for sign in ("1.00×" if lang == "en" else "1,00×", "±", "3–7")), (
            f"{name}: benchmark-howto must explain the 1.00× reference, the ± margin and position ranges"
        )
    assert "/compare/" in home.links and "/pt-br/comparar/" in home_pt.links, "landing pages must link the comparison"
    validate_benchmark_rendering()


PENDING = 'class="benchmark-pending"'
# Text each language's page must contain when rendered from the fixture.
FIXTURE_RENDERING = {
    COMPARE_PAGE: ("en", (
        '<time datetime="2026-11-20">20 November 2026</time> with Rullst 13.0.0-alpha.6 from crates.io, against Axum 0.8.9',
        '<a href="https://github.com/Rullst/Benchmarks/actions/runs/1">measurement run 1</a>',
        '<a href="/Benchmarks/">full results</a> list all 14 ranked implementations',
        '<span class="benchmark-position">3–4</span><span class="benchmark-score">0.61× ‡</span><span class="benchmark-margin">±5.5%</span>',
        '<span class="benchmark-score">0.90×* ‡</span>', '<span class="benchmark-score">0.070×*</span>',
        '<span class="benchmark-score">1.19× †</span>', '<span class="benchmark-score">0.98×* ‡ ≈</span>',
        '<td class="benchmark-status">errors</td>', '<td class="benchmark-status">incomplete</td>', "<td>warn</td>",
        '<td>1,045 <span class="benchmark-axum">(929)</span></td>', "Peak after the 125 logins",
        '<p class="benchmark-note">Fixture note: a note about this Rullst version.</p>',
        "<h3>What this means for Rullst</h3>",
        '<p class="benchmark-reading">Fixture reading: these numbers are not measurements of <code>cargo rullst new</code> starters; '
        "`&lt;b&gt;` and a lone ` stay text.</p>",
    )),
    "pt-br/comparar/index.html": ("pt-BR", (
        '<time datetime="2026-11-20">20 de novembro de 2026</time> com o Rullst 13.0.0-alpha.6 do crates.io, contra o Axum 0.8.9',
        '<a href="https://github.com/Rullst/Benchmarks/actions/runs/1">execução da medição 1</a>',
        '<a href="/Benchmarks/">resultados completos</a> (em inglês) listam as 14 implementações classificadas',
        '<span class="benchmark-position">3–4</span><span class="benchmark-score">0,61× ‡</span><span class="benchmark-margin">±5,5%</span>',
        '<span class="benchmark-score">0,90×* ‡</span>', '<span class="benchmark-score">0,070×*</span>',
        '<span class="benchmark-score">1,19× †</span>', '<span class="benchmark-score">0,98×* ‡ ≈</span>',
        '<td class="benchmark-status">erros</td>', '<td class="benchmark-status">incompleto</td>', "<td>alerta</td>",
        '<td>1.045 <span class="benchmark-axum">(929)</span></td>', "Pico depois dos 125 logins",
        '<p class="benchmark-note" lang="en">Fixture note: a note about this Rullst version.</p>',
        "<h3>O que isto significa para o Rullst</h3>",
        '<p class="benchmark-reading">Leitura do fixture: estes números não são medições de projetos <code>cargo rullst new</code>; '
        "`&lt;b&gt;` e uma ` sozinha ficam como texto.</p>",
    )),
}


# The landing blocks rendered from the fixture (Rullst overall 0.61× ‡ in 3–4,
# shared with Loco; the minimal row; RAM 12/470 MiB against Axum's 17/927).
LANDING_RENDERING = {
    "index.html": ("en", (
        '<strong class="landing-bench-score">0.61× ‡</strong><span class="landing-bench-position">positions 3–4 of 14</span>',
        "It shares positions 3–4 with Loco (0.63×).",
        '<h3>Level with or above Axum</h3><ul><li><span class="landing-bench-category">Memory</span><span class="landing-bench-value">1.00× ≈</span>',
        '<h3>Below Axum</h3><ul><li><span class="landing-bench-category">Hello world</span><span class="landing-bench-value">0.48×*</span>',
        '<span class="landing-bench-value">0.81×</span><span class="landing-bench-position">position 2 of 14</span>',
        '<span class="landing-bench-value">0.90×* ‡</span>',
        "RAM: 12 MiB at the high-water mark under load (Axum: 17 MiB); 470 MiB after the 125 logins",
        "Stability under heavier load: pass.",
        "scores Hello world 1.00×* ≈, Database 0.87×*, Application 0.98×* ‡ ≈; the gap between the two rows is what those layers cost.",
        "<strong>12 MiB</strong> of RAM at its high-water mark under load and 470 MiB after 125 logins; Axum: 17 and 927 MiB",
        "Its lowest speed score is Hello world: 0.48× Axum's.",
        'Measured by Rullst/Benchmarks on <time datetime="2026-11-20">20 November 2026</time>.',
    )),
    "pt-br/index.html": ("pt-BR", (
        '<strong class="landing-bench-score">0,61× ‡</strong><span class="landing-bench-position">posições 3–4 de 14</span>',
        "Divide as posições 3–4 com Loco (0,63×).",
        '<h3>No nível do Axum ou acima</h3><ul><li><span class="landing-bench-category">Memória</span><span class="landing-bench-value">1,00× ≈</span>',
        '<h3>Abaixo do Axum</h3><ul><li><span class="landing-bench-category">Hello world</span><span class="landing-bench-value">0,48×*</span>',
        '<span class="landing-bench-value">0,81×</span><span class="landing-bench-position">posição 2 de 14</span>',
        "RAM: 12 MiB no pico sob carga (Axum: 17 MiB); 470 MiB depois dos 125 logins",
        "Estabilidade sob carga mais pesada: passou.",
        "pontua Hello world 1,00×* ≈, Banco de dados 0,87×*, Aplicação 0,98×* ‡ ≈;",
        "<strong>12 MiB</strong> de RAM no pico sob carga e 470 MiB depois de 125 logins; Axum: 17 e 927 MiB",
        "A menor pontuação de velocidade é a de Hello world: 0,48× a do Axum.",
        'Medido pelo Rullst/Benchmarks em <time datetime="2026-11-20">20 de novembro de 2026</time>.',
    )),
}


def landing_block(rendered: str, name: str) -> str:
    start, end = GENERATOR_BLOCKS[name]
    begin = rendered.index(start) + len(start)
    return rendered[begin:rendered.index(end, begin)]


def landing_pending(rendered: str, name: str) -> bool:
    return 'class="benchmark-pending' in landing_block(rendered, name)


def results_block(rendered: str) -> str:
    start, end = RESULTS_BLOCK
    begin = rendered.index(start) + len(start)
    return rendered[begin:rendered.index(end, begin)]


def table_rows(results: str) -> list[str]:
    """The row names of the first (scores) table."""
    table = results[:results.index("</table>")]
    return re.findall(r'<tr(?: class="benchmark-diagnostic")?><th scope="row">([^<]+)<', table)


# The files the generator reads from its working directory, by fixed name.
BENCHMARK_INPUT = "benchmarks.json"
FOOTPRINT_INPUTS = ("footprint.json", "footprint-pt-br.json")


def write_input(path: Path, text: str) -> None:
    """Write one downloaded-data stand-in for the generator."""
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(text)


def generate(work: Path, benchmarks: str | None = None, today: str | None = None, max_age_days: int | None = None,
             footprint: dict[str, str] | None = None) -> tuple[Path, str]:
    """Run the generator in WORK (a new directory) with the given downloaded data; its output and warnings."""
    work.mkdir(parents=True)
    args = [str(GENERATOR), "--out", str(work / "site")]
    if benchmarks is not None:
        write_input(work / BENCHMARK_INPUT, benchmarks)
        args.append("--benchmarks")
    if max_age_days is not None:
        args += ["--benchmarks-max-age-days", str(max_age_days)]
    if footprint is not None:
        for name in FOOTPRINT_INPUTS:
            if name in footprint:
                write_input(work / name, footprint[name])
        args.append("--footprint")
    if today is not None:
        args += ["--today", today]
    result = subprocess.run(args, cwd=work, capture_output=True, text=True, timeout=60, check=False)
    assert result.returncode == 0, f"the generator failed: {result.stderr}"
    return work / "site", result.stderr


def fixture_variant(change) -> str:
    data = copy.deepcopy(json.loads(BENCHMARK_FIXTURE.read_text(encoding="utf-8")))
    change(data)
    return json.dumps(data)


def fixture_row(data: dict, slug: str) -> dict:
    return next(row for row in data["rows"] if row["slug"] == slug)


# Data the page must not show (it keeps its pending text) and the warning that says why.
REJECTED = (
    ("the earlier latest.v1 shape", lambda d: d.update(schema="rullst.benchmarks.latest.v1"), "schema is not"),
    ("a date after today", lambda d: d.update(measured_on="2026-11-22"), "after today"),
    ("results older than the limit", lambda d: d.update(measured_on="2026-08-22"), "more than 90 days ago"),
    ("a flag the page does not explain", lambda d: fixture_row(d, "rullst")["categories"]["hello"]["flags"].append("new_flag"), "unknown flag"),
    ("a new flag sign table", lambda d: d["flags"].update(new_flag="§"), "the flags are not"),
    ("other overall weights", lambda d: d["weights"].update(hello=1), "overall weights"),
    ("another category", lambda d: d["categories"].append({"key": "latency", "title": "Latency"}), "the categories are not"),
    ("an unknown status", lambda d: fixture_row(d, "loco")["categories"]["database"].update(status="slow"), "unknown status"),
    ("an unknown stability result", lambda d: fixture_row(d, "loco").update(stability="unknown"), "stability"),
    ("a position past the ranked count", lambda d: fixture_row(d, "loco")["overall"].update(position=[3, 15]), "position"),
    ("a ranked minimal row", lambda d: fixture_row(d, "rullst-minimal").update(role="ranked"), "expected the role"),
    ("a negative score", lambda d: fixture_row(d, "gin")["categories"]["hello"].update(score=-1), "finite number"),
    ("an unexpected version", lambda d: fixture_row(d, "rullst").update(version="<b>"), "unexpected version"),
    ("a run outside Rullst/Benchmarks", lambda d: d["run"].update(url="javascript:alert(1)"), "unexpected run url"),
    ("another reference", lambda d: d["reference"].update(slug="actix", framework="Actix Web"), "reference is not Axum"),
    ("no Portuguese reading", lambda d: d["reading"].update(pt_br=""), "reading is empty"),
    ("a duplicated row", lambda d: d["rows"].append(copy.deepcopy(fixture_row(d, "gin"))), "appears twice"),
)


UNPUBLISHED = "This run publishes no overall score; the categories are the result."
BENCHMARK_MAX_AGE_DAYS = 90


def validate_benchmark_rendering() -> None:
    """The results block rendered from the fixture, and the data that must keep the pending text."""
    with tempfile.TemporaryDirectory(prefix="rullst-root-site-") as temp:
        site, _warnings = generate(Path(temp) / "data", BENCHMARK_FIXTURE.read_text(encoding="utf-8"), FIXTURE_DAY.isoformat())
        for page, (lang, expected) in FIXTURE_RENDERING.items():
            rendered = (site / page).read_text(encoding="utf-8")
            document = Document()
            document.feed(rendered)
            validate_document(Path(page), document)
            assert PENDING not in rendered, f"{page}: benchmark rows were not rendered"
            missing = [text for text in expected if text not in rendered]
            assert not missing, f"{page}: unexpected benchmark rendering, missing {missing}"
            results = results_block(rendered)
            # The fixture's FastAPI row is not one the page shows.
            assert "FastAPI" not in results, f"{page}: renders a framework outside the comparison"
            # By overall position (Loco before Rullst in their shared 3–4 by score),
            # the row without an overall position after them, the diagnostic last.
            rows = table_rows(results)
            assert rows == ["Actix Web", "Topcoat", "Loco", "Rullst", "Gin", MINIMAL_NAMES[lang]], f"{page}: benchmark rows {rows}"
            legend = re.search(r'<ul class="benchmark-legend">(.*?)</ul>', results, re.DOTALL)
            assert legend and all(sign in legend[1] for sign in FLAG_SIGNS), f"{page}: the legend must explain every sign shown"
        for page, (lang, expected) in LANDING_RENDERING.items():
            rendered = (site / page).read_text(encoding="utf-8")
            document = Document()
            document.feed(rendered)
            validate_document(Path(page), document)
            assert not landing_pending(rendered, "proof") and not landing_pending(rendered, "summary"), f"{page}: landing blocks were not rendered"
            missing = [text for text in expected if text not in rendered]
            assert not missing, f"{page}: unexpected landing rendering, missing {missing}"
            summary = landing_block(rendered, "summary")
            legend = re.search(r'<ul class="benchmark-legend landing-bench-legend">(.*?)</ul>', summary, re.DOTALL)
            cells = re.findall(r'<span class="landing-bench-value">([^<]*)</span>|<strong class="landing-bench-score">([^<]*)</strong>', summary)
            values = "".join(value + score for value, score in cells)
            values += re.search(r'<p class="landing-bench-minimal">(.*?)</p>', summary, re.DOTALL)[1]
            signs = {sign for sign in FLAG_SIGNS if sign in values}
            assert legend and signs and all(f"<li>{sign} " in legend[1] for sign in signs), f"{page}: the legend must explain {signs}"
            assert "database_note" not in summary and "†" not in summary, f"{page}: the landing explains a sign it does not show"
    # Note rendering (backtick spans) is unit-tested in site/src/benchmarks/format.rs.
    assert (FIXTURE_DAY - date(2026, 8, 22)).days > BENCHMARK_MAX_AGE_DAYS >= (FIXTURE_DAY - date(2026, 11, 20)).days, (
        "the fixture's dates must test both sides of the default age limit"
    )
    with tempfile.TemporaryDirectory(prefix="rullst-root-site-") as temp:
        for index, (label, change, warning) in enumerate(REJECTED):
            site, warnings = generate(Path(temp) / f"rejected-{index}", fixture_variant(change), FIXTURE_DAY.isoformat())
            assert warning in warnings, f"{label}: expected a warning with {warning!r}, got {warnings!r}"
            for page in FIXTURE_RENDERING:
                rendered = (site / page).read_text(encoding="utf-8")
                assert PENDING in rendered and "benchmark-table" not in rendered, f"{label}: {page} must keep the pending text"
            for page in LANDING_RENDERING:
                rendered = (site / page).read_text(encoding="utf-8")
                assert landing_pending(rendered, "proof") and landing_pending(rendered, "summary"), f"{label}: {page} must keep the pending text"
                assert "landing-bench-split" not in rendered and "MiB" not in rendered, f"{label}: {page} shows numbers"
        # The age limit is configurable: the same results within a longer limit are shown.
        site, _warnings = generate(Path(temp) / "longer-limit", fixture_variant(lambda d: d.update(measured_on="2026-08-22")),
                                   FIXTURE_DAY.isoformat(), max_age_days=100)
        assert PENDING not in (site / COMPARE_PAGE).read_text(encoding="utf-8"), "--benchmarks-max-age-days must apply"
        # The generator reads only benchmarks.json in its working directory, never a link out of it.
        work = Path(temp) / "outside"
        work.mkdir()
        shutil.copyfile(BENCHMARK_FIXTURE, Path(temp) / "elsewhere.json")
        (work / BENCHMARK_INPUT).symlink_to(Path(temp) / "elsewhere.json")
        result = subprocess.run([str(GENERATOR), "--out", str(work / "site"), "--benchmarks", "--today", FIXTURE_DAY.isoformat()],
                                cwd=work, capture_output=True, text=True, timeout=60, check=False)
        assert result.returncode == 0 and "inside the working directory" in result.stderr, f"a results link out of the working directory: {result.stderr!r}"
        assert PENDING in (work / "site" / COMPARE_PAGE).read_text(encoding="utf-8"), "results read through a link must not be shown"
        # The minimal row is extra: data without it shows the other rows and no
        # placeholder for it; a missing compared framework says it is not in this run.
        site, _warnings = generate(Path(temp) / "no-minimal", fixture_variant(
            lambda d: d.update(rows=[r for r in d["rows"] if r["slug"] not in ("rullst-minimal", "gin")])), FIXTURE_DAY.isoformat())
        results = results_block((site / COMPARE_PAGE).read_text(encoding="utf-8"))
        assert table_rows(results) == ["Actix Web", "Topcoat", "Loco", "Rullst", "Gin"], f"rows without the minimal one: {table_rows(results)}"
        assert '<td colspan="7">Not in this measurement run.</td>' in results, "a missing framework must say it is not in this run"
        # Without a published overall score the page has no Overall column and says so.
        site, _warnings = generate(Path(temp) / "no-overall", fixture_variant(lambda d: d.update(overall_published=False, weights={})),
                                   FIXTURE_DAY.isoformat())
        results = results_block((site / COMPARE_PAGE).read_text(encoding="utf-8"))
        assert ">Overall<" not in results and UNPUBLISHED in results, "an unpublished overall score must not be shown"
        assert table_rows(results)[:3] == ["Rullst", "Loco", "Topcoat"], "without an overall score the rows keep the fixed order"
        # A source the page does not name is left out of the meta line, not rendered.
        site, _warnings = generate(Path(temp) / "other-source", fixture_variant(lambda d: d.update(rullst_source={"kind": ["crates.io"]})),
                                   FIXTURE_DAY.isoformat())
        results = results_block((site / COMPARE_PAGE).read_text(encoding="utf-8"))
        assert "with Rullst 13.0.0-alpha.6, against Axum" in results, "an unknown Rullst source must be left out"
        validate_landing_variants(Path(temp))


def generate_variant(temp: Path, name: str, change) -> tuple[str, str, str]:
    """English landing, comparison page and warnings for one fixture variant."""
    site, warnings = generate(temp / name, fixture_variant(change), FIXTURE_DAY.isoformat())
    return ((site / "index.html").read_text(encoding="utf-8"), (site / COMPARE_PAGE).read_text(encoding="utf-8"), warnings)


def validate_landing_variants(temp: Path) -> None:
    """How the landing summary words groups, a missing Rullst row, memory or overall score."""
    def position(slugs, first, last):
        return lambda d: [fixture_row(d, slug)["overall"].update(position=[first, last]) for slug in slugs]
    for name, change, sentence in (
        ("group-partial", position(("rullst", "loco"), 3, 5), "It shares positions 3–5 with 2 other implementations, among them Loco (0.63×)."),
        ("group-unnamed", position(("rullst",), 6, 7), "It shares positions 6–7 with one other implementation of the full results."),
        ("group-unnamed-many", position(("rullst",), 6, 8), "It shares positions 6–8 with 2 other implementations of the full results."),
        ("group-alone", position(("rullst",), 4, 4), "No other implementation shares its position."),
        ("group-named", position(("rullst", "loco", "topcoat"), 3, 5), "It shares positions 3–5 with Topcoat (0.99×) and Loco (0.63×)."),
    ):
        landing, _compare, _warnings = generate_variant(temp, name, change)
        assert sentence in landing, f"{name}: expected {sentence!r} in {landing_block(landing, 'summary')[:600]!r}"
    # Gin's fixture overall has no score, so a shared range does not name it.
    landing, _compare, _warnings = generate_variant(temp, "no-gin-score", position(("rullst", "loco", "gin"), 3, 5))
    assert "Gin" not in landing_block(landing, "summary"), "a row without an overall score is not named in the group"
    landing, compare, warnings = generate_variant(temp, "no-rullst", lambda d: d.update(rows=[r for r in d["rows"] if r["slug"] != "rullst"]))
    assert PENDING not in compare and landing_pending(landing, "summary") and landing_pending(landing, "proof"), (
        "without a Rullst row the comparison renders and the landing keeps its pending text"
    )
    assert "no Rullst row" in warnings, f"a missing Rullst row must be logged: {warnings!r}"
    landing, _compare, _warnings = generate_variant(temp, "no-memory", lambda d: fixture_row(d, "rullst").update(memory={}))
    assert landing_pending(landing, "proof") and not landing_pending(landing, "summary") and "RAM:" not in landing, (
        "without Rullst's RAM the proof point stays pending and the summary leaves memory out"
    )
    landing, _compare, _warnings = generate_variant(temp, "landing-no-overall", lambda d: d.update(overall_published=False, weights={}))
    summary = landing_block(landing, "summary")
    assert UNPUBLISHED in summary and "Overall score" not in summary, "an unpublished overall score is not shown"
    landing, _compare, _warnings = generate_variant(
        temp, "landing-errors", lambda d: fixture_row(d, "rullst")["categories"]["database"].update(score=None, position=None, status="errors"))
    summary = landing_block(landing, "summary")
    assert '<h3>Without a score</h3><ul><li><span class="landing-bench-category">Database</span><span class="landing-bench-value">errors</span>' in summary, (
        "a category without a score is listed with its status"
    )


# The first screen's line on the parts around the framework, each linked to its documentation.
AROUND_LINKS = [
    "/Rullst/book/crates/ai.html", "/Rullst/book/crates/capital.html", "/Rullst/book/crates/mail.html",
    "/Rullst/book/crates/messaging.html", "/Rullst/book/crates/iot.html", "/Rullst/book/labs-runner-contract.html",
    "https://github.com/Rullst/Rullst/tree/main/rullst-privacy",
]
# The "Made with Rullst" card: the maintainer's live applications built with Rullst.
MADE_WITH_LINKS = {
    "en": ["https://academy.rullst.win/", "https://capoeira.vene.win/", "https://vene.win/", "/Benchmarks/",
           "https://github.com/Rullst/Rullst/tree/main/site"],
    "pt-BR": ["https://academy.rullst.win/pt-BR", "https://capoeira.vene.win/", "https://vene.win/", "/Benchmarks/",
              "https://github.com/Rullst/Rullst/tree/main/site"],
}
SITE_LOCK_RULLST = re.compile(r'^name = "rullst"\nversion = "([^"]+)"$', re.MULTILINE)
CLI_PIN = re.compile(r"([0-9a-f]{64})  cargo-rullst-(\S+)-x86_64-unknown-linux-gnu\n")


def site_rullst_version() -> str:
    """The `rullst` version the generator is built with (site/Cargo.lock)."""
    match = SITE_LOCK_RULLST.search((ROOT / "site/Cargo.lock").read_text(encoding="utf-8"))
    assert match, "site/Cargo.lock has no rullst package"
    return match[1]


def made_with_line(source: str) -> str:
    match = re.search(r'<(p|span) class="made-with-line">(.*?)</\1>', source, re.DOTALL)
    return match[2] if match else ""


def validate_made_with(path: Path, source: str, facts: dict) -> None:
    """The first screen's line on the parts around the framework, the card and the footer line."""
    hero = source[source.index('<header class="hero'):source.index("</header>")]
    around = re.search(r'<p class="hero-around">(.*?)</p>', hero, re.DOTALL)
    assert around and around[1].startswith(facts["around"]), f"{path}: the first screen names the parts around the framework"
    assert re.findall(r'href="([^"]+)"', around[1]) == AROUND_LINKS, f"{path}: each part around the framework links its documentation"
    card = re.search(r'<article class="made-with" id="made-with" [^>]*>(.*?)</article>', section_text(source, "examples"), re.DOTALL)
    lang = "en" if facts["locale"] == "en_US" else "pt-BR"
    assert card and re.findall(r'<li><a href="([^"]+)">', card[1]) == MADE_WITH_LINKS[lang], f"{path}: the Made with Rullst card lists the live applications"
    line = made_with_line(source[source.index("<footer"):])
    assert line.startswith(f'{facts["made_with"]}<a href="https://github.com/Rullst/Rullst/tree/main/site">Rullst {site_rullst_version()}</a>'), (
        f"{path}: the footer says which Rullst version made the page: {line[:200]!r}"
    )
    assert "This page:" not in line and "Esta página:" not in line, f"{path}: no page weight without a measurement"


def validate_footer_lines(site: Path) -> None:
    for page in site.rglob("*.html"):
        text = page.read_text(encoding="utf-8")
        assert f">Rullst {site_rullst_version()}</a>" in made_with_line(text), f"{page.relative_to(site)}: needs the Made with Rullst footer line"


def validate_cli_pin() -> None:
    """The CLI the deployment installs to weigh the landing is the release of the Rullst the site is made with."""
    match = CLI_PIN.fullmatch((ROOT / "site/cargo-rullst.sha256").read_text(encoding="utf-8"))
    assert match and match[2] == site_rullst_version(), (
        "site/cargo-rullst.sha256 must pin the cargo-rullst release binary of the rullst version in site/Cargo.lock"
    )


def footprint_variant(change=None, path: str = "/") -> str:
    data = copy.deepcopy(json.loads(FOOTPRINT_FIXTURE.read_text(encoding="utf-8")))
    if path != "/":
        data["target"]["url"] = SITE_URL + path[1:]
        data["page_weight"]["path"] = path
        data["page_weight"]["resources"][0]["url"] = path
    if change:
        change(data)
    return json.dumps(data)


def validate_footprint_rendering() -> None:
    """The landing's page weight from a report of the published site, and the reports it must not show."""
    reports = {"footprint.json": footprint_variant(), "footprint-pt-br.json": footprint_variant(path="/pt-br/")}
    with tempfile.TemporaryDirectory(prefix="rullst-root-site-") as temp:
        site, warnings = generate(Path(temp) / "measured", today=FOOTPRINT_DAY, footprint=reports)
        assert not warnings, f"the footprint fixture must be shown: {warnings!r}"
        for lang, page in (("en", "index.html"), ("pt-BR", "pt-br/index.html")):
            line = made_with_line((site / page).read_text(encoding="utf-8").split("<footer", 1)[1])
            assert LANGUAGES[lang]["page_weight"] in line and '<time datetime="2026-11-22">' in line, f"{page}: page weight {line!r}"
            assert "footprint 13.0.0-alpha.6</a>" in line, f"{page}: the page weight names the CLI version"
        for page in ("compare/index.html", "privacy.html"):
            assert "This page:" not in (site / page).read_text(encoding="utf-8"), f"{page}: only the landing is weighed"
        # A day later the report is still shown; two days later it is not.
        later = date.fromordinal(date.fromisoformat(FOOTPRINT_DAY).toordinal() + 2).isoformat()
        rejected = (
            ("an older report", None, later, "not today or yesterday"),
            ("another CLI version", lambda d: d.update(cli_version="13.0.0-alpha.5"), FOOTPRINT_DAY, "measured with cargo-rullst"),
            ("a failed run", lambda d: d.update(failure={"stage": "reach"}), FOOTPRINT_DAY, "failure"),
            ("a resource refused", lambda d: d["page_weight"]["resources"][1].update(status=403), FOOTPRINT_DAY, "did not load with 200"),
            ("a third party", lambda d: d["page_weight"].update(third_party_origins=["https://cdn.example"]), FOOTPRINT_DAY, "third-party"),
            ("another site", lambda d: d["target"].update(url="https://example.com/"), FOOTPRINT_DAY, "not of the published page"),
            ("an unmeasured page", lambda d: d["page_weight"].update(status="not_measured"), FOOTPRINT_DAY, "was not measured"),
            ("unfetched resources", lambda d: d["page_weight"].update(unfetched=1), FOOTPRINT_DAY, "not fetched"),
        )
        for index, (label, change, today, warning) in enumerate(rejected):
            site, warnings = generate(Path(temp) / f"rejected-{index}", today=today,
                                      footprint={"footprint.json": footprint_variant(change)})
            assert warning in warnings, f"{label}: expected a warning with {warning!r}, got {warnings!r}"
            landing = (site / "index.html").read_text(encoding="utf-8")
            assert "This page:" not in landing and "kB" not in landing, f"{label}: the landing must leave the page weight out"


DEMO_LINKS = {
    "https://rullst-showcase.redpond-24d9228d.eastus.azurecontainerapps.io/",
    "https://rullst-lms.redpond-24d9228d.eastus.azurecontainerapps.io/",
    "https://rullst-portfolio.redpond-24d9228d.eastus.azurecontainerapps.io/",
    "https://saas.rullst.win/",
    "https://github.com/Rullst/examples",
}
SOCIAL_LINKS = {
    "https://rullst.github.io", "https://github.com/Rullst",
    "https://rullst.blogspot.com", "https://www.bilibili.tv/en/space/1436672033",
    "https://daily.dev/squads/rullst", "https://dev.to/venelouis",
    "https://discord.com/invite/2ntKFtsSjw", "https://rullst.hashnode.dev/",
    "https://www.instagram.com/officialrullst/", "https://t.me/rullst",
    "https://www.tiktok.com/@venelouis", "https://www.youtube.com/@officialrullst",
    "https://x.com/venelouis", "https://bsky.app/profile/rullst.bsky.social",
    "https://www.linkedin.com/company/rullst", "https://www.reddit.com/r/rullst",
    "https://substack.com/@rullst",
}


def use_generated_site(site: Path) -> None:
    """Point the page paths at the generator's output."""
    global SITE, HOME, PRIVACY, HOME_PT_BR, PRIVACY_PT_BR, COMPARE, COMPARE_PT_BR
    SITE = site
    HOME, PRIVACY = site / "index.html", site / "privacy.html"
    HOME_PT_BR, PRIVACY_PT_BR = site / "pt-br/index.html", site / "pt-br/privacy.html"
    COMPARE, COMPARE_PT_BR = site / "compare/index.html", site / "pt-br/comparar/index.html"
    LANGUAGES["en"].update(home=HOME, privacy=PRIVACY)
    LANGUAGES["pt-BR"].update(home=HOME_PT_BR, privacy=PRIVACY_PT_BR)


def main() -> None:
    os.chdir(ROOT)
    assert GENERATOR.is_file(), (
        f"{GENERATOR} is missing: build it with `cargo build --locked --manifest-path site/Cargo.toml` "
        "or set RULLST_SITE_BIN"
    )
    with tempfile.TemporaryDirectory(prefix="rullst-root-site-") as temp:
        site, warnings = generate(Path(temp) / "plain")
        assert not warnings, f"the generator warned without data: {warnings!r}"
        published = sorted(str(path.relative_to(site)) for path in site.rglob("*") if path.is_file())
        assert published == sorted(PUBLISHED), f"unexpected root site files: {published}"
        for name, origin in ASSET_SOURCES.items():
            assert (site / name).read_bytes() == origin.read_bytes(), f"generated {name} differs from {origin.relative_to(ROOT)}"
        use_generated_site(site)
        validate(site)
        validate_footer_lines(site)
    validate_cli_pin()
    validate_footprint_rendering()


def validate(site: Path) -> None:
    readme = README.read_text(encoding="utf-8")
    home, source = validate_landing("en")
    home_pt, source_pt = validate_landing("pt-BR")
    validate_translation(home, source, home_pt, source_pt, HOME_PT_BR.name)
    landing_checked = [landing_checked_lines(text) for text in (source, source_pt)]
    assert landing_checked[0] == landing_checked[1] and landing_checked[0], (
        f"{HOME_PT_BR.name}: the checked lines differ from the English page: {landing_checked}"
    )
    compare_checked = re.search(r'<p class="compare-checked" id="checked">(.*?)</p>', COMPARE.read_text(encoding="utf-8"), re.DOTALL)
    assert compare_checked, f"{COMPARE.name}: missing its checked line"
    compare_versions = sorted(re.findall(r"<code>((?:loco-rs|topcoat) [^<]+)</code>", compare_checked[1]))
    for ident, values in landing_checked[0]:
        versions = sorted(code for code, _date in values if code.startswith(("loco-rs ", "topcoat ")))
        assert versions == compare_versions, f"landing #{ident} checked {versions}, the comparison page {compare_versions}"
    interface_text = set(re.findall(r"ui\.ui([A-Z]\w+)", (DOCS / "site.js").read_text(encoding="utf-8")))
    translated_text = {
        "".join(part.title() for part in name.split("-"))
        for name in re.findall(r'data-ui-([a-z-]+)="[^"]+"', source_pt[source_pt.index("<body"):source_pt.index(">", source_pt.index("<body"))])
    }
    assert interface_text and translated_text == interface_text, (
        f"{HOME_PT_BR.name}: <body> must translate every interface string of site.js: "
        f"missing {sorted(interface_text - translated_text)}, unknown {sorted(translated_text - interface_text)}"
    )
    privacy, privacy_source = validate_privacy("en", home)
    privacy_pt, privacy_source_pt = validate_privacy("pt-BR", home_pt)
    validate_translation(privacy, privacy_source, privacy_pt, privacy_source_pt, PRIVACY_PT_BR.name)
    validate_comparison(home, home_pt)

    slogan = LANGUAGES["en"]["slogan"]
    assert all(link in readme for link in DEMO_LINKS), "README is missing a published example link"
    assert readme.count("about 20–30 seconds") >= 1, "README must give the Azure demos' wake-up time"
    assert "🌐🦀📜 Rullst 📜🦀🌐" in readme, "README must preserve the project title identity"
    assert slogan in readme, "README must preserve the project slogan"
    integration_count = validate_integrations(source, readme)
    workflow_count = validate_workflow_references(readme)
    assert f"({workflow_count} workflows)" in readme, "README workflow dashboard count is stale"
    assert (
        f"{workflow_count} workflow definitions" in readme
    ), "README workflow-definition count is stale"

    # The landing summary and the full notice carry the statements of the two
    # notices they replaced.
    for statement in (
        "We do not control GitHub's hosting logs or their retention.",
        "benchmark pages currently load Chart.js from jsDelivr.",
        "Public issues and social posts are not private support channels.",
    ):
        assert statement in source and statement in privacy_source, f"privacy statement missing: {statement}"
    assert "Venelouis" in source and "Venelouis" in source_pt, "both landing pages must name the responsible person"

    redirect, redirect_source = parse(REDIRECT)
    validate_document(REDIRECT, redirect)
    assert f'<meta http-equiv="refresh" content="0; url={SITE_URL}">' in redirect_source, "/Rullst/ must redirect to the root"
    assert f'<link rel="canonical" href="{SITE_URL}">' in redirect_source, "/Rullst/ must name the root as canonical"
    assert SITE_URL in redirect.links, "/Rullst/ needs a visible link to the root"
    assert not redirect.scripts and not redirect.stylesheets and not redirect.inline_behavior, "/Rullst/ stays static"

    for template in BENCH_TEMPLATES:
        document, benchmark_source = parse(template)
        validate_document(template, document)
        assert not document.remote_scripts_without_integrity, f"{template}: unpinned external script"
        assert "/privacy.html" in document.links, f"{template}: missing privacy link"
        assert "/Rullst/#privacy" not in document.links, f"{template}: privacy link points at the /Rullst/ redirect"
        assert "fonts.googleapis.com" not in benchmark_source, f"{template}: undocumented remote fonts"

    workflow = (ROOT / ".github" / "workflows" / "pages.yml").read_text(encoding="utf-8")
    assert "generate-llms-txt.py --out public_site" in workflow, "pages workflow does not publish llms.txt"
    for required_copy in ("docs/site.css", "docs/site.js", "images"):
        assert required_copy in workflow, f"pages workflow does not publish {required_copy}"
    assert "cp docs/redirect_template.html public_site/index.html" in workflow, "/Rullst/ must serve the redirect page"
    assert "home_template" not in workflow, "the landing page is published only at the site root"

    assert (DOCS / "site.css").read_text(encoding="utf-8").count("{") == (
        DOCS / "site.css"
    ).read_text(encoding="utf-8").count("}"), "unbalanced site CSS blocks"
    llms_pages = validate_llms()
    print(
        f"validated English and Brazilian Portuguese landing, privacy and comparison pages ({integration_count} "
        f"integration entries matching README; same ids, external links, integration names and checked versions "
        f"in both languages; benchmark rows and page weight rendered from fixtures; Made with Rullst card and footer line), "
        f"/Rullst/ redirect, generated root site, {len(BENCH_TEMPLATES)} benchmark templates "
        f"and llms.txt/llms-full.txt for {llms_pages} book pages"
    )


if __name__ == "__main__":
    main()
