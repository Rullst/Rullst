// Run with: node cargo-rullst/src/generators/pwa/assets/pwa_tests.cjs
// Executes the generated service worker and registration script against
// in-memory browser APIs; no browser, network or third-party package.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const ORIGIN = 'https://app.example';
const workerSource = fs.readFileSync(path.join(__dirname, 'sw.js'), 'utf8');
const registrationSource = fs.readFileSync(path.join(__dirname, 'pwa.js'), 'utf8');

function response({ status = 200, type = 'basic', headers = {}, body = '' } = {}) {
  const lower = Object.fromEntries(Object.entries(headers).map(([k, v]) => [k.toLowerCase(), v]));
  return {
    status,
    type,
    body,
    headers: { get: (name) => lower[name.toLowerCase()] ?? null },
    clone() {
      return response({ status, type, headers, body });
    },
  };
}

function request(pathname, { method = 'GET', mode = 'cors', origin = ORIGIN } = {}) {
  return { method, mode, url: origin + pathname };
}

function loadWorker({ version = 'dev', precache = ['/offline'], network }) {
  const stores = new Map();
  const listeners = {};
  const state = { skipped: 0, claimed: 0, fetches: [] };
  const key = (input) => (typeof input === 'string' ? new URL(input, ORIGIN).href : input.url);
  const caches = {
    async open(name) {
      if (!stores.has(name)) stores.set(name, new Map());
      const entries = stores.get(name);
      return { async put(input, value) { entries.set(key(input), value); } };
    },
    async match(input) {
      for (const entries of stores.values()) if (entries.has(key(input))) return entries.get(key(input));
      return undefined;
    },
    async keys() { return [...stores.keys()]; },
    async delete(name) { return stores.delete(name); },
  };
  const self = {
    location: { origin: ORIGIN },
    addEventListener(type, listener) { listeners[type] = listener; },
    skipWaiting() { state.skipped += 1; return Promise.resolve(); },
    clients: { claim() { state.claimed += 1; return Promise.resolve(); } },
  };
  let source = workerSource;
  const versionLine = "const VERSION = 'dev';";
  const precacheLine = "const PRECACHE = ['/offline'];";
  assert.equal(source.split(versionLine).length, 2, 'src/pwa.rs replaces exactly one version line');
  assert.equal(source.split(precacheLine).length, 2, 'make:pwa replaces exactly one precache line');
  source = source.replace(versionLine, `const VERSION = '${version}';`);
  source = source.replace(precacheLine, `const PRECACHE = ${JSON.stringify(precache)};`);
  const context = vm.createContext({
    self,
    caches,
    URL,
    console,
    async fetch(input, init) {
      state.fetches.push({ url: key(input), init });
      return network(key(input), init);
    },
  });
  vm.runInContext(source, context);
  async function extendable(type, data) {
    let pending = Promise.resolve();
    listeners[type]({ data, waitUntil(promise) { pending = promise; } });
    return pending;
  }
  async function fetchEvent(input) {
    let responded = null;
    listeners.fetch({ request: input, respondWith(promise) { responded = promise; } });
    return responded === null ? null : responded;
  }
  return { stores, state, extendable, fetchEvent, caches };
}

const offline = response({ headers: { 'Cache-Control': 'no-cache' }, body: 'offline' });

async function installAndActivation() {
  const worker = loadWorker({
    version: '1.2.0',
    precache: ['/offline', '/static/rullst.css', '/static/missing.png', '/static/private.js'],
    network: async (url) => {
      if (url.endsWith('/offline')) return offline;
      if (url.endsWith('/rullst.css')) return response({ headers: { 'Cache-Control': 'no-cache' }, body: 'css' });
      if (url.endsWith('/private.js')) return response({ headers: { 'Cache-Control': 'private, max-age=60' } });
      return response({ status: 404 });
    },
  });
  await worker.extendable('install');
  const stored = worker.stores.get('rullst-pwa-1.2.0');
  assert.deepEqual([...stored.keys()].sort(), [`${ORIGIN}/offline`, `${ORIGIN}/static/rullst.css`]);
  assert.equal(plain(worker.state.fetches[0].init).cache, 'no-cache');
  assert.equal(worker.state.fetches[0].init.credentials, 'same-origin');

  await worker.caches.open('rullst-pwa-1.1.0');
  await worker.caches.open('another-library');
  await worker.extendable('activate');
  assert.deepEqual([...worker.stores.keys()].sort(), ['another-library', 'rullst-pwa-1.2.0']);
  assert.equal(worker.state.claimed, 1);

  for (const control of ['no-store', 'private', 'public, NO-STORE', 'private="set-cookie"']) {
    const refused = loadWorker({
      network: async () => response({ headers: { 'Cache-Control': control } }),
    });
    await assert.rejects(refused.extendable('install'), /offline page/, control);
  }
  const unreachable = loadWorker({ network: async () => { throw new TypeError('offline'); } });
  await assert.rejects(unreachable.extendable('install'), /offline page/);
}

async function requestRouting() {
  let online = true;
  const pages = new Map([
    ['/account', response({ headers: { 'Cache-Control': 'no-store' }, body: 'personal' })],
    ['/about', response({ headers: { 'Cache-Control': 'public, max-age=60' }, body: 'about' })],
    ['/static/app.js', response({ headers: { 'Cache-Control': 'no-cache' }, body: 'app' })],
    ['/static/app.3f9a2c1b.css', response({ headers: { 'Cache-Control': 'public, max-age=31536000, immutable' }, body: 'v1' })],
    ['/static/vary.css', response({ headers: { Vary: '*' } })],
    ['/static/opaque.js', response({ type: 'opaque', status: 0 })],
    // A same-origin URL redirected to another origin answers with a CORS response.
    ['/static/redirected.js', response({ type: 'cors' })],
    ['/fragment', response({ body: 'fragment' })],
  ]);
  const worker = loadWorker({
    network: async (url) => {
      if (!online) throw new TypeError('Failed to fetch');
      const pathname = new URL(url).pathname;
      return pathname === '/offline' ? offline : pages.get(pathname) ?? response({ status: 404 });
    },
  });
  await worker.extendable('install');
  const cache = worker.stores.get('rullst-pwa-dev');

  // Requests the worker must leave to the browser.
  assert.equal(await worker.fetchEvent(request('/account', { method: 'POST', mode: 'navigate' })), null);
  assert.equal(await worker.fetchEvent(request('/static/app.js', { origin: 'https://cdn.example' })), null);
  assert.equal(await worker.fetchEvent(request('/fragment')), null);

  // Navigations are network-first; only storable pages are kept.
  assert.equal((await worker.fetchEvent(request('/account', { mode: 'navigate' }))).body, 'personal');
  assert.equal((await worker.fetchEvent(request('/about', { mode: 'navigate' }))).body, 'about');
  assert.equal(cache.has(`${ORIGIN}/account`), false);
  assert.equal(cache.has(`${ORIGIN}/about`), true);

  // Static files: unversioned ones are network-first, versioned ones cache-first.
  assert.equal((await worker.fetchEvent(request('/static/app.js'))).body, 'app');
  assert.equal((await worker.fetchEvent(request('/static/app.3f9a2c1b.css'))).body, 'v1');
  assert.equal((await worker.fetchEvent(request('/static/vary.css'))).status, 200);
  assert.equal(cache.has(`${ORIGIN}/static/vary.css`), false);
  assert.equal((await worker.fetchEvent(request('/static/opaque.js'))).type, 'opaque');
  assert.equal(cache.has(`${ORIGIN}/static/opaque.js`), false);
  assert.equal((await worker.fetchEvent(request('/static/redirected.js'))).type, 'cors');
  assert.equal(cache.has(`${ORIGIN}/static/redirected.js`), false);
  pages.set('/static/app.3f9a2c1b.css', response({ body: 'changed' }));
  assert.equal((await worker.fetchEvent(request('/static/app.3f9a2c1b.css'))).body, 'v1');

  online = false;
  assert.equal((await worker.fetchEvent(request('/about', { mode: 'navigate' }))).body, 'about');
  assert.equal((await worker.fetchEvent(request('/account', { mode: 'navigate' }))).body, 'offline');
  assert.equal((await worker.fetchEvent(request('/static/app.js'))).body, 'app');
  await assert.rejects(worker.fetchEvent(request('/static/never-seen.js')), /Failed to fetch/);

  await worker.extendable('message', { type: 'other' });
  assert.equal(worker.state.skipped, 0);
  await worker.extendable('message', null);
  await worker.extendable('message', { type: 'SKIP_WAITING' });
  assert.equal(worker.state.skipped, 1);
}

async function versionedNames() {
  const worker = loadWorker({ network: async () => offline });
  const cached = response({ body: 'cached' });
  await (await worker.caches.open('rullst-pwa-dev')).put(`${ORIGIN}/static/x`, cached);
  const versioned = ['/static/app.3f9a2c1b.css', '/static/chunk-3f9a2c1b.js', '/static/app.3f9a2c1b.js.map'];
  const plain = ['/static/report-20261008.pdf', '/static/app.css', '/static/.3f9a2c1b.css', '/static/app.3F9A2C1B.css', '/static/app.abcdefab.css', '/static/noextension', '/static/chunk-3f9a2c1bb', `/static/app.${'a1'.repeat(32)}b.css`];
  for (const pathname of [...versioned, ...plain]) {
    await (await worker.caches.open('rullst-pwa-dev')).put(ORIGIN + pathname, cached);
  }
  let networkCalls = 0;
  const probe = loadWorker({ network: async () => { networkCalls += 1; return response({ body: 'network' }); } });
  for (const pathname of [...versioned, ...plain]) {
    await (await probe.caches.open('rullst-pwa-dev')).put(ORIGIN + pathname, cached);
  }
  for (const pathname of versioned) {
    assert.equal((await probe.fetchEvent(request(pathname))).body, 'cached', pathname);
  }
  assert.equal(networkCalls, 0);
  for (const pathname of plain) {
    assert.equal((await probe.fetchEvent(request(pathname))).body, 'network', pathname);
  }
  assert.equal(networkCalls, plain.length);
}

function loadRegistration({ supported = true, controller = null, waiting = null, readyState = 'complete' } = {}) {
  const posted = [];
  const containerListeners = {};
  const windowListeners = {};
  const registrationListeners = {};
  const appended = [];
  const state = { reloads: 0, registered: [], warnings: [] };
  const registration = {
    waiting,
    installing: null,
    addEventListener(type, listener) { registrationListeners[type] = listener; },
  };
  const element = (tag) => {
    const listeners = {};
    return {
      tag,
      style: {},
      children: [],
      attributes: {},
      setAttribute(name, value) { this.attributes[name] = value; },
      appendChild(child) { this.children.push(child); },
      addEventListener(type, listener) { listeners[type] = listener; },
      click() { listeners.click(); },
    };
  };
  const container = {
    controller,
    register(url, options) {
      state.registered.push({ url, options });
      return state.registerResult ?? Promise.resolve(registration);
    },
    addEventListener(type, listener) { containerListeners[type] = listener; },
  };
  const document = {
    readyState,
    body: { appendChild(child) { appended.push(child); } },
    getElementById(id) { return appended.find((child) => child.id === id) ?? null; },
    createElement: element,
  };
  const window = {
    location: { reload() { state.reloads += 1; } },
    addEventListener(type, listener) { windowListeners[type] = listener; },
    console: { warn(...args) { state.warnings.push(args); } },
  };
  const navigator = supported ? { serviceWorker: container } : {};
  const context = vm.createContext({ navigator, document, window });
  return { context, state, appended, registration, container, containerListeners, windowListeners, registrationListeners, posted, run: () => vm.runInContext(registrationSource, context) };
}

function worker() {
  const posted = [];
  const listeners = {};
  return {
    posted,
    state: 'installing',
    postMessage(message) { posted.push(message); },
    addEventListener(type, listener) { listeners[type] = listener; },
    become(state) { this.state = state; listeners.statechange(); },
  };
}

const tick = () => new Promise((resolve) => setImmediate(resolve));
// Objects created inside the script's realm have that realm's prototypes.
const plain = (value) => JSON.parse(JSON.stringify(value));

async function registration() {
  const unsupported = loadRegistration({ supported: false });
  unsupported.run();
  assert.equal(unsupported.state.registered.length, 0);

  // Registration waits for the load event, then registers the root scope.
  const later = loadRegistration({ readyState: 'loading' });
  later.run();
  assert.equal(later.state.registered.length, 0);
  later.windowListeners.load();
  await tick();
  assert.deepEqual(plain(later.state.registered), [{ url: '/sw.js', options: { scope: '/' } }]);

  // A waiting worker without a controlling page is a first install too.
  const uncontrolled = loadRegistration({ waiting: worker() });
  uncontrolled.run();
  await tick();
  assert.equal(uncontrolled.appended.length, 0);

  // First install: no prompt and no reload when the worker claims the page.
  const first = loadRegistration();
  first.run();
  await tick();
  const installing = worker();
  first.registration.installing = installing;
  first.registrationListeners.updatefound();
  installing.become('installed');
  assert.equal(first.appended.length, 0);
  first.containerListeners.controllerchange();
  assert.equal(first.state.reloads, 0);

  // An update waiting behind a controlled page shows one prompt.
  const waiting = worker();
  const update = loadRegistration({ controller: {}, waiting });
  update.run();
  await tick();
  assert.equal(update.appended.length, 1);
  const bar = update.appended[0];
  assert.equal(bar.id, 'rullst-pwa-update');
  assert.equal(bar.attributes.role, 'status');
  assert.equal(bar.style.position, 'fixed');
  assert.equal(bar.attributes.style, undefined, 'no style attribute: CSP refuses inline styles');
  const [text, button] = bar.children;
  assert.equal(text.textContent, 'A new version of this app is available.');
  assert.equal(button.textContent, 'Reload');
  assert.equal(button.type, 'button');
  const next = worker();
  update.registration.installing = next;
  update.registrationListeners.updatefound();
  next.become('installed');
  assert.equal(update.appended.length, 1, 'one prompt at a time');

  update.containerListeners.controllerchange();
  assert.equal(update.state.reloads, 0, 'no reload before the user asks');
  button.click();
  assert.equal(button.disabled, true);
  assert.deepEqual(plain(waiting.posted), [{ type: 'SKIP_WAITING' }]);
  update.containerListeners.controllerchange();
  update.containerListeners.controllerchange();
  assert.equal(update.state.reloads, 1);

  // A worker that is not yet installed shows nothing.
  const pending = loadRegistration({ controller: {} });
  pending.run();
  await tick();
  pending.registrationListeners.updatefound();
  const busy = worker();
  pending.registration.installing = busy;
  pending.registrationListeners.updatefound();
  busy.become('installing');
  assert.equal(pending.appended.length, 0);

  const failing = loadRegistration();
  failing.state.registerResult = Promise.reject(new Error('blocked'));
  failing.run();
  await tick();
  assert.equal(failing.state.warnings.length, 1);
}

(async () => {
  await installAndActivation();
  await requestRouting();
  await versionedNames();
  await registration();
  console.log('PWA worker and registration contracts passed');
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
