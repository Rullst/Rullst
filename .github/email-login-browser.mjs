// Owned HTTP email-login fixture; no provider accounts or external browser requests.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
let input = '';
for await (const chunk of process.stdin) { input += chunk; assert(Buffer.byteLength(input) <= 16384); }
const { origin, escaping } = JSON.parse(input);
assert(/^http:\/\/localhost:\d+$/.test(origin));
const profile = await mkdtemp(join(tmpdir(), 'rullst-email-login-browser-'));
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
  const requests = [];
  handlers.set('Runtime.exceptionThrown', () => failures.push('uncaught browser exception'));
  handlers.set('Network.requestWillBeSent', params => {
    if (!params.request.url.startsWith(origin + '/')) failures.push('non-origin request');
    if (params.request.method === 'POST') requests.push(new URL(params.request.url).pathname);
  });
  await send('Page.navigate', {url:origin + '/'});
  await waitFor(() => evaluate("!!document.getElementById('request')"), 'request form');
  // Parse the actual Rust-rendered pages: hostile attribute values stay data.
  // DOMParser keeps this adversarial probe inert; assertions inspect structure.
  for (const html of escaping.pages) {
    const parsed = await evaluate(`(() => {
      const doc = new DOMParser().parseFromString(${JSON.stringify(html)}, 'text/html');
      return {values: [...doc.querySelectorAll('input')].map(input => input.value),
        forms: doc.querySelectorAll('form').length,
        injected: doc.querySelectorAll('script,img,svg,iframe,[onerror],[onfocus]').length};
    })()`);
    assert.deepEqual(parsed, {values: [escaping.payload, escaping.payload], forms: 1, injected: 0});
  }
  const press = async id => {
    await evaluate(`document.getElementById(${JSON.stringify(id)}).focus(); true`);
    await send('Input.dispatchKeyEvent', {type:'keyDown',key:' ',code:'Space',windowsVirtualKeyCode:32});
    await send('Input.dispatchKeyEvent', {type:'keyUp',key:' ',code:'Space',windowsVirtualKeyCode:32});
  };
  assert(!(await evaluate('document.cookie')).includes('login_browser='));
  const {cookies: initialCookies} = await send('Network.getCookies', {urls:[origin]});
  const binding = initialCookies.find(cookie => cookie.name === 'login_browser');
  assert(binding && binding.httpOnly && binding.secure && binding.sameSite === 'Lax');
  assert.equal(await evaluate("fetch('/request',{method:'POST'}).then(r=>r.status)"),403);
  await press('request');
  await waitFor(() => evaluate("!!document.getElementById('accepted')"), 'uniform acknowledgement');
  const {link} = await evaluate("fetch('/fixture/mail').then(r=>r.json())");
  assert(link.startsWith(origin + '/login?token='));
  // A provider prefetch sees no browser cookie and must not consume the link.
  for (const method of ['HEAD','GET']) {
    const response = await fetch(link,{method,headers:{'User-Agent':'Mozilla/5.0'}});
    assert.equal(response.status,200);
    assert.equal(response.headers.get('cache-control'),'no-store');
    assert.equal(response.headers.get('referrer-policy'),'no-referrer');
    assert(!response.headers.get('set-cookie')?.includes('session='));
  }
  await send('Page.navigate',{url:link});
  await waitFor(() => evaluate("!!document.getElementById('confirm')"), 'confirmation form');
  assert.equal(await evaluate("fetch('/dashboard').then(r=>r.status)"),401);
  const csrf = await evaluate("document.querySelector('[name=_token]').value");
  const token = new URL(link).searchParams.get('token');
  const attempt = (headers, credentials='same-origin') => evaluate(`fetch('/consume',{method:'POST',credentials:${JSON.stringify(credentials)},headers:${JSON.stringify(headers)},body:${JSON.stringify(new URLSearchParams({token}).toString())}}).then(r=>r.status)`);
  assert.equal(await attempt({'content-type':'application/x-www-form-urlencoded'}),403);
  assert.equal(await attempt({'content-type':'application/x-www-form-urlencoded','x-csrf-token':'wrong'}),403);
  // Correct CSRF plus no browser binding is still insufficient.
  await send('Network.deleteCookies',{name:'login_browser',url:origin});
  assert.equal(await attempt({'content-type':'application/x-www-form-urlencoded','x-csrf-token':csrf}),401);
  await send('Network.setCookie',{name:'login_browser',value:binding.value,url:origin,httpOnly:true,secure:true,sameSite:'Lax'});
  await press('confirm');
  await waitFor(() => evaluate("location.pathname === '/dashboard' && !!document.getElementById('authenticated')"), 'authenticated after explicit POST');
  const {cookies} = await send('Network.getCookies',{urls:[origin]});
  const session = cookies.find(cookie=>cookie.name==='session');
  assert(session && session.secure && session.httpOnly && session.sameSite==='Lax');
  assert(!cookies.some(cookie=>cookie.name==='login_browser'));
  assert(!(await evaluate('document.cookie')).includes('session='));
  assert.equal(await evaluate("fetch('/tenants/school-a').then(r=>r.status)"),204);
  assert.equal(await evaluate("fetch('/tenants/school-b').then(r=>r.status)"),403);
  await send('Network.setCookie',{name:'login_browser',value:binding.value,url:origin,httpOnly:true,secure:true,sameSite:'Lax'});
  assert.equal(await attempt({'content-type':'application/x-www-form-urlencoded','x-csrf-token':csrf}),401);
  assert.equal(await evaluate(`fetch('/logout',{method:'POST',headers:{'x-csrf-token':${JSON.stringify(csrf)}}}).then(r=>r.status)`),204);
  assert.equal(await evaluate("fetch('/dashboard').then(r=>r.status)"),401);
  assert(requests.includes('/request') && requests.includes('/consume') && requests.includes('/logout'));
  assert.deepEqual(failures,[]);
  console.log('Email login browser: attribute escaping, prefetch-safe GET/HEAD, explicit CSRF POST, browser binding, secure opaque session, tenant isolation, replay and logout passed');
  await call('Browser.close');
} finally {
  clearTimeout(deadline); socket?.close();
  if (chrome.exitCode === null) { const stopped = once(chrome, 'exit').catch(() => {}); chrome.kill('SIGKILL'); await stopped; }
  await rm(profile, {recursive:true,force:true,maxRetries:5,retryDelay:100});
}
