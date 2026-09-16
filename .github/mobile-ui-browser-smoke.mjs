// Browser contracts over real rendered HTML supplied on stdin. No npm packages.
// CDN/font/image requests are blocked: these tests cover local UI, not providers.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";

const kind = process.argv[2];
assert(["nexus", "portfolio"].includes(kind), "expected nexus or portfolio");
let html = "";
for await (const chunk of process.stdin) {
  html += chunk;
  assert(Buffer.byteLength(html) <= 2 * 1024 * 1024, "bounded HTML fixture");
}
assert(html.includes("<html"), "rendered HTML is required");
const profile = await mkdtemp(join(tmpdir(), "rullst-mobile-browser-"));
const server = createServer((_request, response) => {
  response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
  response.end(html);
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
const deadline = setTimeout(() => { chrome.kill("SIGKILL"); }, 90_000);
try {
  const endpoint = await new Promise((accept, reject) => {
    let stderr = "";
    const timer = setTimeout(() => reject(new Error(`Chrome startup timeout: ${stderr}`)), 45_000);
    const fail = error => { clearTimeout(timer); reject(error); };
    chrome.once("error", fail);
    chrome.once("exit", code => fail(new Error(`Chrome exited: ${code}`)));
    chrome.stderr.on("data", chunk => {
      stderr = (stderr + chunk).slice(-8192);
      const match = stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);
      if (match) { clearTimeout(timer); accept(match[1]); }
    });
  });
  socket = new WebSocket(endpoint);
  await new Promise((accept, reject) => {
    const timer = setTimeout(() => reject(new Error("CDP handshake timeout")), 15_000);
    socket.addEventListener("open", () => { clearTimeout(timer); accept(); }, { once: true });
    socket.addEventListener("error", () => { clearTimeout(timer); reject(new Error("CDP handshake failed")); }, { once: true });
  });
  let sequence = 0;
  const pending = new Map();
  const events = new Map();
  socket.addEventListener("message", ({ data }) => {
    const message = JSON.parse(data);
    if (message.id) {
      const operation = pending.get(message.id);
      if (!operation) return;
      pending.delete(message.id);
      clearTimeout(operation.timer);
      if (message.error) operation.reject(new Error(JSON.stringify(message.error)));
      else operation.accept(message.result);
    } else events.get(message.method)?.(message.params);
  });
  const call = (method, params = {}, sessionId) => new Promise((accept, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 15_000);
    pending.set(id, { accept, reject, timer });
    socket.send(JSON.stringify({ id, method, params, sessionId }));
  });
  const { targetId } = await call("Target.createTarget", { url: "about:blank" });
  const { sessionId } = await call("Target.attachToTarget", { targetId, flatten: true });
  const send = (method, params) => call(method, params, sessionId);
  const evaluate = async expression => {
    const result = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    assert(!result.exceptionDetails, JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  const errors = [];
  events.set("Runtime.exceptionThrown", ({ exceptionDetails }) => errors.push(exceptionDetails));
  await send("Page.enable");
  await send("Runtime.enable");
  await send("Network.enable");
  await send("Network.setBlockedURLs", { urls: ["https://*"] });
  const settle = () => evaluate("new Promise(resolve => setTimeout(resolve, 250))");
  const resize = async width => {
    await send("Emulation.setDeviceMetricsOverride", { width, height: 844, deviceScaleFactor: 1, mobile: false });
    await settle();
  };
  const navigate = async (scripts = true) => {
    const loaded = new Promise((accept, reject) => {
      const timer = setTimeout(() => reject(new Error("page load timeout")), 15_000);
      events.set("Page.loadEventFired", () => { clearTimeout(timer); accept(); });
    });
    await send("Page.navigate", { url: origin });
    await loaded;
    if (scripts) await settle();
  };
  const key = async (key, code, windowsVirtualKeyCode, modifiers = 0) => {
    await send("Input.dispatchKeyEvent", { type: "keyDown", key, code, windowsVirtualKeyCode, modifiers, ...(key === "Enter" ? {text: "\r"} : {}) });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key, code, windowsVirtualKeyCode, modifiers });
  };
  const click = async selector => {
    const point = await evaluate(`(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) throw new Error('Missing control: ' + ${JSON.stringify(selector)});
      const r = el.getBoundingClientRect();
      return {x:r.x + r.width / 2, y:r.y + r.height / 2};
    })()`);
    await send("Input.dispatchMouseEvent", { type: "mousePressed", button: "left", clickCount: 1, ...point });
    await send("Input.dispatchMouseEvent", { type: "mouseReleased", button: "left", clickCount: 1, ...point });
    await settle();
  };
  await resize(390);
  await navigate();
  if (kind === "nexus") {
    const closed = async () => {
      assert.equal(await evaluate("document.querySelector('.nexus-topbar-toggle').getAttribute('aria-expanded')"), "false");
      assert(await evaluate("document.querySelector('#nexus-sidebar').inert"), "closed offscreen links must not remain keyboard-focusable");
      assert.equal(await evaluate("document.querySelector('.nexus-main').inert"), false);
    };
    const opened = async () => {
      assert.equal(await evaluate("document.querySelector('.nexus-topbar-toggle').getAttribute('aria-expanded')"), "true");
      assert(await evaluate("document.querySelector('.nexus-main').inert"));
      assert.equal(await evaluate("document.activeElement.id"), "nexus-sidebar-close");
    };
    await closed();
    await click(".nexus-topbar-toggle");
    await opened();
    await click("#nexus-sidebar-close");
    await closed();
    assert(await evaluate("document.activeElement.classList.contains('nexus-topbar-toggle')"));
    await key("Enter", "Enter", 13);
    await settle();
    await opened();
    await key("Tab", "Tab", 9, 8);
    assert(await evaluate("document.activeElement.classList.contains('nexus-nav-home')"), "Shift+Tab wraps inside drawer");
    await key("Tab", "Tab", 9);
    assert.equal(await evaluate("document.activeElement.id"), "nexus-sidebar-close");
    await key("Escape", "Escape", 27);
    await closed();
    await click(".nexus-topbar-toggle");
    // A real touch outside the 240px drawer hits the backdrop, not the main page.
    await send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: 360, y: 400 }] });
    await send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
    await settle();
    await closed();
    await click(".nexus-topbar-toggle");
    await evaluate("document.querySelector('#nexus-sidebar').addEventListener('click', e => e.preventDefault()); document.querySelector('.nexus-nav-link').click()");
    await closed();
    await click(".nexus-topbar-toggle");
    await resize(1200);
    assert.equal(await evaluate("document.querySelector('.nexus-main').inert || document.querySelector('#nexus-sidebar').inert"), false);
    assert.equal(await evaluate("getComputedStyle(document.querySelector('#nexus-sidebar-backdrop')).display"), "none");
    await resize(900);
    await closed();
    await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
    assert.equal(await evaluate("getComputedStyle(document.querySelector('#nexus-sidebar')).transitionDuration"), "0s");
    // Disabled JavaScript must leave ordinary navigation visible, not trapped.
    await send("Emulation.setScriptExecutionDisabled", { value: true });
    await navigate(false);
    await send("Emulation.setScriptExecutionDisabled", { value: false });
    assert.equal(await evaluate("getComputedStyle(document.querySelector('#nexus-sidebar')).position"), "static");
    assert.equal(await evaluate("getComputedStyle(document.querySelector('.nexus-topbar-toggle')).display"), "none");
  } else {
    for (const width of [320, 360, 390, 420, 640, 900, 901, 1200, 1440]) {
      await resize(width);
      assert(await evaluate("document.documentElement.scrollWidth <= innerWidth"), `page overflow at ${width}`);
      const overflow = await evaluate(`Array.from(document.querySelectorAll('.layout, .sidebar, .content, .project-card, .contact-item a, .cms-btn, h1, h2, .tag'), el => {
        const r = el.getBoundingClientRect();
        return {name:el.className || el.tagName, left:r.left, right:r.right, clipped:el.scrollWidth > el.clientWidth + 1};
      }).filter(r => r.left < -1 || r.right > innerWidth + 1 || r.clipped)`);
      assert.deepEqual(overflow, [], `content overflow/clipping at ${width}`);
      assert.equal(await evaluate("getComputedStyle(document.querySelector('.layout')).flexDirection"), width <= 900 ? "column" : "row");
      assert.equal(await evaluate("getComputedStyle(document.querySelector('.sidebar')).position"), width <= 900 ? "static" : "sticky");
    }
    await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
    assert.equal(await evaluate("document.getAnimations().length"), 0, "reduced-motion preference");
  }
  if (process.env.RULLST_UI_SCREENSHOT_DIR) {
    await resize(390);
    await navigate();
    if (kind === "nexus") await click(".nexus-topbar-toggle");
    const { data } = await send("Page.captureScreenshot", { format: "png" });
    await writeFile(join(process.env.RULLST_UI_SCREENSHOT_DIR, `${kind}-390.png`), Buffer.from(data, "base64"));
  }
  assert.deepEqual(errors, [], "JavaScript runtime errors");
  console.log(`PASS: ${kind} real-browser mobile contract`);
  await call("Browser.close");
} finally {
  clearTimeout(deadline);
  socket?.close();
  if (chrome.exitCode === null) {
    chrome.kill("SIGTERM");
    await Promise.race([once(chrome, "exit"), new Promise(resolve => setTimeout(resolve, 3000))]);
    if (chrome.exitCode === null && chrome.signalCode === null) chrome.kill("SIGKILL");
  }
  server.closeAllConnections();
  await new Promise(resolve => server.close(resolve));
  // Only this test's uniquely created browser profile is removed.
  await rm(profile, { recursive: true, force: true, maxRetries: 3 });
}
