# Website maintenance and privacy boundaries

`https://rullst.win/` has one landing page, in English at `/` and in
Brazilian Portuguese at `/pt-br/`. Its source lives in this repository; two
deployments publish it:

| Address | Built by | Contents |
| --- | --- | --- |
| `https://rullst.win/`, `/privacy.html`, `/pt-br/`, `/pt-br/privacy.html`, `/assets/` | `Rullst/Rullst.github.io`, workflow `deploy.yml` | The landing and privacy pages in both languages, the stylesheet, script and language-switcher flags |
| `https://rullst.win/Rullst/` | This repository's `pages.yml` | Only a redirect page to `https://rullst.win/`; nothing else moved |
| `https://rullst.win/Rullst/book/`, `/Rullst/benches/`, `/Rullst/images/`, `/Rullst/llms.txt` | This repository's `pages.yml` | The mdBook, the benchmark dashboards, images and the LLM text indexes |
| `https://rullst.win/Benchmarks/` | `Rullst/Benchmarks` | The cross-framework comparison (a separate project) |
| `gh-pages` branch in this repository | Benchmark workflow data | Criterion history consumed by the Pages build; not a development branch |

The landing design and copy have one editable source, in `docs/`:

| File | Published at |
| --- | --- |
| `home_template.html` | `/` |
| `home_template.pt-BR.html` | `/pt-br/` |
| `privacy_template.html` | `/privacy.html` |
| `privacy_template.pt-BR.html` | `/pt-br/privacy.html` |
| `site.css`, `site.js` | `/assets/site.css`, `/assets/site.js` |
| `flags/us.svg`, `flags/br.svg` | `/assets/flags/` |
| `redirect_template.html` | `/Rullst/` (by `pages.yml`) |

Change both languages in the same pull request: `validate-site.py` fails when
the two landing pages differ in section ids, external links (apart from the
listed language counterparts, such as the Academy's `/pt-BR` pages) or
integration names, or when the privacy pages differ in section ids or
external links. The interface text that `site.js` writes (copy status, intro
and Academy buttons) is chosen by the page's `<html lang>`. The documentation
and the benchmark dashboards exist in English only; the Portuguese page says
so and links to them.

Preserve the footer dedication and the release notice and keep it current: the
12.x line reached end of life on 16 October 2026, v13 is in prerelease until
13.0.0, and v5 is end of life. The install command must pin the current
`cargo-rullst` version; the validator checks it. Use actual source and release
evidence for claims; do not hardcode aspirational coverage, scorecard, speed or
certification values. The Integrations section lists exactly the entries of
README.md's Integrations table (the validator compares them item by item), and
MongoDB, DuckDB and Qdrant appear only as recipes.

## Validate before deployment

From the framework checkout, using Node 24 and a locally installed Chromium
(`google-chrome`, or set `CHROME_BIN`):

```bash
mdbook build docs
python3 .github/validate-site.py
python3 .github/test-llms-txt.py
node --check docs/site.js
node .github/site-browser-smoke.mjs
```

The Pages build also writes `llms.txt` (an index of every book page in
`SUMMARY.md` order) and `llms-full.txt` (their Markdown, with mdBook include
directives expanded; pages over 64 KiB other than the spec and the CLI
reference are listed under `## Optional`) to `/Rullst/` with `.github/generate-llms-txt.py`.
They are generated on every deployment and never edited by hand; preview them
with `python3 .github/generate-llms-txt.py --out <directory>`.
`validate-site.py` generates both files and fails when either is missing a
`SUMMARY.md` page or exceeds its size bound (128 KiB and 8 MiB).

The browser test checks both languages at desktop and 390/320-pixel layouts,
the flag language switcher, keyboard and mobile navigation, the Academy
greeting, clipboard success/denial, both privacy pages, reduced motion,
no-JavaScript navigation, resource failures, CSP errors, external requests and
browser storage. It is not a complete accessibility audit or cross-browser
certification. Optional `--screenshots /absolute/output/directory` records
viewport previews without adding binary artifacts to the repository.

### Keep cached assets in sync with the page

The landing references CSS and JavaScript with a content version, such as
`/assets/site.css?v=<first-16-hex-digits-of-SHA-256>`. After editing either
asset, compute its SHA-256 and update its reference in all four page
templates. The validator and the assembler reject stale versions. This is cache
invalidation, not an artifact-signing or browser integrity guarantee.

The browser regression check first stores an old unversioned stylesheet and
script in a real four-hour browser cache, confirms they are reused, then loads
the new landing and checks the three illustrated module cards' actual styles.
The privacy pages share the versioned stylesheet. Keep query
parameters in any CDN cache key; an intermediary configured to ignore them
requires a cache-policy change or content-hashed asset filenames instead.
Previously cached HTML can still show the previous design until refreshed.

The source landing has no analytics, social embeds, remote fonts, cookies or
local/session storage. CSS and JavaScript are local; ambient, hero, workflow and
scroll-reveal motion is finite, progressively enhanced and disabled by the
reduced-motion preference. Content stays visible when JavaScript is unavailable.
Clipboard access follows an explicit button click and copies only the displayed
command.

Hosting still processes requests. The privacy notice links GitHub's statement
and does not promise control of its logs or retention. Benchmark dashboards
load integrity-pinned Chart.js from jsDelivr; documentation can store display
preferences and contain external content. Do not extend the landing's narrower
description to those surfaces or to user-built applications. New tracking,
embeds, forms or storage require a fresh privacy review before deployment.

## How the root site is deployed

`Rullst/Rullst.github.io` has no copy of these files. Its `deploy.yml`
workflow runs daily, on a push to that repository and on demand: it checks out
this repository's `main` branch read-only (no token beyond its own read-only
`GITHUB_TOKEN`, no secret), runs

```bash
python3 .github/assemble-organization-site.py --out public_site
```

and deploys the result with `actions/deploy-pages`. The assembler refuses a
stale asset version, a missing canonical URL or CSP, or an asset the pages
reference but the site does not publish. To preview the root site locally:

```bash
python3 .github/assemble-organization-site.py --out /path/to/empty/directory
node .github/site-browser-smoke.mjs --organization-site /path/to/empty/directory
```

A merge to `main` therefore reaches `https://rullst.win/` at the next daily run;
run the `deploy.yml` workflow of `Rullst/Rullst.github.io` by hand to publish it
sooner. The pages also load `/Rullst/Rullst.png` and `/Rullst/images/`, which
`pages.yml` deploys from the same commit.

The footer's seventeen destination links are the owner's supplied community
list. Instagram, TikTok and YouTube handles are normalized to full profile
URLs. These are normal links, not embedded feeds or a claim that account
availability has been independently verified. Recheck ownership and links
when the owner changes that list.
