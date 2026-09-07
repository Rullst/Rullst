# Website maintenance and privacy boundaries

The public entry points have different deployment sources:

| Entry point | Repository source | Purpose |
| --- | --- | --- |
| `https://rullst.github.io/` | `Rullst/Rullst.github.io` | Organization landing page and standalone privacy notice |
| `https://rullst.github.io/Rullst/` | This repository's `pages.yml` workflow | Matching landing page, mdBook, images and benchmark dashboards |
| `gh-pages` branch in this repository | Benchmark workflow data | Criterion history consumed by the Pages build; not a development branch |

The landing design and copy have one editable source: `docs/home_template.html`,
`docs/site.css` and `docs/site.js`. Preserve both dedication lines and the v12
preview/v5 end-of-life notice. Use actual source and release evidence for claims;
do not hardcode aspirational coverage, scorecard, speed or certification values.

## Validate before deployment

From the framework checkout, using Node 24 and a locally installed Chromium
(`google-chrome`, or set `CHROME_BIN`):

```bash
mdbook build docs
python3 .github/validate-site.py
node --check docs/site.js
node .github/site-browser-smoke.mjs
```

The browser test checks desktop and 390/320-pixel layouts, keyboard and mobile
navigation, clipboard success/denial, privacy disclosure, reduced motion,
no-JavaScript navigation, resource failures, CSP errors, external requests and
browser storage. It is not a complete accessibility audit or cross-browser
certification. Optional `--screenshots /absolute/output/directory` records
viewport previews without adding binary artifacts to the repository.

The source landing has no analytics, social embeds, remote fonts, cookies or
local/session storage. CSS and JavaScript are local; motion is finite and
respects reduced-motion preferences. Clipboard access follows an explicit
button click and copies only the displayed command.

Hosting still processes requests. The privacy notice links GitHub's statement
and does not promise control of its logs or retention. Benchmark dashboards
load integrity-pinned Chart.js from jsDelivr; documentation can store display
preferences and contain external content. Do not extend the landing's narrower
description to those surfaces or to user-built applications. New tracking,
embeds, forms or storage require a fresh privacy review before deployment.

## Keep the organization website synchronized

Preserve any work in the separate website checkout before running the exporter:

```bash
node .github/export-organization-site.mjs /path/to/clean/Rullst.github.io
node .github/site-browser-smoke.mjs --organization-site /path/to/Rullst.github.io
```

The exporter refuses a dirty tree or wrong repository. It replaces only the
five known website files, derives `privacy.html` from the landing notice and
does not commit, push or deploy. Review the diff and deploy matching framework
documentation first, then the organization website. Keep independent commit
and deployment receipts for each repository. A framework push alone does not
update the organization's root landing page.

The footer's thirteen destination links are the owner's supplied community
list. Instagram, TikTok and YouTube handles are normalized to full profile
URLs. These are normal links, not embedded feeds or a claim that account
availability has been independently verified. Recheck ownership and links
when the owner changes that list.
