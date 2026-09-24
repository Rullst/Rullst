import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm } from 'node:fs/promises';
import { createInterface } from 'node:readline';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const input = createInterface({ input: process.stdin });
const incoming = [], readers = [];
input.on('line', line => { assert(line.length <= 16384); const value = JSON.parse(line); if (readers.length) readers.shift()(value); else incoming.push(value); });
const next = () => incoming.length ? Promise.resolve(incoming.shift()) : new Promise(resolve => readers.push(resolve));
const configuration = await next();
const { origin, teacher, freshTeacher, learner, other, csrf } = configuration;
assert(/^http:\/\/127\.0\.0\.1:\d+$/.test(origin));
const profile = await mkdtemp(join(tmpdir(), 'rullst-live-browser-'));
const chrome = spawn(process.env.CHROME_BIN || 'google-chrome', [
  '--headless=new', '--no-first-run', '--no-default-browser-check', '--disable-background-networking',
  '--disable-component-update', '--disable-dev-shm-usage', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank',
], { stdio: ['ignore', 'ignore', 'pipe'] });
const deadline = setTimeout(() => chrome.kill('SIGKILL'), 140000);
let socket;
try {
  const endpoint = await new Promise((resolve, reject) => {
    let error = '';
    const timer = setTimeout(() => reject(new Error('Chromium startup timeout')), 30000);
    chrome.once('error', reject);
    chrome.once('exit', () => { clearTimeout(timer); reject(new Error('Chromium exited')); });
    chrome.stderr.on('data', bytes => {
      error = (error + bytes).slice(-8192);
      const found = error.match(/DevTools listening on (ws:\/\/[^\s]+)/);
      if (found) { clearTimeout(timer); resolve(found[1]); }
    });
  });
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('CDP connection timeout')), 10000);
    socket.addEventListener('open', () => { clearTimeout(timer); resolve(); }, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });
  const pending = new Map(); let sequence = 0;
  const errors = [], sentFrames = [];
  socket.addEventListener('message', ({ data }) => {
    const message = JSON.parse(data);
    if (message.id) {
      const waiter = pending.get(message.id); if (!waiter) return;
      pending.delete(message.id); clearTimeout(waiter.timer);
      if (message.error) waiter.reject(new Error(message.error.message)); else waiter.resolve(message.result);
    } else {
      if (message.method === 'Runtime.exceptionThrown') errors.push('uncaught browser exception');
      if (message.method === 'Network.webSocketFrameSent' && message.params.response.opcode === 1) {
        sentFrames.push(JSON.parse(message.params.response.payloadData));
      }
      if (message.method === 'Network.requestWillBeSent') {
        const url = message.params.request.url;
        if (!url.startsWith(origin + '/') && !url.startsWith('about:')) errors.push('external browser request');
      }
    }
  });
  const call = (method, params = {}, sessionId) => new Promise((resolve, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP deadline: ${method}`)); }, 15000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method, params, sessionId }));
  });
  async function page(browserContextId) {
    const { targetId } = await call('Target.createTarget', { url: 'about:blank', browserContextId });
    const { sessionId } = await call('Target.attachToTarget', { targetId, flatten: true });
    const send = (method, params = {}) => call(method, params, sessionId);
    const evaluate = async expression => {
      const result = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true, userGesture: true });
      assert(!result.exceptionDetails, 'browser expression must succeed');
      return result.result.value;
    };
    await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
    return { targetId, send, evaluate };
  }
  const first = await page();
  async function cookie(value, target = first) {
    await target.send('Network.setCookie', { name: 'live_fixture', value, url: origin, httpOnly: true, sameSite: 'Lax' });
    await target.send('Network.setCookie', { name: 'rullst_csrf', value: csrf, url: origin, sameSite: 'Lax' });
  }
  async function isolatedPage(value) {
    const { browserContextId } = await call('Target.createBrowserContext');
    const target = await page(browserContextId);
    await cookie(value, target);
    return target;
  }
  const pause = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds));
  async function wait(predicate, label, describe = null) {
    for (let i = 0; i < 400; i++) { if (await predicate().catch(() => false)) return; await pause(50); }
    const detail = describe ? await describe().catch(() => ({ observation: 'unavailable' })) : null;
    assert.fail(`Browser state did not converge: ${label}${detail ? ' ' + JSON.stringify(detail) : ''}`);
  }
  async function ready(page, value) {
    await wait(() => page.evaluate(`window.live?.state === 'ready' && document.querySelector('#value')?.textContent === '${value}'`), `ready ${value}`);
  }
  await cookie(teacher);
  await first.send('Page.navigate', { url: origin + '/' }); await ready(first, 0);
  const second = await page();
  await second.send('Page.navigate', { url: origin + '/' }); await ready(second, 0);
  assert(await first.evaluate("window.live.send('increment')")); await ready(first, 1);
  assert(await second.evaluate("window.live.send('increment')")); await ready(second, 1);
  assert(await second.evaluate("window.outcomes.some(value => value.outcome === 'conflict')"));

  assert(await first.evaluate("window.live.send('increment-drop')"));
  await ready(first, 2);
  assert(await first.evaluate("window.outcomes.some(value => value.outcome === 'unknown')"));
  assert.equal(sentFrames.filter(frame => frame.action === 'increment-drop').length, 1, 'ambiguous mutation must never be replayed');

  // Force a new connection while offline; offline button interactions stay local.
  await first.send('Network.emulateNetworkConditions', { offline: true, latency: 0, downloadThroughput: 0, uploadThroughput: 0 });
  await first.evaluate('window.live.reconnect()');
  await wait(() => first.evaluate("window.live.state !== 'ready'"), 'offline recovery');
  const before = sentFrames.length;
  assert.equal(await first.evaluate("window.live.send('increment')"), false);
  await first.evaluate("document.querySelector('#increment')?.click(); true");
  assert.equal(sentFrames.length, before);
  await second.evaluate('window.live.reconnect()'); await ready(second, 2);
  assert(await second.evaluate("window.live.send('increment')")); await ready(second, 3);
  await first.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });
  await ready(first, 3);
  assert.equal(sentFrames.length, before + 1, 'offline actions must not be queued');

  process.stdout.write(JSON.stringify({ command: 'restart' }) + '\n');
  assert.equal((await next()).restarted, true);
  await ready(first, 3); await ready(second, 3);
  assert(await first.evaluate("window.live.send('increment')")); await ready(first, 4);

  // Separate principals need separate cookie jars: an automatic reconnect in
  // either teacher tab must not inherit a learner/foreign-tenant credential.
  const foreign = await isolatedPage(other);
  // An empty 403 navigation creates an opaque Chromium error document. Open
  // the public same-origin module first, then fetch the protected page below.
  await foreign.send('Page.navigate', { url: origin + '/live-module.js' });
  await wait(() => foreign.evaluate(`location.origin === ${JSON.stringify(origin)}`), 'foreign page origin');
  assert.equal(await foreign.evaluate("fetch('/').then(response => response.status)"), 403);
  const restricted = await isolatedPage(learner);
  await restricted.send('Page.navigate', { url: origin + '/' }); await ready(restricted, 4);
  assert(await restricted.evaluate("window.live.send('increment')"));
  await wait(() => restricted.evaluate("window.live.state === 'denied' && document.querySelector('#view').textContent === ''"), 'domain permission denied');

  const { cookies } = await first.send('Network.getCookies', { urls: [origin] });
  assert.equal(cookies.find(value => value.name === 'live_fixture')?.value, teacher,
    'other principals must not change the credential used for teacher reconnects');
  const deniedCsrf = await first.evaluate("fetch('/test/revoke',{method:'POST',headers:{'X-CSRF-Token':'wrong'}}).then(response=>response.status)");
  assert.equal(deniedCsrf, 403);
  assert.equal(await first.evaluate(`fetch('/test/revoke',{method:'POST',headers:{'X-CSRF-Token':${JSON.stringify(csrf)}}}).then(response=>response.status)`), 204);
  await wait(() => first.evaluate("window.live.state === 'denied' && document.querySelector('#view').textContent === ''"), 'revoked connection',
    () => first.evaluate("({state: window.live?.state, emptyView: document.querySelector('#view')?.textContent === ''})"));
  await cookie(freshTeacher);
  assert(await first.evaluate('window.live.reconnect()')); await ready(first, 4);
  assert.deepEqual(errors, []);
  process.stdout.write(JSON.stringify({ command: 'passed' }) + '\n');
} finally {
  input.close(); socket?.close(); clearTimeout(deadline);
  if (chrome.exitCode === null && chrome.signalCode === null) {
    const exited = once(chrome, 'exit'); chrome.kill('SIGTERM');
    const forced = setTimeout(() => chrome.kill('SIGKILL'), 3000); await exited; clearTimeout(forced);
  }
  await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
}
