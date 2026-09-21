// Real Auth/PostgreSQL HTTP fixture with a Chromium virtual authenticator.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
let input = '';
for await (const chunk of process.stdin) { input += chunk; assert(Buffer.byteLength(input) <= 16384); }
const { origin, cookie } = JSON.parse(input);
assert(/^http:\/\/localhost:\d+$/.test(origin));
const profile = await mkdtemp(join(tmpdir(), 'rullst-passkey-browser-'));
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
  await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
  await send('WebAuthn.enable');
  const { authenticatorId } = await send('WebAuthn.addVirtualAuthenticator', {options:{protocol:'ctap2',transport:'internal',hasResidentKey:true,hasUserVerification:true,isUserVerified:true,automaticPresenceSimulation:true}});
  const requests = [];
  handlers.set('Runtime.exceptionThrown', () => failures.push('uncaught browser exception'));
  handlers.set('Network.requestWillBeSent', params => {
    if (!params.request.url.startsWith(origin + '/')) failures.push('non-origin request');
    if (params.request.method === 'POST') requests.push(new URL(params.request.url).pathname);
  });
  await send('Network.setCookie', {name:'passkey_fixture',value:cookie,url:origin,httpOnly:true,sameSite:'Lax'});
  await send('Page.navigate', {url:origin + '/'});
  await waitFor(() => evaluate("document.getElementById('status')?.textContent === 'ready'"), 'fixture ready');
  assert.equal(await evaluate("fetch('/a/register',{method:'POST',credentials:'omit',headers:{'x-csrf-token':csrf}}).then(r=>r.status)"),401);
  assert.equal(await evaluate("fetch('/a/register',{method:'POST',headers:{'x-csrf-token':'wrong'}}).then(r=>r.status)"),401);
  const press = async id => {
    await evaluate(`document.getElementById(${JSON.stringify(id)}).focus(); true`);
    await send('Input.dispatchKeyEvent', {type:'keyDown',key:' ',code:'Space',windowsVirtualKeyCode:32});
    await send('Input.dispatchKeyEvent', {type:'keyUp',key:' ',code:'Space',windowsVirtualKeyCode:32});
  };
  const state = label => waitFor(() => evaluate(`document.getElementById('status')?.textContent === ${JSON.stringify(label)}`), label);
  await press('register'); await state('registered');
  await press('authenticate'); await state('authenticated');
  await press('replay'); await state('replay-rejected');
  const { credentials } = await send('WebAuthn.getCredentials', {authenticatorId});
  assert.equal(credentials.length,1); assert(credentials[0].signCount > 0);
  for (const route of ['/a/register','/b/register','/a/authenticate','/b/authenticate']) assert(requests.includes(route));
  assert.deepEqual(failures,[]);
  console.log('Passkey browser: virtual ES256 registration/assertion across independent managers, authenticated HTTP/CSRF and replay rejection passed');
  await call('Browser.close');
} finally {
  clearTimeout(deadline); socket?.close();
  if (chrome.exitCode === null) { const stopped = once(chrome, 'exit').catch(() => {}); chrome.kill('SIGKILL'); await stopped; }
  await rm(profile, {recursive:true,force:true,maxRetries:5,retryDelay:100});
}
