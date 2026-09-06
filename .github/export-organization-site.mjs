// Reproducible, local-only synchronization of the separate organization website.
// Does not commit, push, or deploy. Refuses the wrong repository or a dirty tree.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { lstat, readFile, realpath, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
assert(process.argv.length === 3, "Usage: node .github/export-organization-site.mjs /path/to/clean/Rullst.github.io");
const target = resolve(process.argv[2]);
const git = (...args) => execFileSync("git", ["-C", target, ...args], { encoding: "utf8" }).trim();
assert.equal(git("rev-parse", "--show-toplevel"), target, "Destination must be its own repository root");
assert(/(?:github\.com[:/])Rullst\/Rullst\.github\.io(?:\.git)?$/i.test(git("remote", "get-url", "origin")), "Destination must be the official website checkout");
assert.equal(git("status", "--porcelain"), "", "Commit or preserve destination edits before exporting");
const source = await readFile(join(root, "docs/home_template.html"), "utf8");
const home = source.replace('./assets/site.css', './src/style.css').replace('./assets/site.js', './src/main.js');
const privacy = source.match(/      <section id="privacy"[\s\S]+?      <\/section>/)?.[0];
assert(privacy, "Landing source must contain a complete privacy notice");
const privacyPage = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="referrer" content="no-referrer">
  <meta name="description" content="How the Rullst website handles hosting, local resources, linked services and contact messages.">
  <meta http-equiv="Content-Security-Policy" content="default-src 'self'; style-src 'self'; script-src 'none'; img-src 'self'; connect-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'">
  <title>Website privacy notice — Rullst</title>
  <link rel="canonical" href="https://rullst.github.io/privacy.html">
  <link rel="icon" type="image/png" href="/Rullst/Rullst.png">
  <link rel="stylesheet" href="./src/style.css">
</head>
<body>
  <nav class="site-nav" aria-label="Primary navigation"><div class="shell nav-inner"><a class="brand" href="/">rullst<span class="brand-period">.</span></a><a href="/#privacy">Back to the website ↗</a></div></nav>
  <main class="shell section"><p class="eyebrow">Transparency, without blanket promises</p><h1>Website privacy notice.</h1>
${privacy.replace('<details>', '<details open>')}
  </main>
  <footer class="shell footer-bottom"><a href="/">Rullst home</a><a href="mailto:officialrullst@gmail.com">Contact the team</a></footer>
</body>
</html>
`;
// Only these five known, already tracked site files are replaced. The framework
// checkout and all unrelated destination files remain untouched.
const updates = new Map([
  ["index.html", home],
  ["privacy.html", privacyPage],
  ["src/style.css", await readFile(join(root, "docs/site.css"), "utf8")],
  ["src/main.js", await readFile(join(root, "docs/site.js"), "utf8")],
  ["README.md", `# Rullst website\n\nSource for [rullst.github.io](https://rullst.github.io/).\n\nThis static website describes the unreleased v12 preview honestly: main is active v12 work; v5 is frozen and end-of-life. It makes no universal performance, security or legal-compliance guarantee.\n\n## Source of truth\n\nThe design and copy are maintained in the [framework repository](https://github.com/Rullst/Rullst): docs/home_template.html, docs/site.css and docs/site.js. The website privacy page is generated from that same landing notice.\n\nAfter preserving all local changes, run from the framework checkout:\n\n\`\`\`bash\nnode .github/export-organization-site.mjs /path/to/clean/Rullst.github.io\n\`\`\`\n\nReview the resulting diff, test it, then commit and deploy separately. The exporter never pushes. Publish matching framework documentation first: /Rullst/book/, /Rullst/images/ and /Rullst/Rullst.png are served by the framework Pages deployment.\n\n## Verification\n\nThe framework's static validator and real Chromium smoke checks exercise the source landing page at desktop and mobile widths, keyboard navigation, clipboard success/denial, privacy details, reduced motion and no-JavaScript navigation. The landing page has no analytics, social embeds or browser storage; linked documentation/benchmarks and GitHub hosting have separate boundaries described in the notice.\n\n## Contributing\n\nPrefer a focused change to the framework source followed by this export, so the two entry points stay aligned. Use conventional commits, for example: feat(site): improve navigation. No npm bundle is required.\n`],
]);
for (const name of updates.keys()) {
  git("ls-files", "--error-unmatch", "--", name);
  const file = join(target, name);
  assert((await lstat(file)).isFile(), `Destination must be a regular file: ${name}`);
  assert.equal(await realpath(file), file, `Destination must not traverse symlinks: ${name}`);
}
for (const [name, contents] of updates) {
  await writeFile(join(target, name), contents);
}
console.log(`Exported ${updates.size} known files to ${target}; review, commit and deploy separately.`);
