// Opens the Scalar API reference that `cargo rullst make:scalar` mounts at
// /docs, in Chromium with the Playwright pinned by this directory's lockfile,
// against the application .github/check-scalar-docs-browser.py started in
// production. The page's own Content-Security-Policy applies (no bypassCSP),
// and Chromium loads the pinned Scalar file from jsDelivr.
//
// It fails on a `securitypolicyviolation` event, a console error, an uncaught
// page error or a same-origin response of 400 or more, and unless the
// operation named by --operation becomes visible. The policy admits the
// style elements Scalar inserts by their hashes, which belong to Scalar
// 1.67.0: an upgrade that inserts other styles fails here until
// `SCALAR_STYLE_HASHES` in rullst-core/src/scalar.rs is updated (Chromium's
// console line, recorded in the report, names the hash it expected).
//
// node scalar-docs.mjs --base-url http://127.0.0.1:3000 --operation TEXT --out DIR
// Writes DIR/report.json and DIR/scalar-docs.png; DIR must be inside
// $RUNNER_TEMP (or the system temporary directory).
import { chromium } from "playwright";
import { mkdir, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { join, resolve, sep } from "node:path";
import { parseArgs } from "node:util";

const LOOPBACK = new Set(["127.0.0.1", "localhost", "[::1]"]);
const RENDER_TIMEOUT = 60_000;
// Late styles, requests and reports after the operation appears.
const SETTLE = 2_000;
const require = createRequire(import.meta.url);

const { values: options } = parseArgs({
  options: {
    "base-url": { type: "string" },
    operation: { type: "string" },
    out: { type: "string" },
  },
});
const base = new URL(options["base-url"] ?? "invalid:");
if (base.protocol !== "http:" || !LOOPBACK.has(base.hostname)) {
  throw new Error("--base-url must be an http:// loopback URL: only the application this job started is driven");
}
if (!options.operation || !options.out) throw new Error("--operation and --out are required");
const OUT_ROOT = resolve(process.env.RUNNER_TEMP || tmpdir());
const OUT = resolve(options.out);
if (!OUT.startsWith(OUT_ROOT + sep)) throw new Error(`--out must be inside ${OUT_ROOT}`);
await mkdir(OUT, { recursive: true });

const findings = [];
const steps = [];
const browser = await chromium.launch();
const report = { playwright: require("playwright/package.json").version, chromium: browser.version(), findings, steps };
try {
  // No bypassCSP: the policy /docs answers with applies.
  const context = await browser.newContext({ baseURL: base.href, viewport: { width: 1280, height: 900 } });
  await context.exposeBinding("__rullstCspViolation", (_source, violation) => findings.push({ kind: "csp", ...violation }));
  await context.addInitScript(() => {
    document.addEventListener("securitypolicyviolation", event => {
      window.__rullstCspViolation({
        directive: event.effectiveDirective, blocked: event.blockedURI, disposition: event.disposition,
        source: event.sourceFile, line: event.lineNumber, sample: event.sample,
      });
    }, true);
  });
  const page = await context.newPage();
  page.on("console", message => {
    if (message.type() === "error") findings.push({ kind: "console", text: message.text() });
  });
  page.on("pageerror", error => findings.push({ kind: "pageerror", text: String(error.stack ?? error) }));
  page.on("response", response => {
    if (response.status() >= 400 && new URL(response.url()).origin === base.origin) {
      findings.push({ kind: "http", status: response.status(), path: new URL(response.url()).pathname });
    }
  });

  const response = await page.goto("/docs", { waitUntil: "load" });
  if (!response?.ok()) throw new Error(`GET /docs answered ${response?.status() ?? "nothing"}`);
  const headers = response.headers();
  report.policy = headers["content-security-policy"] ?? null;
  // The page's own policy, kept by the security baseline, which still adds
  // its other headers.
  if (!report.policy?.includes("@scalar/api-reference@") || !report.policy.includes("style-src-elem")
      || /unsafe-eval/.test(report.policy)) {
    throw new Error(`/docs does not answer with its own policy: ${report.policy}`);
  }
  if (headers["x-content-type-options"] !== "nosniff") throw new Error("the security baseline's headers are missing on /docs");
  steps.push("GET /docs with its own Content-Security-Policy behind the baseline");

  await page.getByText(options.operation, { exact: true }).filter({ visible: true }).first()
    .waitFor({ state: "visible", timeout: RENDER_TIMEOUT });
  steps.push(`operation "${options.operation}" visible`);
  await page.waitForTimeout(SETTLE);
  if (await page.getByText("Scalar UI is unavailable", { exact: false }).count()) {
    throw new Error("the offline fallback replaced the reference");
  }
  // Reports the screenshot itself may raise are not the page's.
  report.findings = [...findings];
  await page.screenshot({ path: join(OUT, "scalar-docs.png"), fullPage: false }).catch(() => {});
} catch (error) {
  report.error = String(error.stack ?? error);
} finally {
  await browser.close().catch(() => {});
}
await writeFile(join(OUT, "report.json"), JSON.stringify(report, null, 2) + "\n");
for (const finding of report.findings) console.error(`finding: ${JSON.stringify(finding)}`);
if (report.error) console.error(report.error);
console.log(`Chromium ${report.chromium}, Playwright ${report.playwright}: ${report.steps.join("; ") || "no step"}; `
  + `${report.findings.length} finding(s)`);
process.exit(report.error || report.findings.length ? 1 : 0);
