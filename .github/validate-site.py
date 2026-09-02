#!/usr/bin/env python3
"""Validate the static landing page and benchmark templates without network I/O."""

from __future__ import annotations

from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlparse


ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"
HOME = DOCS / "home_template.html"
BENCH_TEMPLATES = sorted(DOCS.glob("benches*_template.html"))


class Document(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: list[str] = []
        self.links: list[str] = []
        self.scripts: list[str] = []
        self.stylesheets: list[str] = []
        self.images_without_alt: list[str] = []
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
        if tag == "link" and values.get("rel") == "stylesheet" and (href := values.get("href")):
            self.stylesheets.append(href)
        if tag == "img" and "alt" not in values:
            self.images_without_alt.append(values.get("src", "<unknown>"))
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
    if not path or path == "/Rullst/" or path.startswith("/Rullst/benches/"):
        return None
    if path == "/Rullst/book/":
        return DOCS / "book" / "index.html"
    if path.startswith("/Rullst/book/"):
        return DOCS / "book" / path.removeprefix("/Rullst/book/")
    if path == "/Rullst/Rullst.png":
        return DOCS / "Rullst.png"
    if path.startswith("/Rullst/images/"):
        return ROOT / path.removeprefix("/Rullst/")
    if path == "./assets/site.css":
        return DOCS / "site.css"
    if path == "./assets/site.js":
        return DOCS / "site.js"
    raise AssertionError(f"unclassified local home reference: {reference}")


def validate_document(path: Path, document: Document) -> None:
    assert document.has_language, f"{path}: missing html language"
    assert document.has_title, f"{path}: missing title"
    assert document.h1_count == 1, f"{path}: expected exactly one h1"
    assert len(document.ids) == len(set(document.ids)), f"{path}: duplicate id"
    assert not document.images_without_alt, f"{path}: images missing alt: {document.images_without_alt}"


def main() -> None:
    home, source = parse(HOME)
    validate_document(HOME, home)
    assert home.has_main and home.has_nav and home.has_footer, "landing page needs nav, main and footer"
    assert source.count("All glory and honor to God") == 2, "landing page needs the top and bottom dedication"
    assert "NO-GO for production" in source, "landing page must preserve the release warning"
    assert "Content-Security-Policy" in source, "landing page must declare a CSP"
    assert not home.inline_behavior, f"landing page has inline behavior/style: {home.inline_behavior}"
    assert home.stylesheets == ["./assets/site.css"], "landing stylesheet must be repository-local"
    assert home.scripts == ["./assets/site.js"], "landing script must be repository-local"

    for reference in [*home.links, *home.stylesheets, *home.scripts, "/Rullst/Rullst.png", "/Rullst/images/cargo-rullst-dash.png"]:
        if reference.startswith("#"):
            assert reference[1:] in home.ids, f"missing landing anchor: {reference}"
            continue
        target = local_home_target(reference)
        if target is not None:
            assert target.is_file(), f"missing local landing target: {reference} -> {target}"

    for template in BENCH_TEMPLATES:
        document, _source = parse(template)
        validate_document(template, document)

    workflow = (ROOT / ".github" / "workflows" / "pages.yml").read_text(encoding="utf-8")
    for required_copy in ("docs/site.css", "docs/site.js", "images"):
        assert required_copy in workflow, f"pages workflow does not publish {required_copy}"

    assert (DOCS / "site.css").read_text(encoding="utf-8").count("{") == (
        DOCS / "site.css"
    ).read_text(encoding="utf-8").count("}"), "unbalanced site CSS blocks"
    print(f"validated landing page and {len(BENCH_TEMPLATES)} benchmark templates")


if __name__ == "__main__":
    main()
