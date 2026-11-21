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
MongoDB, DuckDB and Qdrant appear only as recipes. Integrations are named in
plain text: a third-party logo may appear only where that brand's published
guidelines allow it to indicate an integration, and the pull request that adds
it names the guideline.

## The landing page

The landing's sections come in this order, with the same ids in both
languages: the first screen (what Rullst is in one sentence, the install
command and three proof points), `#performance`, `#security` (with the
`#verification` list), `#included`, `#integrations`, `#examples`, `#academy`
and `#community`, then the release notice. The validator checks the order, the
sentence, the install command and the three proof points.

- **Performance** shows Rullst as `cargo rullst new` generates it, from the
  same Rullst/Benchmarks `results/site.json` as the comparison page and with
  the same checks (see [the comparison page](#the-comparison-page)). The
  assembler fills two blocks of each landing: `landing-proof` (the first
  screen's memory proof point: RAM under load and after the logins against
  Axum's, and Rullst's lowest speed score) and `landing-benchmarks` (the
  overall score and position range, who shares that range among the rows the
  comparison page shows, the categories level with or above Axum and those
  below it, memory, stability, the minimal row and a legend of every sign
  shown, ‡ included). Without current data each block keeps its pending text;
  the proof point also stays pending when Rullst's RAM was not measured. The
  disclosure that Rullst's maintainer maintains the benchmark stays above the
  results.
- **Security** says what every generated application has on by default, what
  is opt-in and which automated checks run against the framework's code, each
  linked to its workflow. It keeps "no external audit has been completed" (as
  [the assurance case](assurance-case.md) says) and the sole-maintainer
  statement; the validator requires both. A change that is only on `main`
  says which release brings it.
- **Included** lists what ships with the framework. A statement about what
  another framework documents names that framework, its version and the day
  it was checked, in the section's checked line (`checked-security`,
  `checked-included`); the validator requires the line, the same versions as
  the comparison page and the same line in both languages. Recheck those
  statements when Loco or Topcoat releases.
- The validator rejects marketing and absolute wording on the landing
  ("blazing", "the only framework", "unique", "more secure", "experimental"
  and their Portuguese counterparts).

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
and both landings at three widths (as written and with benchmark results
assembled from the fixture, served under `/_rendered/`), reduced motion,
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
the new landing and checks the three first-screen proof cards' actual styles.
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
deployment downloads the site summary that `Rullst/Benchmarks` publishes with
every set of results (`results/site.json` on its `main` branch, read from
`raw.githubusercontent.com` because Cloudflare in front of `rullst.win`
refuses runner requests), saves it as `benchmarks.json` in the working
directory and runs the assembler with `--benchmarks`. The page has no script
and its CSP keeps `connect-src 'none'`. Rullst/Benchmarks documents the file
in its README ("Machine-readable results") and writes it with
`bench/site_summary.py`; `results/latest.json` there (schema
`rullst.benchmarks.latest.v1`, over 1 MiB since the v3 results) is no longer
read by this site. The file must be JSON of this shape (abridged):

```json
{
  "schema": "rullst.benchmarks.site.v2",
  "measured_on": "2026-11-20",
  "rullst_version": "13.0.0-alpha.6",
  "rullst_source": {"kind": "crates.io", "label": "crates.io"},
  "run": {"id": "1", "url": "https://github.com/Rullst/Benchmarks/actions/runs/1"},
  "reference": {"slug": "axum", "framework": "Axum", "version": "0.8.9"},
  "ranked": 14,
  "overall_published": true,
  "weights": {"hello": 2, "database": 2, "application": 2, "memory": 1, "startup": 1},
  "categories": [{"key": "hello", "title": "Hello world"}, {"key": "database", "title": "Database"},
                 {"key": "application", "title": "Application"},
                 {"key": "memory", "title": "Memory (high-water mark)"}, {"key": "startup", "title": "Startup"}],
  "flags": {"cpu_cost": "*", "database_note": "†", "put_not_written": "‡", "within_reference_margin": "≈"},
  "logins": 125,
  "rows": [
    {"slug": "rullst", "version": "13.0.0-alpha.6", "role": "ranked",
     "overall": {"status": "ranked", "score": 0.7489, "position": [4, 5],
                 "margin_percent": 5.48, "flags": ["put_not_written"]},
     "categories": {"hello": {"status": "ranked", "score": 0.497, "position": [3, 6],
                              "margin_percent": 8.98, "flags": ["cpu_cost"]}},
     "stability": "pass",
     "memory": {"peak_ram_mib": 12.1, "axum_peak_ram_mib": 16.6,
                "after_logins_peak_ram_mib": 470.4, "axum_after_logins_peak_ram_mib": 927.2}}
  ],
  "reading": {"en": "Rullst as `cargo rullst new` generates it …", "pt_br": "O Rullst como o `cargo rullst new` o gera …"},
  "note": "",
  "note_pt_br": ""
}
```

The page shows the rows `rullst` (the application `cargo rullst new`
generates, with its security middleware on), `loco`, `topcoat`, `actix`
(Actix Web) and `gin`, ordered by overall position, then `rullst-minimal`
("Rullst (minimal)" / "Rullst (mínimo)", the framework without that
middleware, a diagnostic without a position) last and only when the data has
it; other rows are ignored, and a missing compared framework says it is not
in this run. Each cell shows the position range (a tie such as "3–7"), the
score relative to Axum (1.00×, higher is better everywhere) with its flag
signs and the tie margin (±); a score without a value shows its status
(errors, incomplete, …). A legend under the table explains the position,
margin and every sign that appears, so the ‡ caveat (a PUT that mostly did not
write, which raises the Application and overall scores) is always explained
where it is shown. A second table gives the RAM high-water mark under load
and the peak after the logins of the application endpoints, each with Axum's
on the same runners, and the generated "What this means for Rullst" reading
follows in the page's language (`reading.en` or `reading.pt_br`; backtick
spans become `<code>` as in notes). `note` is shown on the English page and
`note_pt_br` on the Portuguese one (the English note, marked `lang="en"`,
when there is no Portuguese note). The meta line names the date, the Rullst
version and its source, Axum's version and links the measurement run and the
[full results](https://rullst.win/Benchmarks/).

Above the results, a static "How to read these numbers" block in each
template (`benchmark-howto`, six points: 1.00× is Axum, ties and ±, the
categories, the database usually comes first, the overall score is a summary
with the 2:2:2:1:1 weights, and the two Rullst rows) follows the "How to read
these results" section of the Rullst/Benchmarks README; the validator requires
it in both languages, before the results block, naming the minimal row as the
assembler labels it. Rullst's maintainer also maintains that benchmark: the
section says so above the results and the reading ends with the same
disclosure. Show Rullst's weaker results as plainly as its better ones; do
not add wording that ranks or praises a framework beyond the data.

Every value is checked before it reaches the page, and the whole file is
discarded, leaving the template's "no current results" text, when any of
these holds: another schema; a bad date, a date after the build day or a
measurement more than 90 days old (`--benchmarks-max-age-days DAYS`, which
`Rullst/Rullst.github.io` sets with its `BENCHMARKS_MAX_AGE_DAYS` variable);
a category set, weighting (when the overall score is published), flag, flag
sign, status, stability result or role the page does not explain; a
position outside 1 to `ranked`; a negative or non-finite number; an
unexpected version string; a reference other than Axum; a run URL outside
`Rullst/Benchmarks` Actions runs; an empty reading or a note longer than 1,000
characters. A file over 1 MiB is not downloaded. The deployment continues in
every case and logs the reason as a warning, so the page never shows older or
partially understood numbers. A change on the Benchmarks side that the page
must show (a new flag, category, status or field) needs a new schema string
there and a change to the assembler, the templates, this section and the
fixture here.

`.github/fixtures/site-benchmarks.json` is the test fixture (its numbers are
not measurements); `validate-site.py` renders it as of 21 November 2026 and
checks the rendered rows, cells, legend, memory table and reading in both
languages, the landing's proof point and summary (including how it words a
shared position range, a missing Rullst row, missing memory, an unpublished
overall score and a category without a score), and that each kind of rejected
data keeps the pending text on every page; `site-browser-smoke.mjs` renders it
dated the current day and checks the tables and the landing summary at three
widths without horizontal page scrolling. Passing `--organization-site` a
directory assembled with `--benchmarks` previews the pages with real results.

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
