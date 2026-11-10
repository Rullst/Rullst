// Drives the UI component gallery (.github/fixtures/ui-gallery, prepared and
// started by .github/check-ui-components.py) in Chromium, Firefox and WebKit
// with the Playwright and axe-core pinned by this directory's lockfile. It
// observes and records only; the Python script decides what blocks, with the
// rules of .github/check-starter-browsers.py.
//
// The page is audited and walked with Tab in the light and the dark colour
// scheme; then the keyboard paths run: the dialog (open, Escape, the close
// command, focus return), the menu (arrow keys, Home, End, Escape), the tabs
// (arrow keys, Home, End), the htmx toast and the button's loading state.
//
// node ui-components.mjs --base-url http://127.0.0.1:3000 --out DIR [--browsers LIST]
import { chromium, firefox, webkit } from "playwright";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { join, resolve, sep } from "node:path";
import { parseArgs } from "node:util";

const ENGINES = { chromium, firefox, webkit };
const LOOPBACK = new Set(["127.0.0.1", "localhost", "[::1]"]);
const require = createRequire(import.meta.url);
const AXE_SOURCE = await readFile(require.resolve("axe-core/axe.min.js"), "utf8");
const VERSIONS = {
  playwright: require("playwright/package.json").version,
  "axe-core": require("axe-core/package.json").version,
};

const { values: options } = parseArgs({
  options: {
    "base-url": { type: "string" },
    out: { type: "string" },
    browsers: { type: "string", default: "chromium,firefox,webkit" },
  },
});
const base = new URL(options["base-url"] ?? "invalid:");
if (base.protocol !== "http:" || !LOOPBACK.has(base.hostname)) {
  throw new Error("--base-url must be an http:// loopback URL: only the gallery this job started is driven");
}
const OUT_ROOT = resolve(process.env.RUNNER_TEMP || tmpdir());
const OUT = resolve(options.out ?? "");
if (!options.out || !OUT.startsWith(OUT_ROOT + sep)) throw new Error(`--out must be inside ${OUT_ROOT}`);
const browsers = options.browsers.split(",");
if (browsers.some((name) => !ENGINES[name])) throw new Error(`unknown browser in ${options.browsers}`);

// In the page: marks the elements Tab should reach, in document order.
function markFocusables() {
  const selector = "a[href], button, input, select, textarea, summary, [tabindex], [contenteditable='true']";
  const shown = (element) => {
    const box = element.getBoundingClientRect();
    return box.width > 0 && box.height > 0 && getComputedStyle(element).visibility !== "hidden";
  };
  const list = [...document.querySelectorAll(selector)].filter((element) =>
    !element.disabled && element.type !== "hidden" && element.tabIndex >= 0 && !element.closest("[inert]") && shown(element));
  return list.map((element, index) => {
    element.dataset.rullstTab = String(index);
    const name = element.getAttribute("aria-label") || element.textContent.trim() || element.name || element.id;
    return `${element.tagName.toLowerCase()} "${name.slice(0, 40)}"`;
  });
}

// In the page: what focus changes on an element (outline, shadow, colours).
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
  constructor(name, scheme) {
    this.name = name;
    this.scheme = scheme;
    this.findings = [];
    this.steps = [];
    this.shots = 0;
    this.captureReports = 0;
    this.capturing = false;
  }

  path() {
    let path = "(none)";
    try { path = new URL(this.page.url()).pathname; } catch { /* no page yet */ }
    return this.scheme === "dark" ? `${path} (dark)` : path;
  }

  record(kind, key, detail, extra = {}) {
    this.findings.push({ browser: this.name, page: this.path(), kind, key, detail, ...extra });
  }

  async open(browser) {
    // No bypassCSP: the gallery's own Content-Security-Policy applies.
    this.context = await browser.newContext({
      baseURL: base.href, viewport: { width: 1280, height: 900 }, colorScheme: this.scheme,
    });
    this.context.setDefaultTimeout(30_000);
    await this.context.tracing.start({ screenshots: true, snapshots: true });
    await this.context.exposeBinding("__rullstCspViolation", (_source, violation) => {
      if (violation.duringCapture) this.captureReports += 1;
      else this.record("csp", `${violation.directive} ${violation.blocked}`, violation);
    });
    await this.context.addInitScript(() => {
      document.addEventListener("securitypolicyviolation", (event) => {
        window.__rullstCspViolation({
          directive: event.effectiveDirective, blocked: event.blockedURI, disposition: event.disposition,
          source: event.sourceFile, line: event.lineNumber, sample: event.sample,
          duringCapture: window.__rullstCapturing === true,
        });
      }, true);
    });
    this.page = await this.context.newPage();
    this.page.on("console", (message) => {
      if (message.type() !== "error") return;
      const text = message.text();
      const csp = /content[- ]security[- ]policy/i.test(text);
      if (csp && this.capturing) return;
      this.record("console", text.slice(0, 160), text, { csp });
    });
    this.page.on("pageerror", (error) => this.record("pageerror", error.message.slice(0, 160), String(error.stack ?? error)));
    this.page.on("response", (response) => {
      if (response.status() >= 400 && new URL(response.url()).origin === base.origin) {
        this.record("http", `${response.status()} ${new URL(response.url()).pathname}`, response.request().method());
      }
    });
  }

  async close() {
    if (!this.context) return;
    await this.context.tracing.stop({ path: join(OUT, `${this.name}-${this.scheme}-trace.zip`) }).catch(() => {});
    await this.context.close().catch(() => {});
  }

  async screenshot(label) {
    this.shots += 1;
    const file = `${this.name}-${this.scheme}-${String(this.shots).padStart(2, "0")}-${label.replace(/[^a-z0-9]+/gi, "-")}.png`;
    // WebKit's screenshot injects an inline <style> the CSP reports; those
    // reports are counted apart (see .github/starter-browsers.mjs).
    const capturing = (value) => this.page.evaluate((flag) => { window.__rullstCapturing = flag; }, value).catch(() => {});
    this.capturing = true;
    await capturing(true);
    await this.page.screenshot({ path: join(OUT, file), fullPage: true }).catch(() => {});
    await capturing(false);
    this.capturing = false;
  }

  async step(name, action) {
    await action();
    this.steps.push(`${name} (${this.scheme})`);
  }

  async goto(path) {
    const response = await this.page.goto(path, { waitUntil: "load" });
    if (!response?.ok()) throw new Error(`GET ${path} answered ${response?.status() ?? "nothing"}`);
    await this.page.waitForFunction(() => typeof window.htmx === "object");
    await this.page.waitForTimeout(500);
  }

  async audit(context = "document") {
    await this.page.evaluate(AXE_SOURCE);
    const violations = await this.page.evaluate(async (selector) => {
      const target = selector === "document" ? document : document.querySelector(selector);
      const result = await window.axe.run(target, { resultTypes: ["violations"] });
      return result.violations.map((rule) => ({
        id: rule.id, impact: rule.impact, help: rule.help, helpUrl: rule.helpUrl, count: rule.nodes.length,
        nodes: rule.nodes.slice(0, 5).map((node) => ({ target: node.target.join(" "), summary: node.failureSummary })),
      }));
    }, context);
    for (const rule of violations) this.record("axe", rule.id, rule.help, rule);
  }

  // Presses Tab from the top until focus comes back or leaves the page;
  // returns the marked indexes in visiting order and their focus signatures.
  async pressTabs(count) {
    const visited = [];
    const focused = new Map();
    for (let press = 0; press < count; press += 1) {
      await this.page.keyboard.press("Tab");
      const state = await this.page.evaluate(focusSignature, null);
      if (state.index !== null && visited.includes(state.index)) break;
      if (state.index !== null) {
        visited.push(state.index);
        focused.set(state.index, state.signature);
      } else if (state.tag !== "body") {
        this.record("keyboard", `unexpected stop on ${state.tag}`, "Tab focused an element that is not visible or not interactive");
      } else if (visited.length > 0) {
        break;
      }
    }
    return { visited, focused };
  }

  // Tab from the top of a fresh page: every marked element must be reached
  // once, in order, and look different with focus than without.
  async tabWalk() {
    const expected = await this.page.evaluate(markFocusables);
    const { visited, focused } = await this.pressTabs(expected.length + 3);
    await this.page.evaluate(() => document.activeElement?.blur());
    for (const [index, label] of expected.entries()) {
      if (!visited.includes(String(index))) {
        this.record("keyboard", `not reachable: ${label}`, "Tab never focused this element");
        continue;
      }
      const resting = await this.page.evaluate(focusSignature, String(index));
      if (resting.signature === focused.get(String(index))) {
        this.record("keyboard", `no visible focus: ${label}`, `focus changes no outline, shadow, border or colour (${resting.signature})`);
      }
    }
    const order = visited.map(Number);
    if (order.some((value, index) => index > 0 && value < order[index - 1])) {
      this.record("keyboard", "tab order", `Tab order ${order.join(",")} differs from the document order`);
    }
  }

  async focusByKeyboard(selector) {
    await this.page.evaluate(() => document.activeElement?.blur());
    for (let press = 0; press < 80; press += 1) {
      await this.page.keyboard.press("Tab");
      if (await this.page.evaluate((wanted) => document.activeElement?.matches(wanted) ?? false, selector)) return;
    }
    throw new Error(`Tab never reached ${selector}`);
  }

  async active(selector) {
    return this.page.evaluate((wanted) => document.activeElement?.matches(wanted) ?? false, selector);
  }

  async expect(condition, key, detail) {
    if (!(await condition)) this.record("keyboard", key, detail);
  }
}

const DIALOG = "#confirm";
const OPEN_DIALOG = 'button[commandfor="confirm"][command="show-modal"]';
const MENU = "#actions";
const MENU_BUTTON = "#actions-button";
const ITEMS = '#actions [role="menuitem"]';

async function dialogPaths(session) {
  const { page } = session;
  const open = () => page.evaluate((selector) => document.querySelector(selector).open, DIALOG);
  await session.focusByKeyboard(OPEN_DIALOG);
  await page.keyboard.press("Enter");
  await page.waitForFunction((selector) => document.querySelector(selector).open, DIALOG);
  await session.expect(page.evaluate((selector) => document.querySelector(selector).matches(":modal"), DIALOG),
    "dialog is not modal", "show-modal did not open the dialog as a modal");
  await session.expect(page.evaluate((selector) => document.querySelector(selector).contains(document.activeElement), DIALOG),
    "dialog focus", "focus did not move into the opened dialog");
  await session.audit(DIALOG);
  await session.screenshot("dialog-open");
  await page.keyboard.press("Escape");
  await page.waitForFunction((selector) => !document.querySelector(selector).open, DIALOG);
  await session.expect(session.active(OPEN_DIALOG), "dialog focus return after Escape", "focus did not return to the button that opened the dialog");
  // Again, closed with the footer's close command.
  await page.keyboard.press("Enter");
  await page.waitForFunction((selector) => document.querySelector(selector).open, DIALOG);
  for (let press = 0; press < 10 && !(await session.active('#confirm button[command="close"]:not([aria-label])')); press += 1) {
    await page.keyboard.press("Tab");
  }
  if (!(await session.active('#confirm button[command="close"]:not([aria-label])'))) throw new Error("Tab never reached the dialog's Cancel button");
  await page.keyboard.press("Enter");
  await page.waitForFunction((selector) => !document.querySelector(selector).open, DIALOG);
  await session.expect(session.active(OPEN_DIALOG), "dialog focus return after close", "focus did not return after the close command");
  if (await open()) throw new Error("the dialog is still open");
}

async function menuPaths(session) {
  const { page } = session;
  const focusedItem = () => page.evaluate((selector) => [...document.querySelectorAll(selector)].indexOf(document.activeElement), ITEMS);
  const count = await page.locator(ITEMS).count();
  const openMenu = () => page.evaluate((selector) => document.querySelector(selector).matches(":popover-open"), MENU);
  await session.focusByKeyboard(MENU_BUTTON);
  await page.keyboard.press("Enter");
  await page.waitForFunction((selector) => document.querySelector(selector).matches(":popover-open"), MENU);
  await page.waitForFunction((selector) => document.querySelector(selector).contains(document.activeElement), MENU).catch(() => {});
  const moves = [["(open)", 0], ["ArrowDown", 1], ["ArrowUp", 0], ["ArrowUp", count - 1], ["ArrowDown", 0], ["End", count - 1], ["Home", 0]];
  for (const [key, expected] of moves) {
    if (key !== "(open)") await page.keyboard.press(key);
    const index = await focusedItem();
    if (index !== expected) session.record("keyboard", `menu ${key}`, `focus is on item ${index}, expected ${expected}`);
  }
  await session.expect(page.evaluate((selector) => document.querySelector(selector).getAttribute("aria-expanded") === "true", MENU_BUTTON),
    "menu aria-expanded", "the open menu's button does not say aria-expanded=true");
  await session.audit(MENU);
  await session.screenshot("menu-open");
  await page.keyboard.press("Escape");
  await page.waitForFunction((selector) => !document.querySelector(selector).matches(":popover-open"), MENU);
  await page.waitForTimeout(100);
  await session.expect(session.active(MENU_BUTTON), "menu focus return", "focus did not return to the menu button after Escape");
  await session.expect(page.evaluate((selector) => document.querySelector(selector).getAttribute("aria-expanded") === "false", MENU_BUTTON),
    "menu aria-expanded after close", "the closed menu's button still says aria-expanded=true");
  // ArrowUp on the button opens the menu on its last item.
  await page.keyboard.press("ArrowUp");
  await page.waitForFunction((selector) => document.querySelector(selector).matches(":popover-open"), MENU);
  await page.waitForTimeout(100);
  const last = await focusedItem();
  if (last !== count - 1) session.record("keyboard", "menu ArrowUp on the button", `focus is on item ${last}, expected ${count - 1}`);
  await page.keyboard.press("Escape");
  if (await openMenu()) throw new Error("the menu is still open after Escape");
}

async function tabPaths(session) {
  const { page } = session;
  const state = () => page.evaluate(() => {
    const tabs = [...document.querySelectorAll('#settings-tab-profile, #settings-tab-billing, #settings-tab-team')];
    return {
      focused: tabs.indexOf(document.activeElement),
      selected: tabs.findIndex((tab) => tab.getAttribute("aria-selected") === "true"),
      shown: tabs.map((tab) => !document.getElementById(tab.getAttribute("aria-controls")).hidden),
      tabbable: tabs.filter((tab) => tab.tabIndex === 0).length,
    };
  });
  await session.focusByKeyboard('[role="tab"][aria-selected="true"]');
  for (const [key, expected] of [["ArrowRight", 1], ["ArrowRight", 2], ["ArrowRight", 0], ["ArrowLeft", 2], ["Home", 0], ["End", 2]]) {
    await page.keyboard.press(key);
    const now = await state();
    const shown = now.shown.map((value, index) => value === (index === expected)).every(Boolean);
    if (now.focused !== expected || now.selected !== expected || !shown || now.tabbable !== 1) {
      session.record("keyboard", `tabs ${key}`, `expected tab ${expected}: ${JSON.stringify(now)}`);
    }
  }
  // Tab moves from the selected tab into its panel.
  await page.keyboard.press("Tab");
  await session.expect(session.active("#settings-panel-team"), "tabs panel focus", "Tab from the selected tab does not reach its panel");
  await session.screenshot("tabs");
}

async function toastPaths(session) {
  const { page } = session;
  const toasts = page.locator("[data-ui-toasts] [data-ui-toast]");
  const before = await toasts.count();
  await session.focusByKeyboard("#toast-button");
  const answered = page.waitForResponse((response) => new URL(response.url()).pathname === "/toast");
  await page.keyboard.press("Enter");
  const response = await answered;
  if (response.status() !== 200 || !(response.headers()["hx-trigger"] ?? "").includes("ui:toast")) {
    throw new Error(`/toast answered ${response.status()} without the ui:toast trigger`);
  }
  await page.waitForFunction((count) => document.querySelectorAll("[data-ui-toasts] [data-ui-toast]").length > count, before);
  const toast = toasts.last();
  const text = (await toast.locator("[data-ui-toast-message]").textContent()) ?? "";
  if (text !== "Saved to the gallery") session.record("flow", "toast text", `the toast reads "${text}"`);
  if ((await toast.getAttribute("data-tone")) !== "success") session.record("flow", "toast tone", "the toast is not a success toast");
  await session.audit("[data-ui-toasts]");
  await session.screenshot("toast");
  await toast.locator("[data-ui-toast-close]").click();
  if ((await toasts.count()) !== before) session.record("flow", "toast dismiss", "the close button did not remove the toast");
  // A menu item that submits the htmx form shows a toast too.
  await session.focusByKeyboard(MENU_BUTTON);
  await page.keyboard.press("ArrowDown");
  await page.waitForFunction((selector) => document.querySelector(selector).matches(":popover-open"), MENU);
  await page.keyboard.press("End");
  const submitted = page.waitForResponse((response) => new URL(response.url()).pathname === "/toast");
  await page.keyboard.press("Enter");
  await submitted;
  await page.waitForFunction((count) => document.querySelectorAll("[data-ui-toasts] [data-ui-toast]").length > count, before);
}

async function loadingPaths(session) {
  const { page } = session;
  const spinner = "#slow-button svg";
  const shown = () => page.evaluate((selector) => getComputedStyle(document.querySelector(selector)).display !== "none", spinner);
  if (await shown()) session.record("flow", "spinner at rest", "the loading spinner shows before any request");
  await session.focusByKeyboard("#slow-button");
  const answered = page.waitForResponse((response) => new URL(response.url()).pathname === "/slow");
  await page.keyboard.press("Enter");
  await page.waitForFunction(() => document.querySelector("#slow-button").classList.contains("htmx-request"));
  if (!(await shown())) session.record("flow", "spinner during the request", "hx-indicator marked the button but the spinner is hidden");
  if (!(await page.evaluate(() => document.querySelector("#slow-button").disabled))) {
    session.record("flow", "disabled during the request", "hx-disabled-elt did not disable the button");
  }
  await answered;
  await page.waitForFunction(() => !document.querySelector("#slow-button").classList.contains("htmx-request"));
  if (await shown()) session.record("flow", "spinner after the request", "the spinner still shows after the response");
}

async function run(name) {
  const browser = await ENGINES[name].launch();
  const sessions = [];
  try {
    for (const scheme of ["light", "dark"]) {
      const session = new Session(name, scheme);
      sessions.push(session);
      try {
        await session.open(browser);
        await session.step("/: load, axe, Tab walk", async () => {
          await session.goto("/");
          await session.audit();
          await session.tabWalk();
          await session.screenshot("gallery");
        });
        if (scheme === "light") {
          for (const [label, paths] of [["dialog", dialogPaths], ["menu", menuPaths], ["tabs", tabPaths],
            ["toast", toastPaths], ["loading state", loadingPaths]]) {
            await session.step(`${label} with the keyboard`, async () => {
              await session.goto("/");
              await paths(session);
            });
          }
        }
      } catch (error) {
        session.record("flow", String(error.message).split("\n")[0].slice(0, 200), String(error.stack ?? error));
        if (session.page) await session.screenshot("failure");
      } finally {
        await session.close();
      }
    }
    return {
      browser: name, version: browser.version(), steps: sessions.flatMap((session) => session.steps),
      findings: sessions.flatMap((session) => session.findings),
      capture_reports: sessions.reduce((total, session) => total + session.captureReports, 0),
    };
  } finally {
    await browser.close().catch(() => {});
  }
}

await mkdir(OUT, { recursive: true });
const results = [];
for (const name of browsers) results.push(await run(name));
const report = { starter: "ui-components", base_url: base.href, versions: VERSIONS, browsers: results };
await writeFile(join(OUT, "report.json"), JSON.stringify(report, null, 2));
const counts = results.map((result) => result.browser + " " + result.findings.length + " finding(s)");
console.log(`ui-components: ${counts.join(", ")}`);
