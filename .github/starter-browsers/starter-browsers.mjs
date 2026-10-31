// Drives a running generated starter in Chromium, Firefox and WebKit with the
// Playwright and axe-core pinned by this directory's lockfile. It observes and
// records only: .github/check-starter-browsers.py builds and starts the
// starter, decides what blocks and writes the job summary.
//
// node starter-browsers.mjs --starter blank|saas --base-url http://127.0.0.1:3000 --out DIR
//   [--browsers chromium,firefox,webkit]
// Writes DIR/report.json, plus a Playwright trace and page screenshots per browser.
import { chromium, firefox, webkit } from "playwright";
import { randomBytes } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { join } from "node:path";
import { parseArgs } from "node:util";

const ENGINES = { chromium, firefox, webkit };
const LOOPBACK = new Set(["127.0.0.1", "localhost", "[::1]"]);
const STEP_TIMEOUT = 60_000;
const require = createRequire(import.meta.url);
const AXE_SOURCE = await readFile(require.resolve("axe-core/axe.min.js"), "utf8");
const VERSIONS = {
  playwright: require("playwright/package.json").version,
  "axe-core": require("axe-core/package.json").version,
};

const { values: options } = parseArgs({
  options: {
    starter: { type: "string" },
    "base-url": { type: "string" },
    out: { type: "string" },
    browsers: { type: "string", default: "chromium,firefox,webkit" },
  },
});
const base = new URL(options["base-url"] ?? "invalid:");
if (base.protocol !== "http:" || !LOOPBACK.has(base.hostname)) {
  throw new Error("--base-url must be an http:// loopback URL: only the starter this job started is driven");
}
const FLOWS = { blank: blankFlow, saas: saasFlow };
if (!FLOWS[options.starter] || !options.out) throw new Error("--starter blank|saas and --out are required");
const browsers = options.browsers.split(",");
if (browsers.some(name => !ENGINES[name])) throw new Error(`unknown browser in ${options.browsers}`);

// Runs in the page: marks every element sequential keyboard navigation should
// reach, in document order (the starters use no positive tabindex).
function markFocusables() {
  const selector = "a[href], button, input, select, textarea, summary, [tabindex], [contenteditable='true']";
  const shown = element => {
    const box = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return box.width > 0 && box.height > 0 && style.visibility !== "hidden";
  };
  const list = [...document.querySelectorAll(selector)].filter(element =>
    !element.disabled && element.type !== "hidden" && element.tabIndex >= 0 && !element.closest("[inert]") && shown(element));
  return list.map((element, index) => {
    element.dataset.rullstTab = String(index);
    const name = element.getAttribute("aria-label") || element.textContent.trim() || element.name || element.id;
    return `${element.tagName.toLowerCase()} "${name.slice(0, 40)}"`;
  });
}

// Runs in the page: the focus ring signature of an element (outline, shadow,
// border, colours), compared with and without focus.
function focusSignature(index) {
  const element = index === null ? document.activeElement : document.querySelector(`[data-rullst-tab="${index}"]`);
  if (!element || element === document.body) return { index: null, tag: element ? "body" : "none" };
  const style = getComputedStyle(element);
  const outline = style.outlineStyle === "none" || Number.parseFloat(style.outlineWidth) === 0
    ? "none" : `${style.outlineStyle} ${style.outlineWidth} ${style.outlineColor} ${style.outlineOffset}`;
  const signature = [outline, style.boxShadow, style.borderTopColor, style.borderBottomColor,
    style.backgroundColor, style.color, style.textDecorationLine].join("|");
  return { index: element.dataset.rullstTab ?? null, tag: element.tagName.toLowerCase(), signature };
}

class Session {
  constructor(name, out) {
    this.name = name;
    this.out = out;
    this.findings = [];
    this.steps = [];
    this.shots = 0;
  }

  path() {
    try { return new URL(this.page.url()).pathname; } catch { return "(none)"; }
  }

  record(kind, key, detail, extra = {}) {
    this.findings.push({ browser: this.name, page: this.path(), kind, key, detail, ...extra });
  }

  async open() {
    this.browser = await ENGINES[this.name].launch();
    this.version = this.browser.version();
    // No bypassCSP: the starter's own Content-Security-Policy applies.
    this.context = await this.browser.newContext({ baseURL: base.href, viewport: { width: 1280, height: 900 } });
    this.context.setDefaultTimeout(STEP_TIMEOUT);
    await this.context.tracing.start({ screenshots: true, snapshots: true });
    await this.context.exposeBinding("__rullstCspViolation", (_source, violation) => {
      this.record("csp", `${violation.directive} ${violation.blocked}`, violation);
    });
    await this.context.addInitScript(() => {
      document.addEventListener("securitypolicyviolation", event => {
        window.__rullstCspViolation({
          directive: event.effectiveDirective, blocked: event.blockedURI, disposition: event.disposition,
          source: event.sourceFile, line: event.lineNumber, sample: event.sample,
        });
      }, true);
    });
    this.page = await this.context.newPage();
    this.page.on("console", message => {
      if (message.type() !== "error") return;
      const text = message.text();
      this.record("console", text.slice(0, 160), text, { csp: /content[- ]security[- ]policy/i.test(text) });
    });
    this.page.on("pageerror", error => this.record("pageerror", error.message.slice(0, 160), String(error.stack ?? error)));
    this.page.on("response", response => {
      if (response.status() >= 400 && new URL(response.url()).origin === base.origin) {
        this.record("http", `${response.status()} ${new URL(response.url()).pathname}`, response.request().method());
      }
    });
  }

  async close() {
    if (!this.context) return;
    await this.context.tracing.stop({ path: join(this.out, `${this.name}-trace.zip`) }).catch(() => {});
    await this.browser.close().catch(() => {});
  }

  async screenshot(label) {
    this.shots += 1;
    const file = `${this.name}-${String(this.shots).padStart(2, "0")}-${label.replace(/[^a-z0-9]+/gi, "-")}.png`;
    await this.page.screenshot({ path: join(this.out, file), fullPage: true }).catch(() => {});
  }

  async step(name, action) {
    await action();
    this.steps.push(name);
  }

  async goto(path) {
    const response = await this.page.goto(path, { waitUntil: "load" });
    if (!response?.ok()) throw new Error(`GET ${path} answered ${response?.status() ?? "nothing"}`);
    // Late console and CSP reports (scripts run after load) belong to this page.
    await this.page.waitForTimeout(500);
  }

  async audit() {
    await this.page.evaluate(AXE_SOURCE);
    const violations = await this.page.evaluate(async () => {
      const result = await window.axe.run(document, { resultTypes: ["violations"] });
      return result.violations.map(rule => ({
        id: rule.id, impact: rule.impact, help: rule.help, helpUrl: rule.helpUrl, count: rule.nodes.length,
        nodes: rule.nodes.slice(0, 5).map(node => ({ target: node.target.join(" "), summary: node.failureSummary })),
      }));
    });
    for (const rule of violations) this.record("axe", rule.id, rule.help, rule);
  }

  // Tab from the top of a fresh page: every marked element must be reached
  // once, in order, and look different with focus than without.
  async tabWalk() {
    const expected = await this.page.evaluate(markFocusables);
    const visited = [];
    const focused = new Map();
    for (let press = 0; press < expected.length + 3; press += 1) {
      await this.page.keyboard.press("Tab");
      const state = await this.page.evaluate(focusSignature, null);
      if (state.index === null) {
        if (state.tag === "body" && visited.length > 0) break;
        if (state.tag !== "body") this.record("keyboard", `unexpected stop on ${state.tag}`, "Tab focused an element that is not visible or not interactive");
        continue;
      }
      if (visited.includes(state.index)) break;
      visited.push(state.index);
      focused.set(state.index, state.signature);
    }
    await this.page.evaluate(() => document.activeElement?.blur());
    for (const [index, label] of expected.entries()) await this.checkReached(label, String(index), visited, focused);
    const order = visited.map(Number);
    if (order.some((value, index) => index > 0 && value < order[index - 1])) {
      this.record("keyboard", "tab order", `Tab order ${order.join(",")} differs from the document order`);
    }
  }

  async checkReached(label, index, visited, focused) {
    if (!visited.includes(index)) {
      this.record("keyboard", `not reachable: ${label}`, "Tab never focused this element");
      return;
    }
    const resting = await this.page.evaluate(focusSignature, index);
    if (resting.signature === focused.get(index)) {
      this.record("keyboard", `no visible focus: ${label}`, `focus changes no outline, shadow, border or colour (${resting.signature})`);
    }
  }

  // A page as a visitor first sees it: load, axe audit, keyboard walk.
  async visit(path) {
    await this.step(`${path}: load, axe, Tab walk`, async () => {
      await this.goto(path);
      await this.audit();
      await this.tabWalk();
      await this.screenshot(path);
    });
  }

  async focusByKeyboard(selector) {
    for (let press = 0; press < 25; press += 1) {
      await this.page.keyboard.press("Tab");
      if (await this.page.evaluate(wanted => document.activeElement?.matches(wanted) ?? false, selector)) return;
    }
    throw new Error(`Tab never reached ${selector}`);
  }

  async submitWithEnter(expectedPath) {
    await Promise.all([
      this.page.waitForURL(url => url.pathname === expectedPath),
      this.page.keyboard.press("Enter"),
    ]);
    await this.page.waitForLoadState("load");
  }
}

async function blankFlow(session) {
  await session.visit("/");
  await session.step("HTMX counter submitted with Enter", async () => {
    await session.goto("/");
    await session.page.waitForFunction(() => typeof window.htmx === "object");
    await session.focusByKeyboard("#counter-box button");
    await session.page.evaluate(() => { window.__rullstSameDocument = true; });
    const swapped = session.page.waitForResponse(response => new URL(response.url()).pathname === "/clicked");
    await session.page.keyboard.press("Enter");
    const response = await swapped;
    if (response.status() !== 200 || (await response.request().headerValue("hx-request")) !== "true") {
      throw new Error(`the counter answered ${response.status()} without an HTMX request`);
    }
    await session.page.waitForFunction(() => /server: [1-9]\d*/.test(document.querySelector("#counter-box")?.textContent ?? ""));
    if (!(await session.page.evaluate(() => window.__rullstSameDocument === true))) {
      throw new Error("the counter reloaded the page instead of swapping the fragment");
    }
    const focus = await session.page.evaluate(() => document.activeElement?.matches("#counter-box button") ?? false);
    if (!focus) session.record("keyboard", "focus lost after the HTMX swap", "the increment button no longer has focus after Enter");
    await session.screenshot("htmx-counter");
  });
}

async function fillByKeyboard(session, path, fields) {
  await session.goto(path);
  for (const [selector, value] of fields) {
    await session.focusByKeyboard(selector);
    await session.page.keyboard.type(value);
  }
  const token = await session.page.locator("input[name=_token]").first().inputValue();
  const cookie = (await session.context.cookies(base.href)).find(item => item.name === "rullst_csrf");
  if (!token || cookie?.value !== token) throw new Error(`${path}: the form token does not match the rullst_csrf cookie`);
}

async function signOutByKeyboard(session) {
  await session.goto("/dashboard");
  await session.focusByKeyboard("form[action='/logout'] button");
  await session.submitWithEnter("/login");
  await session.page.goto("/dashboard");
  if (session.path() !== "/login") throw new Error("/dashboard still opens after signing out");
}

async function expectDashboard(session, name) {
  const heading = await session.page.locator("h1").first().textContent();
  if (!heading?.includes(name)) throw new Error(`the dashboard greets "${heading}" instead of ${name}`);
}

async function saasFlow(session) {
  const account = {
    name: "Browser Check",
    email: `browser-${session.name}-${randomBytes(4).toString("hex")}@example.com`,
    password: randomBytes(18).toString("base64url"),
  };
  await session.visit("/pricing");
  await session.visit("/register");
  await session.step("sign-up through the form with Enter (CSRF double submit)", async () => {
    await fillByKeyboard(session, "/register", [["#name", account.name], ["#email", account.email], ["#password", account.password]]);
    await session.submitWithEnter("/dashboard");
    await expectDashboard(session, account.name);
  });
  await session.visit("/dashboard");
  await session.step("sign-out with the keyboard", () => signOutByKeyboard(session));
  await session.visit("/login");
  await session.step("sign-in through the form with Enter (CSRF double submit)", async () => {
    await fillByKeyboard(session, "/login", [["#email", account.email], ["#password", account.password]]);
    await session.submitWithEnter("/dashboard");
    await expectDashboard(session, account.name);
  });
  await session.step("sign-out after sign-in", () => signOutByKeyboard(session));
  await session.visit("/password/forgot");
}

async function run(name) {
  const session = new Session(name, options.out);
  try {
    await session.open();
    await FLOWS[options.starter](session);
  } catch (error) {
    session.record("flow", String(error.message).split("\n")[0].slice(0, 200), String(error.stack ?? error));
    if (session.page) await session.screenshot("failure");
  } finally {
    await session.close();
  }
  return { browser: name, version: session.version ?? null, steps: session.steps, findings: session.findings };
}

await mkdir(options.out, { recursive: true });
const results = [];
for (const name of browsers) results.push(await run(name));
const report = { starter: options.starter, base_url: base.href, versions: VERSIONS, browsers: results };
await writeFile(join(options.out, "report.json"), JSON.stringify(report, null, 2));
console.log(`${options.starter}: ${results.map(r => `${r.browser} ${r.findings.length} finding(s)`).join(", ")}`);
