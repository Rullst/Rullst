#!/usr/bin/env python3
"""Assemble the https://rullst.win/ root site from this repository's sources.

    python3 .github/assemble-organization-site.py --out DIRECTORY

Writes the English landing and privacy pages at the root, their Brazilian
Portuguese versions under pt-br/, and the shared assets (stylesheet, script
and language-switcher flags) under assets/ into DIRECTORY, which must be
missing or empty. The pages also reference /Rullst/book/, /Rullst/images/ and
/Rullst/Rullst.png, which this repository's pages.yml deploys, so every path
is valid at the site root.

The Rullst/Rullst.github.io deployment workflow runs this script from a
read-only checkout of this repository's main branch; it reads no secret and
never commits, pushes or deploys anything itself. validate-site.py runs it too.
"""

from __future__ import annotations

import argparse
from hashlib import sha256
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
}
LANDINGS = ("index.html", "pt-br/index.html")
ASSET_REFERENCE = re.compile(r'(?:href|src)="/assets/(site\.(?:css|js))\?v=([0-9a-f]{16})"')
LOCAL_ASSET = re.compile(r'(?:href|src)="/(assets/[^"?#]+)')


class AssemblyError(Exception):
    """The sources cannot produce a consistent root site."""


def check_sources(root: Path = ROOT) -> dict[str, bytes]:
    """Read every source and check the references the deployment relies on."""
    contents = {}
    for published, source in FILES.items():
        path = root / source
        if not path.is_file() or path.is_symlink():
            raise AssemblyError(f"missing regular source file: {source}")
        contents[published] = path.read_bytes()
    digests = {
        name: sha256(contents[f"assets/{name}"]).hexdigest()[:16]
        for name in ("site.css", "site.js")
    }
    for page, canonical in CANONICAL.items():
        text = contents[page].decode("utf-8")
        if f'<link rel="canonical" href="{canonical}">' not in text:
            raise AssemblyError(f"{FILES[page]} must declare the canonical URL {canonical}")
        if "Content-Security-Policy" not in text:
            raise AssemblyError(f"{FILES[page]} must declare a Content-Security-Policy")
        for asset in LOCAL_ASSET.findall(text):
            if asset not in FILES:
                raise AssemblyError(f"{FILES[page]} references /{asset}, which the root site does not publish")
        references = ASSET_REFERENCE.findall(text)
        if not references:
            raise AssemblyError(f"{FILES[page]} references no content-versioned asset")
        for name, version in references:
            if version != digests[name]:
                raise AssemblyError(
                    f"{FILES[page]} references {name}?v={version}; the current content version is {digests[name]}"
                )
    for page in LANDINGS:
        landing = contents[page].decode("utf-8")
        for name in ("site.css", "site.js"):
            if f"/assets/{name}?v={digests[name]}" not in landing:
                raise AssemblyError(f"{FILES[page]} must load /assets/{name}?v={digests[name]}")
    return contents


def assemble(out: Path, root: Path = ROOT) -> list[str]:
    """Write the root site into OUT and return the published paths."""
    contents = check_sources(root)
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
    args = parser.parse_args(argv)
    try:
        published = assemble(args.out)
    except AssemblyError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"assembled {len(published)} files into {args.out}: {', '.join(published)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
