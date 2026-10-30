#!/usr/bin/env python3
"""Validate the static landing page and benchmark templates without network I/O."""

from __future__ import annotations

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


def main() -> None:
    home, source = parse(HOME)
    validate_document(HOME, home)
    assert home.has_main and home.has_nav and home.has_footer, "landing page needs nav, main and footer"
    assert source.count("All glory and honor to God") == 1, "landing page needs the footer dedication"
    assert "Rullst v13 prerelease" in source, "landing page must identify the v13 prerelease line"
    assert "Rullst v12 stable" not in source, "landing page still presents end-of-life v12 as current"
    assert "evaluate before stable v12" not in source, "landing page retains obsolete RC copy"
    assert "/Rullst/book/v12.html" not in home.links, "landing release-status links must not point to the v12 record"
    cli_version = CLI_VERSION.search((ROOT / "cargo-rullst" / "Cargo.toml").read_text(encoding="utf-8"))
    assert cli_version, "cannot read the cargo-rullst version"
    pins = INSTALL_PIN.findall(source)
    assert pins and set(pins) == {cli_version[1]}, (
        f"landing install command must pin the current CLI version ={cli_version[1]}: {pins}"
    )
    assert "Content-Security-Policy" in source, "landing page must declare a CSP"
    assert not home.inline_behavior, f"landing page has inline behavior/style: {home.inline_behavior}"
    for references, filename in ((home.stylesheets, "site.css"), (home.scripts, "site.js")):
        digest = sha256((DOCS / filename).read_bytes()).hexdigest()[:16]
        expected = f"./assets/{filename}?v={digest}"
        assert references == [expected], f"Use the current content-versioned local asset: {expected}"
    assert not home.embeds, "landing privacy contract forbids third-party embeds"
    assert "privacy" in home.ids, "landing page must expose its privacy notice"
    assert 'name="referrer" content="no-referrer"' in source, "landing must suppress outgoing referrers"
    assert not any(urlparse(src).netloc for src in home.images), "landing images must be same-origin"
    required_social_links = {
        "https://rullst.github.io", "https://github.com/Rullst",
        "https://rullst.blogspot.com", "https://www.bilibili.tv/en/space/1436672033",
        "https://daily.dev/squads/rullst", "https://dev.to/venelouis",
        "https://discord.com/invite/2ntKFtsSjw", "https://rullst.hashnode.dev/",
        "https://www.instagram.com/rullst_official/", "https://t.me/rullst",
        "https://www.tiktok.com/@venelouis", "https://www.youtube.com/@rullst_official",
        "https://x.com/venelouis",
    }
    assert required_social_links <= set(home.links), "missing requested community links"
    demo_links = {
        "https://rullst-showcase.redpond-24d9228d.eastus.azurecontainerapps.io/",
        "https://rullst-lms.redpond-24d9228d.eastus.azurecontainerapps.io/",
        "https://rullst-portfolio.redpond-24d9228d.eastus.azurecontainerapps.io/",
        "https://github.com/Rullst/examples",
    }
    assert "examples" in home.ids, "landing must expose the demo directory"
    assert demo_links <= set(home.links), "landing is missing a published example link"

    slogan = "Intelligent, Security-Conscious, and Designed for Effortless Productivity"
    assert slogan in source, "landing page must preserve the project slogan"
    readme = README.read_text(encoding="utf-8")
    assert all(link in readme for link in demo_links), "README is missing a published example link"
    assert "🌐🦀📜 Rullst 📜🦀🌐" in readme, "README must preserve the project title identity"
    assert slogan in readme, "README must preserve the project slogan"
    workflow_count = validate_workflow_references(readme)
    assert f"({workflow_count} workflows)" in readme, "README workflow dashboard count is stale"
    assert (
        f"{workflow_count} workflow definitions" in readme
    ), "README workflow-definition count is stale"

    for reference in [*home.links, *home.stylesheets, *home.scripts, *home.images]:
        if reference.startswith("#"):
            assert reference[1:] in home.ids, f"missing landing anchor: {reference}"
            continue
        target = local_home_target(reference)
        if target is not None:
            assert target.is_file(), f"missing local landing target: {reference} -> {target}"

    for template in BENCH_TEMPLATES:
        document, benchmark_source = parse(template)
        validate_document(template, document)
        assert not document.remote_scripts_without_integrity, f"{template}: unpinned external script"
        assert "/Rullst/#privacy" in document.links, f"{template}: missing privacy link"
        assert "fonts.googleapis.com" not in benchmark_source, f"{template}: undocumented remote fonts"

    workflow = (ROOT / ".github" / "workflows" / "pages.yml").read_text(encoding="utf-8")
    assert "generate-llms-txt.py --out public_site" in workflow, "pages workflow does not publish llms.txt"
    for required_copy in ("docs/site.css", "docs/site.js", "images"):
        assert required_copy in workflow, f"pages workflow does not publish {required_copy}"

    assert (DOCS / "site.css").read_text(encoding="utf-8").count("{") == (
        DOCS / "site.css"
    ).read_text(encoding="utf-8").count("}"), "unbalanced site CSS blocks"
    llms_pages = validate_llms()
    print(
        f"validated landing page, {len(BENCH_TEMPLATES)} benchmark templates "
        f"and llms.txt/llms-full.txt for {llms_pages} book pages"
    )


if __name__ == "__main__":
    main()
