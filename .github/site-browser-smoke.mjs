// Real Chromium checks for the landing page. No npm packages or external requests.
// Run after `mdbook build docs`; optional: --screenshots /absolute/output/directory.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { createServer } from "node:http";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, extname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const profile = await mkdtemp(join(tmpdir(), "rullst-site-chromium-"));
const outputIndex = process.argv.indexOf("--screenshots");
const output = outputIndex < 0 ? null : process.argv[outputIndex + 1];
const organizationIndex = process.argv.indexOf("--organization-site");
const organization = organizationIndex < 0 ? null : resolve(process.argv[organizationIndex + 1]);
const landingPath = organization ? "/" : "/Rullst/";
if (outputIndex >= 0) assert(output, "--screenshots requires a directory");
if (output) await mkdir(output, { recursive: true });
const types = { ".html": "text/html", ".css": "text/css", ".js": "text/javascript", ".png": "image/png", ".svg": "image/svg+xml", ".woff2": "font/woff2" };
const server = createServer(async (request, response) => {
  try {
    if (request.method !== "GET") { response.writeHead(405).end(); return; }
    const path = decodeURIComponent(new URL(request.url, "http://localhost").pathname);
    let base = root;
    let relative;
    if (organization && ["/", "/privacy.html", "/src/style.css", "/src/main.js"].includes(path)) {
      base = organization;
      relative = path === "/" ? "index.html" : path.slice(1);
    }
    else if (path === "/Rullst/") relative = "docs/home_template.html";
    else if (path === "/Rullst/assets/site.css") relative = "docs/site.css";
    else if (path === "/Rullst/assets/site.js") relative = "docs/site.js";
    else if (path === "/Rullst/Rullst.png") relative = "docs/Rullst.png";
    else if (path.startsWith("/Rullst/book/")) { base = join(root, "docs/book"); relative = path.slice(13) || "index.html"; }
    else if (path.startsWith("/Rullst/images/")) { base = join(root, "images"); relative = path.slice(15); }
    else { response.writeHead(404).end(); return; }
    const file = resolve(base, relative);
    if (!file.startsWith(base + sep)) { response.writeHead(403).end(); return; }
    const bytes = await readFile(file);
    response.writeHead(200, { "Content-Type": types[extname(file)] ?? "application/octet-stream", "Cache-Control": "no-store" });
    response.end(bytes);
  } catch { response.writeHead(404).end(); }
});
server.listen(0, "127.0.0.1");
await once(server, "listening");
const origin = `http://127.0.0.1:${server.address().port}`;
const chrome = spawn(process.env.CHROME_BIN || "google-chrome", [
  "--headless=new", "--no-first-run", "--no-default-browser-check",
  "--disable-background-networking", "--disable-component-update",
  "--disable-dev-shm-usage", "--remote-debugging-port=0",
  `--user-data-dir=${profile}`, "about:blank",
], { stdio: ["ignore", "ignore", "pipe"] });
let socket;
try {
  const endpoint = await new Promise((accept, reject) => {
    let stderr = "";
    const timeout = setTimeout(() => reject(new Error(`Chrome startup timed out: ${stderr}`)), 15000);
    chrome.once("error", (error) => { clearTimeout(timeout); reject(error); });
    chrome.once("exit", (code) => { clearTimeout(timeout); reject(new Error(`Chrome exited ${code}: ${stderr}`)); });
    chrome.stderr.on("data", (chunk) => {
      stderr = (stderr + chunk).slice(-8192);
      const match = stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);
      if (match) { clearTimeout(timeout); accept(match[1]); }
    });
  });
  socket = new WebSocket(endpoint);
  await new Promise((accept, reject) => {
    const timeout = setTimeout(() => reject(new Error("DevTools WebSocket handshake timed out")), 15000);
    socket.addEventListener("open", () => { clearTimeout(timeout); accept(); }, { once: true });
    socket.addEventListener("error", () => { clearTimeout(timeout); reject(new Error("DevTools WebSocket handshake failed")); }, { once: true });
    socket.addEventListener("close", () => { clearTimeout(timeout); reject(new Error("DevTools WebSocket closed during handshake")); }, { once: true });
  });
  let sequence = 0;
  const pending = new Map();
  const listeners = new Map();
  socket.addEventListener("message", ({ data }) => {
    const message = JSON.parse(data);
    if (message.id) {
      const callback = pending.get(message.id);
      if (!callback) return;
      pending.delete(message.id);
      clearTimeout(callback.timeout);
      if (message.error) callback.reject(new Error(JSON.stringify(message.error)));
      else callback.accept(message.result);
    } else {
      for (const listener of listeners.get(message.method) ?? []) listener(message.params);
    }
  });
  const call = (method, params = {}, sessionId) => new Promise((accept, reject) => {
    const id = ++sequence;
    const timeout = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 15000);
    pending.set(id, { accept, reject, timeout });
    socket.send(JSON.stringify({ id, method, params, sessionId }));
  });
  const { targetId } = await call("Target.createTarget", { url: "about:blank" });
  const { sessionId } = await call("Target.attachToTarget", { targetId, flatten: true });
  const send = (method, params) => call(method, params, sessionId);
  const evaluate = async (expression) => {
    const result = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    assert(!result.exceptionDetails, JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  const requests = new Set();
  const failures = [];
  listeners.set("Network.requestWillBeSent", [({ request }) => requests.add(request.url)]);
  listeners.set("Network.responseReceived", [({ response }) => { if (response.status >= 400) failures.push(`${response.status}: ${response.url}`); }]);
  listeners.set("Runtime.exceptionThrown", [({ exceptionDetails }) => failures.push(JSON.stringify(exceptionDetails))]);
  listeners.set("Log.entryAdded", [({ entry }) => { if (entry.level === "error") failures.push(entry.text); }]);
  await send("Page.enable");
  await send("Runtime.enable");
  await send("Network.enable");
  await send("Log.enable");
  const navigate = async (scriptsEnabled = true, route = landingPath) => {
    const loaded = new Promise((accept, reject) => {
      const timeout = setTimeout(() => reject(new Error("Page load timeout")), 15000);
      listeners.set("Page.loadEventFired", [() => { clearTimeout(timeout); listeners.delete("Page.loadEventFired"); accept(); }]);
    });
    await send("Page.navigate", { url: `${origin}${route}` });
    await loaded;
    if (scriptsEnabled) await evaluate("document.fonts.ready.then(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))))");
  };
  const layout = async (width, height) => {
    await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
    await navigate();
    assert(await evaluate("document.documentElement.scrollWidth <= innerWidth"), `Horizontal overflow at ${width}px`);
    assert.equal(await evaluate("document.querySelectorAll('h1').length"), 1);
    assert.equal(await evaluate("document.querySelectorAll('.social-links a').length"), 13);
    assert(await evaluate("[...document.images].filter(i => i.loading !== 'lazy').every(i => i.complete && i.naturalWidth > 0)"), "Hero image failed");
    assert(await evaluate("document.body.innerText.includes('NO-GO for production')"));
    if (output) {
      // Wait for the finite entrance animation before recording the visual.
      await evaluate("Promise.all(document.getAnimations().map(animation => animation.finished))");
      const { data } = await send("Page.captureScreenshot", { format: "png" });
      await writeFile(join(output, `rullst-site-${width}.png`), Buffer.from(data, "base64"));
    }
  };
  await layout(1440, 1100);
  await send("Input.dispatchKeyEvent", { type: "keyDown", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9 });
  await send("Input.dispatchKeyEvent", { type: "keyUp", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9 });
  assert(await evaluate("document.activeElement.classList.contains('skip-link')"), "First keyboard target must skip navigation");
  await send("Browser.grantPermissions", { origin, permissions: ["clipboardReadWrite", "clipboardSanitizedWrite"] });
  await evaluate("document.querySelector('[data-copy-command]').click()");
  assert.equal(await evaluate("navigator.clipboard.readText()"), "cargo rullst new my_app");
  await evaluate("Object.defineProperty(navigator, 'clipboard', {configurable:true,value:{writeText:()=>Promise.reject(new Error('denied'))}}); document.querySelector('[data-copy-command]').click()");
  assert(await evaluate("document.querySelector('[data-copy-status]').textContent.includes('Clipboard unavailable')"), "Denied clipboard must have accessible fallback");
  await layout(390, 844);
  assert(await evaluate("getComputedStyle(document.querySelector('[data-navigation]')).display === 'none'"));
  await evaluate("document.querySelector('[data-nav-toggle]').click()");
  assert(await evaluate("document.querySelector('[data-nav-toggle]').getAttribute('aria-expanded') === 'true'"));
  await send("Input.dispatchKeyEvent", { type: "keyDown", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
  assert(await evaluate("document.querySelector('[data-nav-toggle]').getAttribute('aria-expanded') === 'false' && document.activeElement.hasAttribute('data-nav-toggle')"));
  await evaluate("document.querySelector('[data-nav-toggle]').click(); document.querySelector('[data-navigation] a').click()");
  assert(await evaluate("document.querySelector('[data-nav-toggle]').getAttribute('aria-expanded') === 'false'"));
  await evaluate("document.querySelector('#privacy summary').click()");
  assert(await evaluate("document.querySelector('#privacy details').open"));
  await layout(320, 740);
  await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
  await navigate();
  assert.equal(await evaluate("document.getAnimations().length"), 0, "Reduced motion must remove entrance animation");
  await send("Emulation.setScriptExecutionDisabled", { value: true });
  await navigate(false);
  const snapshot = await send("DOMSnapshot.captureSnapshot", { computedStyles: ["display"] });
  assert(snapshot.strings.includes("nav-links"), "No-JS navigation remains in document");
  await send("Emulation.setScriptExecutionDisabled", { value: false });
  assert(await evaluate("!document.documentElement.classList.contains('js') && getComputedStyle(document.querySelector('[data-navigation]')).display !== 'none'"), "Mobile navigation must work without JS");
  assert.equal(await evaluate("document.cookie"), "");
  assert.equal(await evaluate("localStorage.length + sessionStorage.length"), 0);
  if (organization) {
    await navigate(true, "/privacy.html");
    assert(await evaluate("document.title === 'Website privacy notice — Rullst' && document.querySelector('#privacy details').open"), "Standalone privacy page must render with expanded notice");
  }
  assert([...requests].every(url => url.startsWith(origin + "/")), `External resource requests: ${[...requests].filter(url => !url.startsWith(origin + "/"))}`);
  assert.deepEqual(failures, [], "Browser errors, CSP failures or broken resources");
  console.log("PASS: desktop/390px/320px, keyboard/mobile menu, clipboard success/denial, privacy, reduced motion, no-JS navigation, no storage or external landing requests.");
  await call("Browser.close");
} finally {
  socket?.close();
  if (chrome.exitCode === null) {
    chrome.kill("SIGTERM");
    await Promise.race([once(chrome, "exit"), new Promise(resolve => setTimeout(resolve, 3000))]);
    if (chrome.exitCode === null && chrome.signalCode === null) chrome.kill("SIGKILL");
  }
  server.closeAllConnections();
  await new Promise(resolve => server.close(resolve));
  // This unique mkdtemp directory belongs exclusively to this test process.
  await rm(profile, { recursive: true, force: true, maxRetries: 3 });
}
