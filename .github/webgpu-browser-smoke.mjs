import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, writeFile } from 'node:fs/promises';
import { browser } from './webgpu-cdp.mjs';
import { bounded, defaults, displacement } from '../examples/webgpu/waves.mjs';

assert.equal(bounded('wavelength', -100), 0.15);
assert.equal(bounded('phase', 999), 180);
assert.equal(bounded('separation', NaN), defaults.separation);
assert.throws(() => bounded('unknown', 1));
assert.throws(() => bounded('__proto__', 1));
for (const time of [0, 0.2, 0.7]) {
  assert(Math.abs(displacement(0, 0.3, time, { ...defaults, phase: 180 })) < 1e-12, 'opposite waves cancel at equal distance');
  assert(Math.abs(displacement(-0.8, 0.4, time, defaults) - displacement(0.8, 0.4, time, defaults)) < 1e-12, 'in-phase sources are symmetric');
}

// --static runs only the frontend offline. CI passes the real Rust server origin.
let server, origin = process.argv[2];
if (origin === '--static') {
  const assets = new Map();
  for (const name of ['index.html', 'style.css', 'app.mjs', 'controller.mjs', 'waves.mjs', 'gpu.mjs']) {
    assets.set(`/webgpu/${name === 'index.html' ? '' : name}`, {
      bytes: await readFile(new URL(`../examples/webgpu/${name}`, import.meta.url)),
      type: name.endsWith('.mjs') ? 'text/javascript' : name.endsWith('.css') ? 'text/css' : 'text/html',
    });
  }
  server = createServer((request, response) => {
    const asset = assets.get(request.url);
    response.writeHead(asset ? 200 : 404, {
      'Content-Type': asset?.type || 'text/plain', 'X-Content-Type-Options': 'nosniff',
      'Content-Security-Policy': "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; object-src 'none'; frame-ancestors 'none'",
    });
    response.end(asset?.bytes || 'Not found');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  origin = `http://127.0.0.1:${server.address().port}`;
}
assert(/^http:\/\/127\.0\.0\.1:\d+$/.test(origin));
try {
  await browser(origin, async ({ send, evaluate, pause, wait, responses }) => {
    const instrumentation = `
      globalThis.fixture = { devices: [], destroyed: 0, submits: 0, pixels: 0, violations: [], gpuErrors: [] };
      addEventListener('securitypolicyviolation', e => fixture.violations.push(e.violatedDirective));
      if (globalThis.GPUAdapter) {
        const request = GPUAdapter.prototype.requestDevice;
        GPUAdapter.prototype.requestDevice = async function(...args) {
          const device = await request.apply(this, args); fixture.devices.push(device);
          device.addEventListener('uncapturederror', event => fixture.gpuErrors.push(event.error.message));
          device.lost.then(info => fixture.gpuErrors.push('lost: ' + info.reason + ': ' + info.message));
          return device;
        };
        const destroy = GPUDevice.prototype.destroy;
        GPUDevice.prototype.destroy = function() { fixture.destroyed++; return destroy.call(this); };
        const submit = GPUQueue.prototype.submit;
        GPUQueue.prototype.submit = function(...args) { fixture.submits++; return submit.apply(this, args); };
        const completed = GPUQueue.prototype.onSubmittedWorkDone;
        GPUQueue.prototype.onSubmittedWorkDone = async function() {
          await completed.call(this);
          if (fixture.holdFrame) await new Promise(resolve => fixture.releaseFrame = resolve);
        };
      }
      const pixels = CanvasRenderingContext2D.prototype.putImageData;
      CanvasRenderingContext2D.prototype.putImageData = function(...args) { fixture.pixels++; return pixels.apply(this, args); };
    `;
    await send('Page.addScriptToEvaluateOnNewDocument', { source: instrumentation });
    await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
    const navigate = () => send('Page.navigate', { url: origin + '/webgpu/' });
    const status = text => wait(`document.querySelector('[data-status]')?.textContent.includes(${JSON.stringify(text)})`, text);
    const click = action => evaluate(`document.querySelector('[data-action="${action}"]').click()`);
    const select = value => evaluate(`{
      const mode = document.querySelector('[data-control="renderer"]');
      mode.value = '${value}'; mode.dispatchEvent(new Event('change'));
    }`);
    await navigate(); await status('WebGPU · paused'); await wait('fixture.submits > 0', 'actual GPU draw');
    assert.deepEqual(await evaluate(`Array.from(document.querySelectorAll('canvas'), c => [c.width,c.height])`), [[160, 100], [960, 600]]);
    const first = await evaluate('fixture.submits'); await pause(150);
    assert.equal(await evaluate('fixture.submits'), first, 'starts paused with reduced motion');
    if (process.env.RULLST_WEBGPU_SCREENSHOT) {
      const screenshot = await send('Page.captureScreenshot', { format: 'png' });
      await writeFile(process.env.RULLST_WEBGPU_SCREENSHOT, Buffer.from(screenshot.data, 'base64'));
    }
    await send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 1, mobile: false });
    assert(await evaluate('document.documentElement.scrollWidth <= innerWidth'), 'mobile layout has no horizontal overflow');
    await send('Emulation.clearDeviceMetricsOverride');

    // Compare actual compiled WGSL output with the analytic Canvas model, including cancellation.
    const comparison = await evaluate(`(async () => {
      const { createGpuRenderer } = await import('./gpu.mjs');
      const { createCanvasRenderer, defaults } = await import('./waves.mjs');
      const gpu = document.createElement('canvas'), cpu = document.createElement('canvas'), copy = document.createElement('canvas');
      for (const canvas of [gpu,cpu,copy]) { canvas.width=80; canvas.height=50; }
      const graphics = await createGpuRenderer(gpu, new AbortController().signal), software = createCanvasRenderer(cpu);
      const context = copy.getContext('2d'); let error = 0, min = 255, max = 0;
      try {
        for (const phase of [0,180]) {
          const parameters={...defaults,phase};
          await software.draw(0.137,parameters); await graphics.draw(0.137,parameters);
          context.drawImage(gpu,0,0);
          const a=context.getImageData(0,0,80,50).data,b=cpu.getContext('2d').getImageData(0,0,80,50).data;
          for(let i=0;i<a.length;i++) error=Math.max(error,Math.abs(a[i]-b[i]));
          for(let i=1;i<a.length;i+=4) {min=Math.min(min,a[i]);max=Math.max(max,a[i]);}
        }
        return {error,range:max-min};
      } finally {graphics.dispose();software.dispose();}
    })()`);
    assert(comparison.error <= 2 && comparison.range > 100, `shader/Canvas parity: ${JSON.stringify(comparison)}`);
    await evaluate("fixture.holdFrame=true; document.querySelector('[data-action=play]').focus()");
    await send('Input.dispatchKeyEvent', { type: 'keyDown', key: ' ', code: 'Space', windowsVirtualKeyCode: 32 });
    await send('Input.dispatchKeyEvent', { type: 'keyUp', key: ' ', code: 'Space', windowsVirtualKeyCode: 32 });
    await wait("typeof fixture.releaseFrame === 'function'", 'submitted frame is in flight');
    const started = await evaluate('fixture.submits'); await pause(150);
    assert.equal(await evaluate('fixture.submits'), started, 'only one GPU frame in flight');
    await evaluate('fixture.holdFrame=false;fixture.releaseFrame()');
    await wait(`fixture.submits > ${started + 2}`, 'keyboard starts animation');
    await click('play'); await pause(100);
    const paused = await evaluate('fixture.submits'); await pause(150);
    assert.equal(await evaluate('fixture.submits'), paused, 'pause stops submissions');
    await evaluate('fixture.devices[0].destroy()'); await status('Canvas compatibility mode');
    await wait('fixture.pixels > 2', 'device loss renders Canvas');
    const beforeInput = await evaluate("document.querySelector('[data-canvas=cpu]').toDataURL()");
    await evaluate("const phase=document.querySelector('[data-control=phase]'); phase.value=180; phase.dispatchEvent(new Event('input'))");
    await wait(`document.querySelector('[data-canvas=cpu]').toDataURL() !== ${JSON.stringify(beforeInput)}`, 'control changes pixels');
    await click('play');
    await evaluate("Object.defineProperty(document,'hidden',{configurable:true,value:true});document.dispatchEvent(new Event('visibilitychange'))");
    await status('paused'); await pause(100);
    const hidden = await evaluate('fixture.pixels'); await pause(150);
    assert.equal(await evaluate('fixture.pixels'), hidden, 'hidden page stops frames');
    await evaluate("delete document.hidden;document.dispatchEvent(new Event('visibilitychange'))");
    await click('reset');
    assert.equal(await evaluate("document.querySelector('[data-control=phase]').value"), '0');
    await select('auto'); await status('WebGPU · paused');
    const removed = await evaluate('fixture.destroyed');
    await evaluate("globalThis.lesson=document.querySelector('[data-wave-demo]');lesson.remove()");
    await wait(`fixture.destroyed === ${removed + 1}`, 'removed lesson releases device');
    const stopped = await evaluate('fixture.submits'); await pause(150);
    assert.equal(await evaluate('fixture.submits'), stopped);
    await evaluate('document.body.prepend(lesson)'); await status('WebGPU · paused');
    assert(await evaluate("import('./controller.mjs').then(({mountWaveDemo}) => mountWaveDemo(lesson) === mountWaveDemo(lesson))"));
    await evaluate("dispatchEvent(new PageTransitionEvent('pagehide'))"); await status('Visualization stopped');
    await evaluate("dispatchEvent(new PageTransitionEvent('pageshow',{persisted:true}))"); await status('WebGPU · paused');
    assert.deepEqual(await evaluate('fixture.violations'), []);

    // Unavailable and rejected adapters must both retain an interactive fallback.
    for (const source of [
      "Object.defineProperty(navigator,'gpu',{value:undefined})",
      "navigator.gpu.requestAdapter = async () => {throw new Error('fixture adapter rejection')}",
      "navigator.gpu.requestAdapter = () => new Promise(() => {})",
    ]) {
      const { identifier } = await send('Page.addScriptToEvaluateOnNewDocument', { source });
      await navigate(); await status('Canvas compatibility mode'); await wait('fixture.pixels > 0', 'fallback draws');
      await click('play'); const pixels = await evaluate('fixture.pixels');
      await wait(`fixture.pixels > ${pixels + 2}`, 'fallback animates');
      assert.equal(await evaluate('fixture.devices.length'), 0);
      await send('Page.removeScriptToEvaluateOnNewDocument', { identifier });
    }
    // A device delivered after cancellation is destroyed without reviving the old selection.
    const { identifier } = await send('Page.addScriptToEvaluateOnNewDocument', { source: `
      const original = GPUAdapter.prototype.requestDevice;
      GPUAdapter.prototype.requestDevice = async function(...args) {
        const device = await original.apply(this,args);
        await new Promise(resolve => globalThis.deliverDevice=resolve); return device;
      };
    ` });
    await navigate(); await wait("typeof deliverDevice === 'function'", 'delayed device');
    await select('canvas'); await status('Canvas compatibility mode');
    await evaluate('deliverDevice()'); await wait('fixture.destroyed === 1', 'late device released');
    assert.equal(await evaluate('fixture.submits'), 0);
    assert.deepEqual(await evaluate('fixture.violations'), []);
    await send('Page.removeScriptToEvaluateOnNewDocument', { identifier });
    const response = responses.find(item => item.url === origin + '/webgpu/');
    const headers = Object.fromEntries(Object.entries(response.headers).map(([k,v]) => [k.toLowerCase(),v]));
    assert(headers['content-security-policy'].includes("script-src 'self'"));
    assert(!headers['content-security-policy'].includes('unsafe-inline'));
    assert.equal(headers['x-content-type-options'], 'nosniff');
    console.log('WebGPU example passed: real software-GPU shader/Canvas parity, controls, pause, loss/failure fallback, lifecycle cleanup and CSP.');
  });
} finally {
  if (server) await new Promise(resolve => server.close(resolve));
}
