import './media-upload-tests.mjs';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
let input = '';
for await (const chunk of process.stdin) { input += chunk; assert(Buffer.byteLength(input) < 16384); }
const { origin, provider, teacher, learner, file } = JSON.parse(input);
assert(/^http:\/\/127\.0\.0\.1:\d+$/.test(origin)); assert(/^http:\/\/127\.0\.0\.1:\d+$/.test(provider));
const profile = await mkdtemp(join(tmpdir(), 'rullst-media-browser-'));
const chrome = spawn(process.env.CHROME_BIN || 'google-chrome', [
  '--headless=new', '--no-first-run', '--no-default-browser-check', '--disable-background-networking',
  '--disable-component-update', '--disable-dev-shm-usage', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank',
], { stdio: ['ignore', 'ignore', 'pipe'] });
let socket;
const deadline = setTimeout(() => chrome.kill('SIGKILL'), 90000);
try {
  const endpoint = await new Promise((resolve, reject) => {
    let stderr = '';
    const timer = setTimeout(() => reject(new Error('Chromium startup timeout')), 45000);
    const fail = error => { clearTimeout(timer); reject(error); };
    chrome.once('error', fail); chrome.once('exit', () => fail(new Error('Chromium exited')));
    chrome.stderr.on('data', bytes => {
      stderr = (stderr + bytes).slice(-8192);
      const found = stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);
      if (found) { clearTimeout(timer); resolve(found[1]); }
    });
  });
  socket = new WebSocket(endpoint);
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('CDP connection timeout')), 15000);
    socket.addEventListener('open', () => { clearTimeout(timer); resolve(); }, { once: true });
    socket.addEventListener('error', () => { clearTimeout(timer); reject(new Error('CDP failed')); }, { once: true });
  });
  const pending = new Map(); let sequence = 0;
  const failures = []; const requests = []; const responses = [];
  socket.addEventListener('message', ({ data }) => {
    const message = JSON.parse(data);
    if (message.id) {
      const promise = pending.get(message.id); if (!promise) return;
      pending.delete(message.id); clearTimeout(promise.timer);
      if (message.error) promise.reject(new Error(message.error.message)); else promise.resolve(message.result);
    } else {
      if (message.method === 'Runtime.exceptionThrown') failures.push('uncaught browser exception');
      if (message.method === 'Network.requestWillBeSent') {
        const request = message.params.request;
        if (![origin + '/', provider + '/', 'data:', 'about:'].some(prefix => request.url.startsWith(prefix))) failures.push('external request');
        requests.push({ url: request.url, method: request.method });
      }
      if (message.method === 'Network.responseReceived') responses.push(message.params.response);
    }
  });
  const call = (method, params = {}, sessionId) => new Promise((resolve, reject) => {
    const id = ++sequence; const timer = setTimeout(() => reject(new Error(`CDP timeout: ${method}`)), 15000);
    pending.set(id, { resolve, reject, timer }); socket.send(JSON.stringify({ id, method, params, sessionId }));
  });
  const { targetId } = await call('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await call('Target.attachToTarget', { targetId, flatten: true });
  const send = (method, params = {}) => call(method, params, sessionId);
  const evaluate = async (expression, contextId) => {
    const result = await send('Runtime.evaluate', { expression, contextId, returnByValue: true, awaitPromise: true, userGesture: true });
    assert(!result.exceptionDetails, 'browser evaluation failed'); return result.result.value;
  };
  const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
  const wait = async (predicate, label) => {
    for (let i = 0; i < 200; i++) { if (await predicate().catch(() => false)) return; await pause(50); }
    assert.fail(`Browser condition failed: ${label}`);
  };
  const status = expected => wait(() => evaluate(`document.querySelector('#status')?.textContent === ${JSON.stringify(expected)}`), expected);
  const press = async id => {
    assert(await evaluate(`document.getElementById(${JSON.stringify(id)}).focus(); document.activeElement.id === ${JSON.stringify(id)}`));
    await send('Input.dispatchKeyEvent', { type: 'keyDown', key: ' ', code: 'Space', windowsVirtualKeyCode: 32 });
    await send('Input.dispatchKeyEvent', { type: 'keyUp', key: ' ', code: 'Space', windowsVirtualKeyCode: 32 });
  };
  const cookie = value => send('Network.setCookie', { name: 'media_fixture', value, url: origin, httpOnly: true, sameSite: 'Lax' });
  const request = (action, tenant = 'school-a', csrf = null, credentials = 'same-origin') => evaluate(`fetch('/video/${action}', {method:'POST',credentials:${JSON.stringify(credentials)},headers:{'Content-Type':'application/json','X-CSRF-Token':${csrf === null ? "document.querySelector('meta[name=csrf]').content" : JSON.stringify(csrf)}},body:JSON.stringify({scope:{tenant:${JSON.stringify(tenant)},course:'rust-course'}})}).then(r=>r.status)`);
  await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
  await cookie(teacher); await send('Page.navigate', { url: origin + '/' }); await status('ready');
  assert.equal(await request('create', 'school-a', 'wrong'), 403);
  assert([401, 403].includes(await request('create', 'school-a', null, 'omit')));
  assert.equal(await request('create', 'school-b'), 403);
  assert.equal(await evaluate("fetch('/bunny/events',{method:'POST',headers:{'Content-Type':'application/json'},body:'{}'}).then(r=>r.status)"), 401);
  await press('create'); await status('created');
  const { root } = await send('DOM.getDocument');
  const { nodeId } = await send('DOM.querySelector', { nodeId: root.nodeId, selector: '#file' });
  await send('DOM.setFileInputFiles', { nodeId, files: [file] });
  await press('upload'); await status('uploaded');
  await press('publish'); await status('published');
  await press('playback'); await status('playing');
  await wait(async () => (await send('Page.getFrameTree')).frameTree.childFrames?.length === 1, 'player frame');
  const frame = (await send('Page.getFrameTree')).frameTree.childFrames[0].frame.id;
  const { executionContextId } = await send('Page.createIsolatedWorld', { frameId: frame, worldName: 'media-acceptance' });
  await wait(() => evaluate("document.querySelector('video')?.readyState >= 2", executionContextId), 'controlled media decoded');
  assert(await evaluate("const v=document.querySelector('video'); v.controls && v.textTracks.length===1 && document.querySelector('track').srclang==='en'", executionContextId));
  await evaluate("document.querySelector('video').focus(); true", executionContextId);
  await send('Input.dispatchKeyEvent', { type: 'keyDown', key: ' ', code: 'Space', windowsVirtualKeyCode: 32 });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: ' ', code: 'Space', windowsVirtualKeyCode: 32 });
  await wait(() => evaluate("document.querySelector('video').currentTime > 0", executionContextId), 'keyboard playback');
  await cookie(learner); assert.equal(await request('playback'), 200); assert.equal(await request('delete'), 403);
  await cookie('unknown-session'); assert.equal(await request('playback'), 401);
  await cookie(teacher); await press('withdraw'); await status('withdrawn'); assert.equal(await request('playback'), 403);
  assert.equal(await evaluate("document.querySelectorAll('iframe').length"), 0);
  await press('delete'); await status('deleted');
  const main = responses.find(r => r.url === origin + '/');
  const headers = Object.fromEntries(Object.entries(main.headers).map(([k, v]) => [k.toLowerCase(), v]));
  assert(headers['content-security-policy'].includes('frame-ancestors')); assert.equal(headers['x-content-type-options'], 'nosniff');
  assert.equal(requests.filter(r => r.url === provider + '/tusupload' && r.method === 'POST').length, 1);
  assert(!await evaluate("document.documentElement.outerHTML.includes('fixture_api_key')"));
  assert.deepEqual(failures, []);
  console.log('Chromium media journey passed: authenticated upload, CSRF/tenant denial, real media/captions/keyboard playback, entitlement and withdrawal/deletion.');
} finally {
  socket?.close(); clearTimeout(deadline);
  if (chrome.exitCode === null && chrome.signalCode === null) {
    const exited = once(chrome, 'exit'); chrome.kill('SIGTERM');
    const forced = setTimeout(() => chrome.kill('SIGKILL'), 3000); await exited; clearTimeout(forced);
  }
  await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
}
