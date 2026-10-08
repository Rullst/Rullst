#!/usr/bin/env python3
"""Generate llms.txt and llms-full.txt from the mdBook sources.

The pages are read in docs/src/SUMMARY.md order. `{{#include}}` directives are
expanded the way mdBook expands them (whole file, line ranges and anchors),
HTML comments are removed and relative links become absolute book or
repository URLs, so both files read correctly outside the book. The output is
deterministic: it depends only on the sources, never on time or environment.

Usage: generate-llms-txt.py [--docs docs] [--out DIR]
"""
import argparse
from pathlib import Path, PurePosixPath
import re
import sys
from urllib.parse import unquote, urlsplit

SITE_URL = "https://rullst.github.io/Rullst/"
BOOK_URL = SITE_URL + "book/"
REPOSITORY_URL = "https://github.com/Rullst/Rullst/blob/main/"
TITLE = "Rullst"
SUMMARY = (
    "Rullst is a modular, strictly typed Rust framework suite (Tokio, Axum, "
    "Tower, SQLx) for full-stack web applications: Active Record ORM, "
    "server-rendered html! views with HTMX, authentication, security "
    "middleware and the cargo-rullst CLI. This index lists every page of the "
    "Rullst book in reading order; llms-full.txt holds their full Markdown."
)
DESCRIPTION_LIMIT = 200

CHAPTER = re.compile(r"^(?P<indent>\s*)(?:[-*]\s+)?\[(?P<title>[^\]]+)\]\((?P<path>[^)]*)\)\s*$")
HEADING = re.compile(r"^(?P<level>#{1,6})\s+(?P<text>.+?)\s*#*\s*$")
INCLUDE = re.compile(r"(?<!\\)\{\{#include\s+(?P<arg>[^}]+?)\s*\}\}")
COMMENT = re.compile(r"<!--.*?-->", re.S)
FENCE = re.compile(r"^\s*(```+|~~~+)")
INLINE_LINK = re.compile(r"(?P<prefix>\]\(<?)(?P<url>[^\s<>)]+)(?=[\s>)])")
LINK_TEXT = re.compile(r"!?\[([^\]]*)\]\([^)]*\)")
NOT_PROSE = re.compile(r"^(?:[#>|<!]|\{\{|[-*+]\s|\d+[.)]\s|---|\[!)")
SENTENCE_END = re.compile(r"(?<=[.!?])\s+(?=[A-Z0-9`*\[(])")


def clean_title(text):
    """Drop decorative leading emoji and Markdown emphasis from a title."""
    text = re.sub(r"^[^\w`(\[]+", "", text.strip())
    return text.replace("**", "").strip()


def parse_summary(source):
    """Return [(section, [(title, path)])] in SUMMARY order."""
    sections = []
    part = "Introduction"
    current = None
    in_fence = False
    for line in (source / "SUMMARY.md").read_text(encoding="utf-8").splitlines():
        if FENCE.match(line):
            in_fence = not in_fence
        if in_fence or not line.strip():
            continue
        heading = HEADING.match(line)
        if heading:
            text = clean_title(heading["text"])
            if text == "Summary":
                continue
            if heading["level"] == "#":
                part = text
                name = text
            else:
                name = f"{part}: {text}"
            current = (name, [])
            sections.append(current)
            continue
        chapter = CHAPTER.match(line)
        if not chapter or not chapter["path"].strip():
            continue  # Separators and draft chapters have no page.
        if current is None:
            current = (part, [])
            sections.append(current)
        current[1].append((clean_title(chapter["title"]), chapter["path"].strip()))
    return [(name, pages) for name, pages in sections if pages]


def page_url(path):
    return BOOK_URL + PurePosixPath(path).with_suffix(".html").as_posix()


def select_lines(text, selector):
    """Apply an mdBook include selector: a line range or an anchor name."""
    if not selector:
        return text
    lines = text.splitlines()
    parts = selector.split(":")
    if all(part == "" or part.isdigit() for part in parts) and len(parts) <= 2:
        start = int(parts[0]) if parts[0] else 1
        if len(parts) == 1:
            end = start
        else:
            end = int(parts[1]) if parts[1] else len(lines)
        chosen = lines[max(start, 1) - 1:end]
    else:
        begin = re.compile(r"ANCHOR:\s*" + re.escape(selector) + r"\b")
        finish = re.compile(r"ANCHOR_END:\s*" + re.escape(selector) + r"\b")
        chosen, inside = [], False
        for line in lines:
            if not inside and begin.search(line):
                inside = True
            elif inside and finish.search(line):
                break
            elif inside:
                chosen.append(line)
        if not inside:
            raise ValueError(f"include anchor {selector!r} not found")
    chosen = [line for line in chosen if not re.search(r"ANCHOR(_END)?:", line)]
    return "\n".join(chosen)


def expand_includes(text, directory, repository, rewrite, depth=0):
    """Expand includes; rewrite(text, directory) fixes links in included Markdown."""
    if depth > 10:
        raise ValueError("include nesting exceeds mdBook's depth limit")

    def replace(match):
        argument = match["arg"].strip()
        path, _, selector = argument.partition(":")
        target = (directory / path).resolve()
        if not target.is_relative_to(repository) or not target.is_file():
            raise ValueError(f"include target is missing or outside the repository: {argument}")
        included = select_lines(target.read_text(encoding="utf-8"), selector)
        included = expand_includes(included, target.parent, repository, rewrite, depth + 1)
        return rewrite(included, target.parent) if target.suffix == ".md" else included

    return INCLUDE.sub(replace, text)


def outside_fences(text, transform):
    """Apply transform to the text outside fenced code blocks."""
    output, block, fence = [], [], None
    for line in text.splitlines(keepends=True):
        marker = FENCE.match(line)
        if fence is None and marker:
            output.append(transform("".join(block)))
            block, fence = [line], marker[1][0] * 3
        elif fence is not None:
            block.append(line)
            if marker and marker[1].startswith(fence):
                output.append("".join(block))
                block, fence = [], None
        else:
            block.append(line)
    output.append("".join(block) if fence is not None else transform("".join(block)))
    return "".join(output)


def absolute_links(text, origin, page, source, repository, pages):
    """Rewrite links relative to origin to the published book or repository URLs."""

    def rewrite(match):
        value = urlsplit(match["url"])
        if value.scheme or value.netloc or match["url"].startswith(("/", "mailto:")):
            return match[0]
        fragment = "#" + value.fragment if value.fragment else ""
        if not value.path:
            return match["prefix"] + page_url(page) + fragment
        target = (origin / unquote(value.path)).resolve()
        if not target.is_relative_to(repository) or not target.exists():
            return match[0]
        if target.is_relative_to(source):
            relative = target.relative_to(source).as_posix()
            if relative in pages:
                return match["prefix"] + page_url(relative) + fragment
        relative = target.relative_to(repository).as_posix()
        return match["prefix"] + REPOSITORY_URL + relative + fragment

    return outside_fences(text, lambda block: INLINE_LINK.sub(rewrite, block))


def render_page(page, source, repository, pages):
    path = source / page
    text = path.read_text(encoding="utf-8")

    def rewrite(markdown, origin):
        return absolute_links(markdown, origin, page, source, repository, pages)

    text = rewrite(expand_includes(text, path.parent, repository, rewrite), path.parent)
    text = COMMENT.sub("", text)
    text = re.sub(r"\n{3,}", "\n\n", text).strip()
    return text.replace("\\{{#", "{{#")


def describe(text):
    """Return the first prose sentence of a page, bounded in length."""
    paragraph, in_fence = [], False
    for line in text.splitlines():
        stripped = line.strip()
        if FENCE.match(line):
            in_fence = not in_fence
            if paragraph:
                break
            continue
        if in_fence:
            continue
        if not stripped:
            if paragraph:
                break
            continue
        if NOT_PROSE.match(stripped):
            if paragraph:
                break
            continue
        paragraph.append(stripped)
    sentence = LINK_TEXT.sub(r"\1", " ".join(paragraph)).replace("**", "")
    sentence = SENTENCE_END.split(sentence, maxsplit=1)[0].strip()
    if len(sentence) > DESCRIPTION_LIMIT:
        sentence = sentence[:DESCRIPTION_LIMIT].rsplit(" ", 1)[0].rstrip(",;:") + "…"
    return sentence


def generate(docs):
    docs = docs.resolve()
    repository = docs.parent
    source = docs / "src"
    sections = parse_summary(source)
    pages = {path for _, entries in sections for _, path in entries}
    index = [f"# {TITLE}", "", f"> {SUMMARY}", "",
             f"Full text of every page: {SITE_URL}llms-full.txt", ""]
    full = [f"# {TITLE} book: full text", "", f"> {SUMMARY}", ""]
    seen = set()
    for name, entries in sections:
        index += [f"## {name}", ""]
        for title, page in entries:
            text = render_page(page, source, repository, pages)
            description = describe(text)
            entry = f"- [{title}]({page_url(page)})"
            index.append(f"{entry}: {description}" if description else entry)
            if page in seen:
                continue
            seen.add(page)
            full += ["---", "", f"# {title}", "", f"Source: {page_url(page)}", "", text, ""]
        index.append("")
    return "\n".join(index).rstrip() + "\n", "\n".join(full).rstrip() + "\n"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--docs", type=Path, default=Path(__file__).resolve().parents[1] / "docs")
    parser.add_argument("--out", type=Path, default=Path("."))
    arguments = parser.parse_args(argv)
    try:
        index, full = generate(arguments.docs)
    except (OSError, ValueError) as error:
        print(f"generate-llms-txt: {error}", file=sys.stderr)
        return 1
    arguments.out.mkdir(parents=True, exist_ok=True)
    (arguments.out / "llms.txt").write_text(index, encoding="utf-8")
    (arguments.out / "llms-full.txt").write_text(full, encoding="utf-8")
    print(f"wrote {arguments.out / 'llms.txt'} ({len(index.encode())} bytes) and "
          f"{arguments.out / 'llms-full.txt'} ({len(full.encode())} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
