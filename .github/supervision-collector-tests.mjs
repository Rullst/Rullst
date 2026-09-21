// Deterministic fault tests execute the shipped collector, with a controlled clock/network.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { runInNewContext } from 'node:vm';
const code = await readFile(new URL('../cargo-rullst/src/generators/supervision/visibility.js', import.meta.url), 'utf8');
function fixture(selected = 15) {
  let now = 0, nextTimer = 0;
  const timers = new Map(), requests = [], controls = [];
  class Target {
    listeners = new Map();
    addEventListener(name, fn) { this.listeners.set(name, fn); }
    removeEventListener(name, fn) { if (this.listeners.get(name) === fn) this.listeners.delete(name); }
    emit(name, trusted = true) { this.listeners.get(name)?.({ isTrusted: trusted,
      get clipboardData() { assert.fail('clipboard content read'); }, get key() { assert.fail('key content read'); } }); }
  }
  const doc = new Target(), win = new Target(), control = new Target(); controls.push(control);
  const status = { textContent: '' };
  const fields = { _token: 'csrf', revision: '1', issued_at: '0', proof: 'proof', sequence: '0' };
  const form = { action: '/supervision/event', dataset: { collection: String(selected), expires: '600' }, elements: { namedItem: name => ({ value: fields[name] }) } };
  doc.hidden = true; doc.fullscreenElement = null;
  doc.getElementById = id => id === 'visibility-report' ? form : status;
  doc.querySelectorAll = () => controls;
  const scope = { document: doc, window: win, URLSearchParams, AbortController,
    FormData: class { *[Symbol.iterator]() { yield* Object.entries(fields); } },
    performance: { now: () => now }, Date: { now: () => now },
    setTimeout: (fn, delay) => { const id = ++nextTimer; timers.set(id, { fn, due: now + delay }); return id; },
    clearTimeout: id => timers.delete(id),
    fetch: (url, options) => new Promise((resolve, reject) => {
      assert.equal(url, '/supervision/event');
      options.signal.addEventListener('abort', () => reject(new Error('aborted')));
      requests.push({ fields: Object.fromEntries(options.body), signal: options.signal, resolve, reject });
    }) };
  runInNewContext(code, scope);
  const flush = async () => { for (let i = 0; i < 5; i++) await Promise.resolve(); };
  const advance = async ms => {
    now += ms;
    for (const [id, task] of [...timers]) if (task.due <= now) { timers.delete(id); task.fn(); }
    await flush();
  };
  return { doc, win, control, requests, status, advance, flush, timers };
}
{
  const f = fixture(1); f.doc.emit('copy'); f.win.emit('blur'); f.doc.emit('visibilitychange', false);
  assert.equal(f.requests.length, 0);
  f.doc.emit('visibilitychange'); assert.equal(f.requests.length, 1);
  assert.deepEqual(Object.keys(f.requests[0].fields).sort(), ['_token','event','issued_at','proof','revision','sequence'].sort());
  assert.equal(f.requests[0].fields.event, 'page_hidden');
  f.control.emit('submit'); await f.flush();
  assert(f.requests[0].signal.aborted); f.doc.emit('visibilitychange');
  assert.equal(f.requests.length, 1); assert.equal(f.timers.size, 0);
}
{
  const f = fixture(); f.doc.emit('copy'); f.doc.emit('cut'); f.doc.emit('paste');
  assert.equal(f.requests.length, 1); f.requests[0].resolve({ status: 204 }); await f.flush();
  await f.advance(1099); assert.equal(f.requests.length, 1);
  await f.advance(1); assert.equal(f.requests.length, 2);
  assert.equal(f.requests[1].fields.event, 'cut_attempt'); assert.equal(f.requests[1].fields.sequence, '2');
  f.requests[1].resolve({ status: 204 }); await f.flush(); await f.advance(1100);
  assert.equal(f.requests[2].fields.event, 'paste_attempt'); assert.equal(f.requests[2].fields.sequence, '3');
  f.win.emit('pagehide'); await f.flush(); assert(f.requests[2].signal.aborted);
  f.doc.emit('copy'); assert.equal(f.requests.length, 3);
}
{
  const f = fixture(); for (let i = 0; i < 18; i++) f.doc.emit('copy'); await f.flush();
  assert.equal(f.requests.length, 1); assert(f.requests[0].signal.aborted);
  assert.match(f.status.textContent, /too many events/); await f.advance(10000); assert.equal(f.requests.length, 1);
}
for (const failure of [200, 401, 403, 409, 429, 500, 'network']) {
  const f = fixture(); f.win.emit('blur'); f.doc.emit('copy');
  if (failure === 'network') f.requests[0].reject(new Error('offline'));
  else f.requests[0].resolve({ status: failure });
  await f.flush(); await f.advance(1200); f.doc.emit('paste'); assert.equal(f.requests.length, 1);
  assert.match(f.status.textContent, /Collection stopped/);
}
{
  const f = fixture(); f.doc.emit('copy'); await f.advance(300000);
  assert(f.requests[0].signal.aborted); f.win.emit('focus'); assert.equal(f.requests.length, 1);
  assert.match(f.status.textContent, /renew/);
}
for (const selection of [-1, 16, NaN]) {
  const f = fixture(selection); f.doc.emit('copy'); assert.equal(f.requests.length, 0);
  assert.match(f.status.textContent, /configuration/);
}
console.log('Supervision collector: selected categories, minimal payload, serialized sequence/rate, overflow, expiry, revocation and connection failures passed');
