#!/usr/bin/env python3
"""Validate the static landing pages and benchmark templates without network I/O.

The landing page and the full privacy notice, in English
(docs/home_template.html, docs/privacy_template.html) and Brazilian
Portuguese (docs/*.pt-BR.html), are published at https://rullst.win/ and
https://rullst.win/pt-br/ by Rullst/Rullst.github.io through
assemble-organization-site.py; /Rullst/ only serves
docs/redirect_template.html, pointing there. The two languages must keep the
same section ids, external links and integration names.
"""

from __future__ import annotations

from contextlib import redirect_stderr
from html import unescape
import io
import os
from html.parser import HTMLParser
from hashlib import sha256
import importlib.util
import re
import tempfile
from pathlib import Path
from urllib.parse import urlparse


ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"
HOME = DOCS / "home_template.html"
PRIVACY = DOCS / "privacy_template.html"
HOME_PT_BR = DOCS / "home_template.pt-BR.html"
PRIVACY_PT_BR = DOCS / "privacy_template.pt-BR.html"
COMPARE = DOCS / "compare_template.html"
COMPARE_PT_BR = DOCS / "compare_template.pt-BR.html"
BENCHMARK_FIXTURE = ROOT / ".github" / "fixtures" / "site-benchmarks.json"
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
           "performance": "measured in June 2026, before v13"},
    "pt-BR": {"home": HOME_PT_BR, "privacy": PRIVACY_PT_BR, "url": SITE_URL + "pt-br/", "locale": "pt_BR",
              "dedication": "Toda glória e honra a Deus", "status": "Pré-lançamento do Rullst v13",
              "slogan": "Inteligente, consciente da segurança e feito para uma produtividade sem esforço",
              "performance": "medida em junho de 2026, antes da v13"},
}


def validate_language_links(path: Path, source: str, page: str, lang: str, pt_page: str | None = None) -> None:
    """Language attribute, hreflang alternates and the flag switcher.

    PAGE is the English path below the site root; PT_PAGE the Portuguese one
    (default: the same path under pt-br/).
    """
    english, portuguese = f"/{page}", f"/pt-br/{page}" if pt_page is None else f"/{pt_page}"
    assert f'<html lang="{lang}">' in source, f"{path.name}: <html lang> must be {lang}"
    for hreflang, href in (("en", english), ("pt-BR", portuguese), ("x-default", english)):
        alternate = f'<link rel="alternate" hreflang="{hreflang}" href="{SITE_URL}{href[1:]}">'
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
    assert f'<link rel="canonical" href="{facts["url"]}">' in source, f"{path.name}: canonical must be {facts['url']}"
    assert f'<meta property="og:url" content="{facts["url"]}">' in source, f"{path.name}: og:url must be {facts['url']}"
    assert f'<meta property="og:locale" content="{facts["locale"]}">' in source, f"{path.name}: og:locale"
    assert "academy" in home.ids and any(link.startswith("https://academy.rullst.win/") for link in home.links), (
        f"{path.name}: must keep the Academy section"
    )
    assert {"integrations", "performance"} <= set(home.ids), f"{path.name}: needs Integrations and Performance"
    assert {"/Rullst/benches/", "/Benchmarks/"} <= set(home.links), f"{path.name}: Performance must link both sources"
    assert facts["performance"] in source, f"{path.name}: Performance must date the cross-framework comparison"
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
    assert f'<link rel="canonical" href="{url}">' in source, f"{path.name}: canonical URL must be {url}"
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


def validate_comparison(home: Document, home_pt: Document, assembler) -> None:
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
        assert text.count(assembler.BENCHMARK_START) == 1 and 'class="benchmark-pending"' in text, (
            f"{name}: needs the benchmark-results block with its pending text"
        )
    # "How to read these numbers": the same four points in both languages, naming
    # the minimal row as the assembler labels it, before the results block.
    for text, name, lang in ((source, COMPARE.name, "en"), (source_pt, COMPARE_PT_BR.name, "pt-BR")):
        howto = re.search(r'<div class="benchmark-howto">(.*?)</div>', text, re.DOTALL)
        assert howto and len(re.findall(r"<li>", howto[1])) == 4, f"{name}: needs the four-point benchmark-howto block"
        assert text.index('class="benchmark-howto"') < text.index(assembler.BENCHMARK_START), f"{name}: benchmark-howto goes before the results"
        minimal = assembler.TEXT[lang]["names"][assembler.MINIMAL]
        assert f'"{minimal}"' in howto[1] and "cargo rullst new" in howto[1], f"{name}: benchmark-howto must explain {minimal}"
    assert "/compare/" in home.links and "/pt-br/comparar/" in home_pt.links, "landing pages must link the comparison"
    with tempfile.TemporaryDirectory(prefix="rullst-root-site-") as temp:
        assembler.assemble(Path(temp) / "data", benchmarks=BENCHMARK_FIXTURE)
        run_link = '<a href="https://github.com/Rullst/Benchmarks/actions/runs/1">'
        for page, lang, expected in ((COMPARE_PAGE, "en", ("0.48× Axum", "(90,486)", "3.50 ms", "version 13.0.0-alpha.6", "Not in this measurement run.",
                                                        "Fixture note: these numbers are not measurements of <code>cargo rullst new</code> starters; "
                                                        "`&lt;b&gt;` and a lone ` stay text.</p>",
                                                        "1.00× Axum", "(114,422)", run_link)),
                               ("pt-br/comparar/index.html", "pt-BR", ("0,48× Axum", "(90.486)", "3,50 ms", "versão 13.0.0-alpha.6", "Fora desta rodada de medição.",
                                                              "Nota do fixture: estes números não são medições de projetos <code>cargo rullst new</code>; "
                                                              "`&lt;b&gt;` e uma ` sozinha ficam como texto.</p>",
                                                              "1,00× Axum", "(114.422)", run_link))):
            rendered = (Path(temp) / "data" / page).read_text(encoding="utf-8")
            assert 'class="benchmark-pending"' not in rendered, f"{page}: benchmark rows were not rendered"
            assert all(text in rendered for text in expected), f"{page}: unexpected benchmark rendering"
            # The fixture's Actix-Web row stays out of the results (the
            # "Why not Leptos or Dioxus?" text names Actix Web on purpose).
            results = rendered[rendered.index(assembler.BENCHMARK_START):rendered.index(assembler.BENCHMARK_END)]
            assert "Actix" not in results, f"{page}: renders a framework outside the comparison"
            # Rullst, then the minimal row labelled as the framework without the
            # generated app's layers, then Loco and Topcoat.
            rows = re.search(r'<ul class="benchmark-rows">(.*?)</ul>', rendered, re.DOTALL)
            titles = re.findall(r'<li><h3>([^<]*?) ?(?:<span>|</h3>)', rows[1] if rows else "")
            assert titles == ["Rullst", assembler.TEXT[lang]["names"][assembler.MINIMAL], "Loco", "Topcoat"], f"{page}: benchmark rows {titles}"
            variants = re.findall(r'<p class="benchmark-variant">(.*?)</p>', rendered)
            assert variants == [assembler.TEXT[lang]["variants"][row] for row in ("Rullst", assembler.MINIMAL)], f"{page}: variant labels {variants}"
    # Backtick spans of a note: only short, balanced ones without markup become <code>.
    for note, html in (("`a` & `b`", "<code>a</code> &amp; <code>b</code>"), ("`a", "`a"), ("``", "``"), ("` `", "` `"),
                       ("`" + "x" * 81 + "`", "`" + "x" * 81 + "`"), ("`x\ny`", "`x\ny`"), ('`"q"`', "<code>&quot;q&quot;</code>"),
                       ("`<i>` `ok` `", "`&lt;i&gt;` <code>ok</code> `")):
        assert assembler.render_note(note) == html, f"note {note!r} rendered as {assembler.render_note(note)!r}"
    # The assembler reads results only from inside the working directory.
    with tempfile.TemporaryDirectory(prefix=".validate-site-", dir=ROOT) as temp, redirect_stderr(io.StringIO()) as warnings:
        bad = Path(temp) / "bad.json"
        bad.write_text('{"schema": "rullst.benchmarks.latest.v1", "measured_on": "2026-11-20", "results": [{"framework": "Rullst", "version": "<b>", "json_rps": 1}]}')
        assembler.assemble(Path(temp) / "bad", benchmarks=bad)
        assert "unexpected version" in warnings.getvalue(), "invalid benchmark data must be reported"
        bad.write_text('{"schema": "rullst.benchmarks.latest.v1", "measured_on": "2026-11-20", "run": {"url": "javascript:alert(1)"}, "results": [{"framework": "Rullst", "version": "1.0.0", "json_rps": 1}]}')
        assembler.assemble(Path(temp) / "bad-run", benchmarks=bad)
        assert "unexpected run url" in warnings.getvalue(), "a run link outside Rullst/Benchmarks must be rejected"
        # The minimal row is extra: data without it shows the other rows and no placeholder for it.
        bad.write_text('{"schema": "rullst.benchmarks.latest.v1", "measured_on": "2026-11-20", "results": [{"framework": "Rullst", "version": "1.0.0", "json_rps": 1}]}')
        assembler.assemble(Path(temp) / "no-minimal", benchmarks=bad)
        rendered = (Path(temp) / "no-minimal" / COMPARE_PAGE).read_text(encoding="utf-8")
        assert "<li><h3>Rullst <span>" in rendered and "<li><h3>Rullst (minimal)" not in rendered, "a missing minimal row must be left out"
        assert 'class="benchmark-pending"' in (Path(temp) / "bad" / COMPARE_PAGE).read_text(encoding="utf-8"), (
            "invalid benchmark data must keep the pending text"
        )


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


def main() -> None:
    os.chdir(ROOT)
    readme = README.read_text(encoding="utf-8")
    home, source = validate_landing("en")
    home_pt, source_pt = validate_landing("pt-BR")
    validate_translation(home, source, home_pt, source_pt, HOME_PT_BR.name)
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
    spec = importlib.util.spec_from_file_location("assemble_site", ROOT / ".github/assemble-organization-site.py")
    assert spec is not None and spec.loader is not None, "cannot load assemble-organization-site.py"
    assembler = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(assembler)
    validate_comparison(home, home_pt, assembler)

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

    with tempfile.TemporaryDirectory(prefix="rullst-root-site-") as temp:
        published = assembler.assemble(Path(temp) / "site")
        assert published == list(assembler.FILES), f"unexpected root site files: {published}"
        for name, origin in assembler.FILES.items():
            assert (Path(temp) / "site" / name).read_bytes() == (ROOT / origin).read_bytes(), f"assembled {name} differs"

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
    assert "cp docs/home_template.html" not in workflow, "the landing page is published only at the site root"

    assert (DOCS / "site.css").read_text(encoding="utf-8").count("{") == (
        DOCS / "site.css"
    ).read_text(encoding="utf-8").count("}"), "unbalanced site CSS blocks"
    llms_pages = validate_llms()
    print(
        f"validated English and Brazilian Portuguese landing, privacy and comparison pages ({integration_count} "
        f"integration entries matching README; same ids, external links, integration names and checked versions "
        f"in both languages; benchmark rows rendered from a fixture), "
        f"/Rullst/ redirect, assembled root site, {len(BENCH_TEMPLATES)} benchmark templates "
        f"and llms.txt/llms-full.txt for {llms_pages} book pages"
    )


if __name__ == "__main__":
    main()
