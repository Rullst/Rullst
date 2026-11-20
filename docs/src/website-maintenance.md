# Website maintenance and privacy boundaries

`https://rullst.win/` has one landing page, in English at `/` and in
Brazilian Portuguese at `/pt-br/`. Its source lives in this repository; two
deployments publish it:

| Address | Built by | Contents |
| --- | --- | --- |
| `https://rullst.win/`, `/privacy.html`, `/compare/`, `/pt-br/`, `/pt-br/privacy.html`, `/pt-br/comparar/`, `/assets/` | `Rullst/Rullst.github.io`, workflow `deploy.yml` | The landing, privacy and comparison pages in both languages, the stylesheet, script and language-switcher flags |
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
| `compare_template.html` | `/compare/` |
| `compare_template.pt-BR.html` | `/pt-br/comparar/` |
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
python3 .github/set-book-language.py
python3 .github/validate-site.py
python3 .github/test-llms-txt.py
node --check docs/site.js
node .github/site-browser-smoke.mjs
```

mdBook writes the book's language, `en`, into the `<html lang>` of every
page. `.github/set-book-language.py`, which `pages.yml` and
`documentation.yml` run after the build, sets `lang="pt-BR"` on the pages
under `pt-BR/`, so screen readers read them in Portuguese, and fails when a
translated page or any other page declares the wrong language, or a
translation has no page.

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
greeting, clipboard success/denial, both privacy pages, both comparison pages
at three widths (as written and with benchmark rows assembled from the
fixture, served under `/_rendered/`), reduced motion,
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

## The comparison page

`/compare/` and `/pt-br/comparar/` compare Rullst, Loco and Topcoat topic by
topic. Every Loco and Topcoat fact comes from that project's own
documentation, repository or crates.io page, and the page states the versions
and the date it was checked against (`loco-rs 1.3.1` and `topcoat 0.12.1` on
14 November 2026); Rullst facts come from this repository. When either
project releases a version that changes a fact, update both languages and the
checked versions together: the validator requires the same topics, the same
checked versions and dates and the same external links in both pages. Say
"not documented" rather than guessing, and keep the "where each one fits"
section fair to all three.

A "Why not Leptos or Dioxus?" / "Por que não Leptos ou Dioxus?" section
(`ui-frameworks`, between the topics and Performance) explains why the page
leaves those two out: they are UI frameworks whose components render on the
server and run in the browser as WebAssembly, with server functions; they ship
no ORM, migrations or authentication; their server side runs on Axum (and, for
Leptos, Actix Web), which is already the benchmark's baseline; and a Leptos or
Dioxus interface can call a Rullst API. Its facts come from the Leptos book,
the Leptos repository, the Dioxus 0.7 documentation and crates.io, and its own
checked line (`checked-ui`) names `leptos 0.8.23` and `dioxus 0.7.10`, checked
on 19 November 2026 (stable releases; `leptos 0.9.0-rc1` and
`dioxus 0.8.0-alpha.3` were prereleases then). The validator applies the same
rules to both checked lines, `checked` and `checked-ui`: the same versions and
date in both languages, the named packages present, and a crates.io link in
Sources for every package a line names. Recheck the section when either project
releases a new stable line.

Its Performance section is filled at build time, never by hand. The
deployment downloads the latest results that `Rullst/Benchmarks` publishes and
saves them as `benchmarks.json` in the working directory and runs the
assembler with `--benchmarks`, which renders the
Rullst, Rullst (minimal), Loco and Topcoat rows into both languages. The page has no script and
its CSP keeps `connect-src 'none'`. The file must be JSON of this shape:

```json
{
  "schema": "rullst.benchmarks.latest.v1",
  "measured_on": "2026-11-20",
  "note": "Optional note about this run.",
  "note_pt_br": "Nota opcional sobre esta rodada.",
  "run": {"url": "https://github.com/Rullst/Benchmarks/actions/runs/1"},
  "results": [
    {"framework": "Rullst", "version": "13.0.0-alpha.6",
     "json_rps": 90485.67, "db_single_rps": 16186.7,
     "avg_latency_ms": 3.14, "peak_ram_mib": 20.83,
     "vs_axum": {"json_rps": 0.48, "db_single_rps": 0.81,
                 "avg_latency_ms": 2.1, "peak_ram_mib": 1.9}},
    {"framework": "Rullst (minimal)", "variant": "minimal",
     "version": "13.0.0-alpha.6", "json_rps": 114421.89,
     "vs_axum": {"json_rps": 0.9958}}
  ]
}
```

`framework` is matched without regard to case; at least one of the four
metrics is required per row and other frameworks are ignored. `Rullst` is the
application `cargo rullst new` generates, with its security middleware on;
`Rullst (minimal)` is the framework without that middleware (the
`frameworks/rullst-minimal` folder of Rullst/Benchmarks). The minimal row is
shown after Rullst, labelled "Rullst (minimal)" / "Rullst (mínimo)" with a line
saying it lacks the generated app's security layers, and is left out without a
placeholder when the data has no such row; a missing Rullst, Loco or Topcoat
row says it is not in this run. When a row has
`vs_axum` (each metric divided by Axum's, measured on the same machine in the
same round), the page shows that ratio first and the absolute median in
parentheses: the runs use different GitHub-hosted machines, so the ratio is
the comparable number. Above the rows, a static "How to read these numbers"
block in each template (`benchmark-howto`, four points) explains 1.00×, which
direction is better, why ratios beat absolute numbers and the two Rullst rows;
the validator requires it in both languages, before the results block, naming
the minimal row as the assembler labels it. `note` is shown on the English page and `note_pt_br`
on the Portuguese one (the English note, marked `lang="en"`, when there is no
Portuguese note); `run.url` must be a `Rullst/Benchmarks` Actions run and is
linked as the measurement run. An unknown schema, a bad date, a negative or
non-finite number, an unexpected version string, a note longer than 1,000
characters or another run URL discards the whole file: the page then keeps
its "first measurement in progress" text and the deployment continues. `.github/fixtures/site-benchmarks.json`
is the test fixture (its numbers are not measurements).

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
