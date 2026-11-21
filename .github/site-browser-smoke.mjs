// Real Chromium checks for the rullst.win landing and privacy pages, served at
// the site root as Rullst/Rullst.github.io publishes them. No npm packages or
// external requests. Run after `mdbook build docs`; optional:
// --screenshots /absolute/output/directory, and --organization-site DIRECTORY
// to serve the pages from assemble-organization-site.py output instead.
import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { once } from "node:events";
import { createServer } from "node:http";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, extname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const profile = await mkdtemp(join(tmpdir(), "rullst-site-chromium-"));
// The comparison pages with benchmark rows, assembled from the test fixture
// (its numbers are not measurements), dated today so that the assembler's
// default age limit applies, and served under /_rendered/.
const rendered = await mkdtemp(join(tmpdir(), "rullst-site-rendered-"));
const fixture = JSON.parse(await readFile(join(root, ".github/fixtures/site-benchmarks.json"), "utf8"));
fixture.measured_on = new Date().toISOString().slice(0, 10);
await writeFile(join(rendered, "benchmarks.json"), JSON.stringify(fixture));
// An absolute interpreter path, not a PATH lookup (Ubuntu and macOS both have it).
execFileSync("/usr/bin/python3", [join(root, ".github/assemble-organization-site.py"), "--out", join(rendered, "site"), "--benchmarks"],
  { cwd: rendered, stdio: ["ignore", "ignore", "inherit"] });
const outputIndex = process.argv.indexOf("--screenshots");
const output = outputIndex < 0 ? null : process.argv[outputIndex + 1];
const organizationIndex = process.argv.indexOf("--organization-site");
const organization = organizationIndex < 0 ? null : resolve(process.argv[organizationIndex + 1]);
const landingPath = "/";
const assetPaths = ["/assets/site.css", "/assets/site.js"];
const oldAssetReads = new Map(assetPaths.map(path => [path, 0]));
if (outputIndex >= 0) assert(output, "--screenshots requires a directory");
if (output) await mkdir(output, { recursive: true });
const types = { ".html": "text/html", ".css": "text/css", ".js": "text/javascript", ".png": "image/png", ".svg": "image/svg+xml", ".woff2": "font/woff2" };
const server = createServer(async (request, response) => {
  try {
    if (request.method !== "GET") { response.writeHead(405).end(); return; }
    const url = new URL(request.url, "http://localhost");
    const path = decodeURIComponent(url.pathname);
    if (path === "/_cache-probe.html") {
      response.writeHead(200, { "Content-Type": "text/html", "Cache-Control": "no-store" });
      response.end(`<!doctype html><html lang="en"><head><link rel="icon" href="data:,"><link rel="stylesheet" href="${assetPaths[0]}"><script src="${assetPaths[1]}"></script></head><body><div class="engine-module">Old presentation</div></body></html>`);
      return;
    }
    if (oldAssetReads.has(path) && !url.search) {
      // Seed a real four-hour browser cache with the pre-deployment URLs.
      // Only the content-versioned URLs may load the current presentation.
      oldAssetReads.set(path, oldAssetReads.get(path) + 1);
      response.writeHead(200, { "Content-Type": types[extname(path)], "Cache-Control": "public, max-age=14400" });
      response.end(path.endsWith(".css") ? ".engine-module { position: static; border: 0; }" : "window.__oldSiteScript = true;");
      return;
    }
    let base = root;
    let relative;
    const rootSite = { "/": ["index.html", "docs/home_template.html"], "/privacy.html": ["privacy.html", "docs/privacy_template.html"],
      "/pt-br/": ["pt-br/index.html", "docs/home_template.pt-BR.html"], "/pt-br/privacy.html": ["pt-br/privacy.html", "docs/privacy_template.pt-BR.html"],
      "/compare/": ["compare/index.html", "docs/compare_template.html"], "/pt-br/comparar/": ["pt-br/comparar/index.html", "docs/compare_template.pt-BR.html"],
      "/assets/site.css": ["assets/site.css", "docs/site.css"], "/assets/site.js": ["assets/site.js", "docs/site.js"],
      "/assets/flags/us.svg": ["assets/flags/us.svg", "docs/flags/us.svg"], "/assets/flags/br.svg": ["assets/flags/br.svg", "docs/flags/br.svg"] };
    if (rootSite[path]) {
      if (organization) base = organization;
      relative = rootSite[path][organization ? 0 : 1];
    }
    else if (path === "/_rendered/compare/" || path === "/_rendered/pt-br/comparar/") { base = join(rendered, "site"); relative = path.slice(11) + "index.html"; }
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
// Hosted runners can spend more than 15 seconds starting Chromium while other
// release jobs contend for CPU and I/O. Keep navigation/CDP assertions strict,
// but give the one-time browser bootstrap a bounded, runner-safe window.
const chromeStartupTimeoutMs = 45_000;
let socket;
try {
  const endpoint = await new Promise((accept, reject) => {
    let stderr = "";
    const timeout = setTimeout(
      () => reject(new Error(`Chrome startup timed out after ${chromeStartupTimeoutMs}ms: ${stderr}`)),
      chromeStartupTimeoutMs,
    );
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
  const layout = async (width, height, route = landingPath) => {
    await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
    await navigate(true, route);
    const horizontal = await evaluate(`(() => ({
      viewport: innerWidth,
      scrollWidth: document.documentElement.scrollWidth,
      overflowers: [...document.querySelectorAll("body *")]
        .map(element => ({
          element: element.tagName.toLowerCase() + (element.id ? "#" + element.id : "") +
            [...element.classList].map(name => "." + name).join(""),
          left: Math.round(element.getBoundingClientRect().left * 10) / 10,
          right: Math.round(element.getBoundingClientRect().right * 10) / 10,
        }))
        .filter(rect => rect.left < -0.5 || rect.right > innerWidth + 0.5)
        .slice(0, 8),
    }))()`);
    assert(
      horizontal.scrollWidth <= horizontal.viewport,
      `Horizontal overflow at ${width}px: ${JSON.stringify(horizontal)}`,
    );
    assert.equal(await evaluate("document.querySelectorAll('h1').length"), 1);
    assert.equal(await evaluate("document.querySelectorAll('.social-links a').length"), 17);
    assert.deepEqual(await evaluate("[...document.querySelectorAll('[data-demo-link]')].map(link => link.href)"), [
      "https://rullst-showcase.redpond-24d9228d.eastus.azurecontainerapps.io/",
      "https://rullst-lms.redpond-24d9228d.eastus.azurecontainerapps.io/",
      "https://rullst-portfolio.redpond-24d9228d.eastus.azurecontainerapps.io/",
      "https://saas.rullst.win/",
    ], "Demo buttons must retain the independently hosted application URLs");
    assert.equal(await evaluate("document.querySelectorAll('#integrations [data-integration-level] li').length"), 16, "Integrations must list every README entry");
    assert(await evaluate("[...document.querySelectorAll('#academy a')].some(link => link.href.startsWith('https://academy.rullst.win/'))"), "Academy section must link the Academy");
    const language = await evaluate(`(() => {
      const switcher = document.querySelector('.lang-switch');
      const box = switcher.getBoundingClientRect();
      return {
        lang: document.documentElement.lang,
        current: [...switcher.querySelectorAll('[aria-current=page]')].map(link => link.hreflang),
        labels: [...switcher.querySelectorAll('a')].map(link => link.textContent.trim()),
        flags: [...switcher.querySelectorAll('img')].every(image => image.complete && image.naturalWidth > 0),
        visible: getComputedStyle(switcher).display !== 'none' && box.left >= 0 && box.right <= innerWidth && box.height > 0,
      };
    })()`);
    const expectedLanguage = route === "/pt-br/" ? "pt-BR" : "en";
    assert.deepEqual(language, { lang: expectedLanguage, current: [expectedLanguage], labels: ["English", "Português"], flags: true, visible: true },
      `Language switcher at ${width}px on ${route}`);
    assert(await evaluate("[...document.images].filter(i => i.loading !== 'lazy').every(i => i.complete && i.naturalWidth > 0)"), "Hero image failed");
    assert(await evaluate(`document.body.innerText.includes(${JSON.stringify(route === "/pt-br/" ? "Pré-lançamento do Rullst v13" : "Rullst v13 prerelease")})`));
    assert.equal(await evaluate("window.__oldSiteScript === true"), false, "A cached old script must not run with new HTML");
    assert.deepEqual(await evaluate(`Array.from(document.querySelectorAll('.engine-module'), element => {
      const style = getComputedStyle(element);
      return { position: style.position, border: style.borderTopWidth, background: style.backgroundImage !== 'none' };
    })`), Array.from({length: 3}, () => ({position: "absolute", border: "1px", background: true})), "All three engine cards must load their actual layout and box styling");
    assert(await evaluate(`document.querySelector('#examples h2').textContent.includes(${JSON.stringify(route === "/pt-br/" ? "Não é só um framework." : "Not just a framework.")})`));
    if (output) {
      // Capture the settled identity, not an arbitrary color mid-intro.
      await evaluate("new Promise(resolve => setTimeout(resolve, 5100))");
      // Stopping the bounded intro intentionally cancels CSS animations.
      // Cancellation is not a page error; an unbounded animation still times out.
      await evaluate("Promise.allSettled(document.getAnimations().map(animation => animation.finished))");
      const { data } = await send("Page.captureScreenshot", { format: "png" });
      await writeFile(join(output, `rullst-site-${expectedLanguage}-${width}.png`), Buffer.from(data, "base64"));
      await evaluate("document.querySelector('#examples').scrollIntoView({behavior:'instant'}); new Promise(resolve => setTimeout(resolve, 1200))");
      const demoScreenshot = await send("Page.captureScreenshot", { format: "png" });
      await writeFile(join(output, `rullst-examples-${expectedLanguage}-${width}.png`), Buffer.from(demoScreenshot.data, "base64"));
      // Scrolling an element into view changes Chromium's sequential focus
      // navigation starting point. Reload so the keyboard test starts fresh.
      await navigate(true, route);
    }
  };
  await navigate(true, "/_cache-probe.html");
  assert(await evaluate("window.__oldSiteScript === true && getComputedStyle(document.querySelector('.engine-module')).borderTopWidth === '0px'"));
  await navigate(true, "/_cache-probe.html");
  assert.deepEqual([...oldAssetReads.values()], [1, 1], "The old CSS and script must actually be reused from browser cache");
  await layout(1440, 1100);
  assert(await evaluate("document.querySelector('.hero').classList.contains('is-visible')"), "Hero reveal must initialize");
  assert.equal(await evaluate("document.querySelector('link[rel=canonical]').href"), `https://rullst.win${landingPath}`, "Each entry point needs its own canonical URL");
  await evaluate("if (document.documentElement.classList.contains('intro-playing')) document.querySelector('[data-intro-toggle]').click(); document.querySelector('[data-intro-toggle]').click()");
  assert(await evaluate("document.querySelector('[data-intro-toggle]').getAttribute('aria-pressed') === 'true'"), "Intro must replay on request");
  const startPosition = await evaluate("getComputedStyle(document.querySelector('.hero-wordmark')).backgroundPosition");
  await evaluate("new Promise(resolve => setTimeout(resolve, 1000))");
  assert.notEqual(await evaluate("getComputedStyle(document.querySelector('.hero-wordmark')).backgroundPosition"), startPosition, "Wordmark colors must actually animate");
  await evaluate("document.querySelector('[data-intro-toggle]').click()");
  assert(await evaluate("!document.documentElement.classList.contains('intro-playing') && document.querySelector('[data-intro-toggle]').textContent === 'Replay intro'"), "Intro must stop immediately");
  await evaluate("document.querySelector('[data-intro-toggle]').click(); new Promise(resolve => setTimeout(resolve, 5100))");
  assert(await evaluate("!document.documentElement.classList.contains('intro-playing') && document.querySelector('[data-intro-toggle]').getAttribute('aria-pressed') === 'false'"), "Intro must stop automatically instead of looping indefinitely");
  await send("Input.dispatchKeyEvent", { type: "keyDown", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9 });
  await send("Input.dispatchKeyEvent", { type: "keyUp", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9 });
  assert(await evaluate("document.activeElement.classList.contains('skip-link')"), "First keyboard target must skip navigation");
  await evaluate("new Promise(resolve => { document.querySelector('#ecosystem').scrollIntoView({ behavior: 'instant', block: 'center' }); setTimeout(resolve, 120); })");
  assert(await evaluate("document.querySelector('#ecosystem').classList.contains('is-visible')"), "Scroll reveal must initialize");
  assert(await evaluate("!document.querySelector('[data-academy-wave]').hidden"), "Academy greeting control must appear with motion allowed");
  await evaluate("document.querySelector('.academy-companion').scrollIntoView({ behavior: 'instant', block: 'center' }); new Promise(resolve => setTimeout(resolve, 200))");
  if (!await evaluate("document.querySelector('#academy').classList.contains('academy-is-waving')")) await evaluate("document.querySelector('[data-academy-wave]').click()");
  assert(await evaluate("document.querySelector('[data-academy-wave]').getAttribute('aria-pressed') === 'true' && document.getAnimations().some(animation => animation.animationName === 'academy-claw-wave')"), "Academy mascot must wave");
  await evaluate("document.querySelector('[data-academy-wave]').click()");
  assert(await evaluate("!document.querySelector('#academy').classList.contains('academy-is-waving') && document.querySelector('[data-academy-wave]').getAttribute('aria-pressed') === 'false'"), "Academy greeting must stop on request");
  await send("Browser.grantPermissions", { origin, permissions: ["clipboardReadWrite", "clipboardSanitizedWrite"] });
  await evaluate("document.querySelector('[data-copy-command=install-command]').click()");
  assert.match(await evaluate("navigator.clipboard.readText()"), /^cargo install cargo-rullst --version =\S+ --locked$/);
  await evaluate("document.querySelector('[data-copy-command=new-command]').click()");
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
  await evaluate("if (!document.documentElement.classList.contains('intro-playing')) document.querySelector('[data-intro-toggle]').click()");
  await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
  await evaluate("new Promise(resolve => setTimeout(resolve, 100))");
  assert(await evaluate("document.querySelector('[data-intro-toggle]').hidden && !document.documentElement.classList.contains('intro-playing')"), "Changing reduced-motion preference must stop an active intro");
  assert(await evaluate("document.querySelector('[data-academy-wave]').hidden && !document.querySelector('#academy').classList.contains('academy-is-waving')"), "Reduced motion must hide and stop the Academy greeting");
  await navigate();
  assert.equal(await evaluate("document.getAnimations().length"), 0, "Reduced motion must remove entrance animation");
  assert(await evaluate("[...document.querySelectorAll('[data-reveal]')].every(element => element.classList.contains('is-visible'))"), "Reduced motion must reveal content immediately");
  await send("Emulation.setScriptExecutionDisabled", { value: true });
  await navigate(false);
  const snapshot = await send("DOMSnapshot.captureSnapshot", { computedStyles: ["display"] });
  assert(snapshot.strings.includes("nav-links"), "No-JS navigation remains in document");
  await send("Emulation.setScriptExecutionDisabled", { value: false });
  assert(await evaluate("!document.documentElement.classList.contains('js') && getComputedStyle(document.querySelector('[data-navigation]')).display !== 'none'"), "Mobile navigation must work without JS");
  assert(await evaluate("document.querySelector('[data-intro-toggle]').hidden && document.querySelector('[data-academy-wave]').hidden"), "Nonfunctional motion controls must stay hidden without JS");
  assert.equal(await evaluate("document.cookie"), "");
  assert.equal(await evaluate("localStorage.length + sessionStorage.length"), 0);
  await send("Emulation.setEmulatedMedia", { features: [] });
  for (const [width, height] of [[1440, 1100], [390, 844], [320, 740]]) await layout(width, height, "/pt-br/");
  await evaluate("document.querySelector('[data-copy-command=new-command]').click()");
  assert.equal(await evaluate("document.querySelector('[data-copy-status]').textContent"), "Comando copiado.", "Portuguese page must announce copies in Portuguese");
  assert.equal(await evaluate("document.querySelector('[data-intro-toggle]').textContent"), await evaluate("document.documentElement.classList.contains('intro-playing') ? 'Parar introdução' : 'Repetir introdução'"));
  await navigate(true, "/pt-br/privacy.html");
  assert(await evaluate("document.documentElement.lang === 'pt-BR' && document.title === 'Aviso de privacidade do site — Rullst' && document.querySelectorAll('main section h2').length === 9"), "Portuguese privacy page must render the full notice");
  assert.equal(await evaluate("document.querySelector('link[rel=canonical]').href"), "https://rullst.win/pt-br/privacy.html");
  assert.equal(await evaluate("document.querySelector('.lang-switch [aria-current=page]').hreflang"), "pt-BR");
  for (const route of ["/compare/", "/pt-br/comparar/"]) {
    for (const [width, height] of [[1440, 1100], [390, 844], [320, 740]]) {
      await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
      await navigate(true, route);
      const page = await evaluate(`(() => ({
        overflow: document.documentElement.scrollWidth > innerWidth,
        topics: document.querySelectorAll('.compare-topic').length,
        columns: [...document.querySelectorAll('.compare-topic')].every(topic => topic.querySelectorAll('dt').length === 3),
        uiFrameworks: document.querySelectorAll('#ui-frameworks .compare-checked code').length,
        benchmarks: document.querySelectorAll('.benchmark-pending, .benchmark-table').length,
        current: document.querySelector('.lang-switch [aria-current=page]').getAttribute('href'),
        flags: [...document.querySelectorAll('.lang-switch img')].every(image => image.complete && image.naturalWidth > 0),
      }))()`);
      assert.deepEqual(page, { overflow: false, topics: 11, columns: true, uiFrameworks: 2, benchmarks: 1, current: route, flags: true }, `Comparison page ${route} at ${width}px`);
    }
  }
  for (const [route, minimal] of [["/_rendered/compare/", "Rullst (minimal)"], ["/_rendered/pt-br/comparar/", "Rullst (mínimo)"]]) {
    for (const [width, height] of [[1440, 1100], [390, 844], [320, 740]]) {
      await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
      await navigate(true, route);
      const page = await evaluate(`(() => ({
        overflow: document.documentElement.scrollWidth > innerWidth,
        howto: document.querySelectorAll('.benchmark-howto li').length,
        pending: document.querySelectorAll('.benchmark-pending').length,
        tables: [...document.querySelectorAll('.benchmark-table')].map(region => region.getBoundingClientRect().right <= innerWidth),
        rows: [...document.querySelectorAll('.benchmark-table')[0].querySelectorAll('tbody th[scope=row]')].map(title => title.firstChild.textContent.trim()),
        legend: document.querySelectorAll('.benchmark-legend li').length,
        reading: document.querySelectorAll('.benchmark-reading code').length,
      }))()`);
      assert.deepEqual(page, { overflow: false, howto: 6, pending: 0, tables: [true, true], rows: ["Actix Web", "Topcoat", "Loco", "Rullst", "Gin", minimal], legend: 5, reading: 1 }, `Rendered comparison ${route} at ${width}px`);
      if (output) {
        const clip = await evaluate(`(() => { const box = document.querySelector('#benchmarks').getBoundingClientRect(); return { x: 0, y: box.top + scrollY, width: innerWidth, height: box.height, scale: 1 }; })()`);
        const { data } = await send("Page.captureScreenshot", { format: "png", clip, captureBeyondViewport: true });
        await writeFile(join(output, `rullst-compare-${route.includes("/pt-br/") ? "pt-BR" : "en"}-${width}.png`), Buffer.from(data, "base64"));
      }
    }
  }
  await navigate(true, "/privacy.html");
  assert(await evaluate("document.title === 'Website privacy notice — Rullst' && document.querySelectorAll('main section h2').length === 9"), "Standalone privacy page must render the full notice");
  assert.equal(await evaluate("document.querySelector('link[rel=canonical]').href"), "https://rullst.win/privacy.html");
  assert.equal(await evaluate("getComputedStyle(document.body).margin"), "0px", "Privacy must also load the current stylesheet despite stale cached CSS");
  assert.equal(await evaluate("getComputedStyle(document.querySelector('.privacy-page')).maxWidth"), "950px", "Privacy page layout styles must load");
  assert.deepEqual([...oldAssetReads.values()], [1, 1], "Current pages must not request the old unversioned assets again");
  assert([...requests].every(url => url.startsWith(origin + "/")), `External resource requests: ${[...requests].filter(url => !url.startsWith(origin + "/"))}`);
  assert.deepEqual(failures, [], "Browser errors, CSP failures or broken resources");
  console.log(`PASS (${organization ? "assembled root site" : "sources"}): stale-cache upgrade, engine card styling, desktop/390px/320px, keyboard/mobile menu, animated intro/replay/stop, Academy greeting/stop, integrations, canonical URLs, English and Brazilian Portuguese pages with flag language switcher, comparison pages with fixture benchmark rows, clipboard success/denial, privacy summary and full notice, reduced motion, no-JS navigation, no storage or external landing requests.`);
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
  await rm(rendered, { recursive: true, force: true, maxRetries: 3 });
}
