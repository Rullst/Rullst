#!/usr/bin/env python3
"""Keep repository Markdown links usable when canonical pages enter the book.

Only repository-owned .md destinations are rebased. Chapter destinations stay
inside the book; other Markdown files link to their canonical repository page.
Source files, external URLs, fragments and fenced examples remain unchanged.
"""
import json
import os
from pathlib import Path
import re
import sys
from urllib.parse import quote, unquote, urlsplit, urlunsplit

INCLUDES = {'roadmap.md': 'ROADMAP.md', 'workflows.md': 'WORKFLOWS.md'}
INLINE = re.compile(r'(?P<prefix>\]\(<?)(?P<url>[^\s<>)]*\.md(?:[?#][^\s<>)]*)?)(?=[\s>)])')
REFERENCE = re.compile(r'(?P<prefix>^ {0,3}\[[^\]\n]+\]:\s*<?)(?P<url>[^\s<>]*\.md(?:[?#][^\s<>]*)?)(?=[\s>]|$)')


def chapters(items):
    for item in items:
        chapter = item.get('Chapter') if isinstance(item, dict) else None
        if chapter:
            yield chapter
            yield from chapters(chapter.get('sub_items', []))


def process(context, book):
    root = Path(context['root']).resolve()
    repository = root.parent
    source = root / context['config']['book'].get('src', 'src')
    pages = list(chapters(book.get('items', book.get('sections', []))))
    aliases = {}
    origins = {}
    for page in pages:
        if not page.get('source_path') or not page.get('path'):
            continue
        origin = (source / page['source_path']).resolve()
        if not origin.is_relative_to(repository):
            raise ValueError('chapter source escapes the repository')
        canonical = INCLUDES.get(page['source_path'])
        if canonical:
            marker = '{{#include ../../' + canonical + '}}'
            if marker not in page['content']:
                raise ValueError('canonical include must run before the links preprocessor')
            origin = repository / canonical
            page['content'] = page['content'].replace(marker, origin.read_text())
        aliases[origin] = page['path']
        aliases[(source / page['source_path']).resolve()] = page['path']
        origins[page['path']] = origin
    config = context['config']['preprocessor']['canonical-links']
    revision = quote(config['source-revision'], safe='')
    repository_url = context['config']['output']['html']['git-repository-url'].rstrip('/')
    for page in pages:
        origin = origins.get(page.get('path'))
        if origin is None:
            continue

        def rewrite(match):
            value = urlsplit(match['url'])
            if value.scheme or value.netloc or not value.path:
                return match[0]
            target = (origin.parent / unquote(value.path)).resolve()
            if not target.is_relative_to(repository) or not target.is_file():
                return match[0]  # The normal link validator reports missing files.
            if target in aliases:
                path = Path(os.path.relpath(aliases[target], Path(page['path']).parent)).as_posix()
                destination = urlunsplit(('', '', path, value.query, value.fragment))
            else:
                path = quote(target.relative_to(repository).as_posix(), safe='/')
                destination = f'{repository_url}/blob/{revision}/{path}'
                if value.query:
                    destination += '?' + value.query
                if value.fragment:
                    destination += '#' + value.fragment
            return match['prefix'] + destination

        lines = []
        fence = None
        for line in page['content'].splitlines(keepends=True):
            marker = re.match(r'^ {0,3}(`{3,}|~{3,})', line)
            if marker:
                token = marker[1]
                if fence is None:
                    fence = token
                elif token[0] == fence[0] and len(token) >= len(fence):
                    fence = None
                lines.append(line)
            elif fence is not None:
                lines.append(line)
            else:
                lines.append(REFERENCE.sub(rewrite, INLINE.sub(rewrite, line)))
        page['content'] = ''.join(lines)
    return book


if __name__ == '__main__':
    if len(sys.argv) > 1:
        sys.exit(0 if sys.argv[1:] == ['supports', 'html'] else 1)
    context, book = json.load(sys.stdin)
    json.dump(process(context, book), sys.stdout)
