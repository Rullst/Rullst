// Real generated LMS HTTP/CSP/session/forms; no provider and no synthetic server.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
let input = '';
for await (const chunk of process.stdin) { input += chunk; assert(Buffer.byteLength(input) <= 16384); }
const { origin, cookie } = JSON.parse(input);
assert(/^http:\/\/127\.0\.0\.1:\d+$/.test(origin));
const profile = await mkdtemp(join(tmpdir(), 'rullst-supervision-browser-'));
const chrome = spawn(process.env.CHROME_BIN || 'google-chrome', [
  '--headless=new', '--no-first-run', '--no-default-browser-check',
  '--disable-background-networking', '--disable-component-update', '--disable-dev-shm-usage',
  '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank'
], { stdio: ['ignore', 'ignore', 'pipe'] });
let socket;
const deadline = setTimeout(() => chrome.kill('SIGKILL'), 90000);
try {
  const endpoint = await new Promise((accept, reject) => {
    let stderr = '';
    const timer = setTimeout(() => reject(new Error('Chrome startup timeout')), 30000);
    const fail = error => { clearTimeout(timer); reject(error); };
    chrome.once('error', fail); chrome.once('exit', code => fail(new Error(`Chrome exited: ${code}`)));
    chrome.stderr.on('data', chunk => {
      stderr = (stderr + chunk).slice(-8192);
      const found = stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);
      if (found) { clearTimeout(timer); accept(found[1]); }
    });
  });
  socket = new WebSocket(endpoint);
  await new Promise((accept, reject) => {
    const timer = setTimeout(() => reject(new Error('CDP handshake timeout')), 15000);
    socket.addEventListener('open', () => { clearTimeout(timer); accept(); }, { once: true });
    socket.addEventListener('error', () => { clearTimeout(timer); reject(new Error('CDP handshake failed')); }, { once: true });
  });
  let sequence = 0;
  const pending = new Map();
  const handlers = new Map();
  const failures = [];
  socket.addEventListener('message', ({ data }) => {
    const message = JSON.parse(data);
    if (message.id) {
      const op = pending.get(message.id); if (!op) return;
      pending.delete(message.id); clearTimeout(op.timer);
      if (message.error) op.reject(new Error(JSON.stringify(message.error))); else op.accept(message.result);
    } else { try { handlers.get(message.method)?.(message.params, message.sessionId); } catch (error) { failures.push(String(error)); } }
  });
  const call = (method, params = {}, sessionId) => new Promise((accept, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 15000);
    pending.set(id, { accept, reject, timer }); socket.send(JSON.stringify({ id, method, params, sessionId }));
  });
  const { targetId } = await call('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await call('Target.attachToTarget', { targetId, flatten: true });
  const send = (method, params) => call(method, params, sessionId);
  const evaluate = async expression => {
    const result = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
    assert(!result.exceptionDetails, 'browser evaluation failed'); return result.result.value;
  };
  const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
  const waitFor = async (predicate, label) => {
    for (let i = 0; i < 200; i++) { try { if (await predicate()) return; } catch { /* navigation replaces context */ } await pause(50); }
    const state = await evaluate("JSON.stringify({path:location.pathname,title:document.title,text:document.body?.innerText?.slice(-400),checked:document.querySelector('input[name=acknowledge]')?.checked,valid:document.querySelector('form')?.checkValidity()})").catch(() => 'unavailable');
    assert.fail(`browser condition timed out: ${label}; ${state}`);
  };
  const waitState = state => waitFor(() => evaluate(`document.getElementById('session-state')?.textContent === ${JSON.stringify(state)}`), state);
  const keyboard = async (selector, key, code, virtual) => {
    assert(await evaluate(`!!document.querySelector(${JSON.stringify(selector)})`));
    await evaluate(`document.querySelector(${JSON.stringify(selector)}).focus(); true`);
    assert(await evaluate(`document.activeElement === document.querySelector(${JSON.stringify(selector)})`), 'keyboard focus');
    await send('Input.dispatchKeyEvent', { type: 'keyDown', key, code, windowsVirtualKeyCode: virtual });
    await send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode: virtual });
  };
  const press = selector => keyboard(selector, ' ', 'Space', 32);
  const acknowledge = async () => { await keyboard('input[name=acknowledge]', ' ', 'Space', 32); assert(await evaluate('document.querySelector("input[name=acknowledge]").checked'), 'keyboard acknowledgement'); };
  await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
  const reports = [];
  const accepted = new Set();
  handlers.set('Runtime.exceptionThrown', () => failures.push('uncaught page exception'));
  handlers.set('Network.requestWillBeSent', params => {
    if (!params.request.url.startsWith(origin + '/')) failures.push('non-origin page request');
    if (new URL(params.request.url).pathname === '/supervision/event') {
      assert(reports.length < 32, 'bounded browser event fixture');
      const fields = new URLSearchParams(params.request.postData);
      assert.deepEqual([...fields.keys()].sort(), ['_token','event','issued_at','proof','revision','sequence'].sort());
      assert(['page_hidden','page_visible'].includes(fields.get('event')));
      reports.push(params.requestId);
    }
  });
  handlers.set('Network.responseReceived', params => {
    if (reports.includes(params.requestId) && params.response.status === 204) accepted.add(params.requestId);
  });
  await send('Page.addScriptToEvaluateOnNewDocument', { source: `
    window.supervisionCaptureCalls = 0;
    const rejectCapture = () => { window.supervisionCaptureCalls++; throw new Error('Capture forbidden by test'); };
    if (navigator.mediaDevices) { navigator.mediaDevices.getUserMedia = rejectCapture; navigator.mediaDevices.getDisplayMedia = rejectCapture; }
    if (navigator.geolocation) { navigator.geolocation.getCurrentPosition = rejectCapture; navigator.geolocation.watchPosition = rejectCapture; }
    if (navigator.clipboard) { navigator.clipboard.read = rejectCapture; navigator.clipboard.readText = rejectCapture; }
  ` });
  await send('Network.setCookie', { name: 'rullst_session', value: cookie, url: origin, httpOnly: true, sameSite: 'Lax' });
  const start = origin + '/supervision/exam?school=academy-demo&lesson=1';
  await send('Emulation.setScriptExecutionDisabled', { value: true });
  await send('Page.navigate', { url: start });
  await waitFor(() => evaluate('!!document.querySelector("button[value=start]")'), 'no-JS start');
  await acknowledge(); await press('button[value=start]'); await waitState('Active');
  await press('button[value=pause]'); await waitState('Paused');
  await press('button[value=end]'); await waitState('Ended');
  assert.equal(reports.length, 0, 'no-JS controls must send no visibility reports');
  console.log('Supervision browser: keyboard start/pause/end without JavaScript passed');
  await send('Emulation.setScriptExecutionDisabled', { value: false });
  await send('Page.navigate', { url: start });
  await waitFor(() => evaluate('!!document.querySelector("button[value=start]")'), 'JS start');
  await acknowledge(); await press('button[value=start]'); await waitState('Active');
  await waitFor(() => evaluate('!!document.getElementById("visibility-report")'), 'visibility collector');
  const other = await call('Target.createTarget', { url: 'about:blank', background: true });
  const toggle = async () => {
    await call('Target.activateTarget', { targetId: other.targetId }); await pause(1300);
    await send('Page.bringToFront'); await pause(1300);
  };
  await toggle();
  await waitFor(async () => accepted.size > 0, 'real visibility report accepted');
  assert.equal(await evaluate('window.supervisionCaptureCalls'), 0);
  assert(await evaluate('getComputedStyle(document.querySelector("#visibility-report")).display === "none"'));
  await press('button[value=pause]'); await waitState('Paused');
  const afterPause = reports.length; await toggle(); assert.equal(reports.length, afterPause, 'no reports after pause');
  await acknowledge(); await press('button[value=resume]'); await waitState('Active');
  await press('button[value=end]'); await waitState('Ended');
  const afterEnd = reports.length; await toggle(); assert.equal(reports.length, afterEnd, 'no reports after end');
  assert.equal(await evaluate('window.supervisionCaptureCalls'), 0); assert.deepEqual(failures, []);
  console.log('Supervision browser: real visibility, minimal payload, pause/resume/end, no capture or external request passed');
  await call('Browser.close');
} finally {
  clearTimeout(deadline); socket?.close();
  if (chrome.exitCode === null) { const stopped = once(chrome, 'exit').catch(() => {}); chrome.kill('SIGKILL'); await stopped; }
  await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
}
