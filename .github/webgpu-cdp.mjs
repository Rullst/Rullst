// Chromium/CDP fixture. Software rendering is test-only, never a deployment flag.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

export async function browser(origin, journey) {
  const profile = await mkdtemp(join(tmpdir(), 'rullst-webgpu-'));
  const chrome = spawn(process.env.CHROME_BIN || 'google-chrome', [
    '--headless=new', '--no-first-run', '--no-default-browser-check',
    '--disable-background-networking', '--disable-component-update', '--disable-dev-shm-usage',
    '--enable-unsafe-webgpu', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
    '--enable-gpu', '--enable-features=Vulkan', '--use-vulkan=swiftshader',
    '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank',
  ], { stdio: ['ignore', 'ignore', 'pipe'] });
  let socket, diagnostics = '';
  chrome.stderr.on('data', bytes => { diagnostics = (diagnostics + bytes).slice(-8192); });
  const pending = new Map();
  const deadline = setTimeout(() => chrome.kill('SIGKILL'), 90000);
  try {
    const endpoint = await new Promise((resolve, reject) => {
      let stderr = '';
      const timer = setTimeout(() => reject(new Error('Chromium startup timeout')), 20000);
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
      const timer = setTimeout(() => reject(new Error('CDP connection timeout')), 10000);
      socket.addEventListener('open', () => { clearTimeout(timer); resolve(); }, { once: true });
      socket.addEventListener('error', () => { clearTimeout(timer); reject(new Error('CDP failed')); }, { once: true });
    });
    let sequence = 0;
    const failures = [], responses = [];
    socket.addEventListener('message', ({ data }) => {
      const message = JSON.parse(data);
      if (message.id) {
        const promise = pending.get(message.id); if (!promise) return;
        pending.delete(message.id); clearTimeout(promise.timer);
        if (message.error) promise.reject(new Error(message.error.message)); else promise.resolve(message.result);
      } else if (message.method === 'Runtime.exceptionThrown') {
        failures.push(message.params.exceptionDetails.exception?.description || 'uncaught browser exception');
      } else if (message.method === 'Network.requestWillBeSent') {
        const url = message.params.request.url;
        if (![origin + '/', 'data:', 'about:'].some(prefix => url.startsWith(prefix))) failures.push(`external request: ${url}`);
      } else if (message.method === 'Network.loadingFailed' && message.params.blockedReason === 'csp') {
        failures.push('CSP blocked an example resource');
      } else if (message.method === 'Network.responseReceived') responses.push(message.params.response);
    });
    const call = (method, params = {}, sessionId) => new Promise((resolve, reject) => {
      const id = ++sequence;
      const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 15000);
      pending.set(id, { resolve, reject, timer }); socket.send(JSON.stringify({ id, method, params, sessionId }));
    });
    const { targetId } = await call('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await call('Target.attachToTarget', { targetId, flatten: true });
    const send = (method, params = {}) => call(method, params, sessionId);
    const evaluate = async expression => {
      const result = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true, userGesture: true });
      assert(!result.exceptionDetails, result.exceptionDetails?.exception?.description || 'browser evaluation failed');
      return result.result.value;
    };
    const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
    const wait = async (expression, label) => {
      for (let i = 0; i < 200; i++) {
        if (await evaluate(expression).catch(() => false)) return;
        await pause(50);
      }
      assert.fail(`Browser condition failed: ${label}; ${await evaluate('JSON.stringify(globalThis.fixture?.gpuErrors)')}; ${await evaluate('document.body?.innerText')}`);
    };
    await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
    await journey({ send, evaluate, pause, wait, responses });
    assert.deepEqual(failures, []);
  } catch (error) {
    console.error(diagnostics);
    throw error;
  } finally {
    for (const item of pending.values()) clearTimeout(item.timer);
    socket?.close(); clearTimeout(deadline);
    if (chrome.exitCode === null && chrome.signalCode === null) {
      const exited = once(chrome, 'exit'); chrome.kill('SIGTERM');
      const forced = setTimeout(() => chrome.kill('SIGKILL'), 3000); await exited; clearTimeout(forced);
    }
    await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
  }
}
