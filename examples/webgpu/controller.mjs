import { bounded, createCanvasRenderer, defaults } from './waves.mjs';
import { createGpuRenderer } from './gpu.mjs';

const mounted = new WeakMap();

// Call destroy before removing a lesson during partial navigation.
export function mountWaveDemo(root) {
  if (mounted.has(root)) return mounted.get(root);
  const required = selector => {
    const element = root.querySelector(selector);
    if (!element) throw new Error(`Missing wave demo element: ${selector}`);
    return element;
  };
  const cpu = required('[data-canvas="cpu"]'), gpu = required('[data-canvas="gpu"]');
  const play = required('[data-action="play"]'), reset = required('[data-action="reset"]');
  const status = required('[data-status]'), mode = required('[data-control="renderer"]');
  // Fixed backing sizes bound resource use independently of viewport/device pixel ratio.
  cpu.width = 160; cpu.height = 100; gpu.width = 960; gpu.height = 600;
  const parameters = { ...defaults }, listeners = new AbortController();
  let renderer, startup, generation = 0, destroyed = false, running = false;
  let frame = 0, drawing = false, dirty = true, time = 0, previous = 0, note = '';

  function describe() {
    play.disabled = reset.disabled = !renderer || destroyed;
    play.textContent = running ? 'Pause animation' : 'Start animation';
    play.setAttribute('aria-pressed', String(running));
    if (destroyed) status.textContent = 'Visualization stopped.';
    else if (renderer) status.textContent = `${renderer.kind === 'webgpu' ? 'WebGPU' : 'Canvas compatibility mode'} · ${running ? 'running' : 'paused'}. ${note}`;
  }
  function pause() { running = false; previous = 0; describe(); }
  function schedule() {
    if (destroyed || drawing || frame || !renderer || document.hidden) return;
    if (dirty || running) frame = requestAnimationFrame(draw);
  }
  function useCanvas(reason) {
    renderer?.dispose(); renderer = undefined;
    gpu.hidden = true; cpu.hidden = false; note = reason;
    try { renderer = createCanvasRenderer(cpu); }
    catch { pause(); status.textContent = 'Visualization unavailable. The lesson below remains readable.'; return; }
    dirty = true; describe(); schedule();
  }
  async function draw(timestamp) {
    frame = 0;
    if (destroyed) return;
    if (!root.isConnected) { destroy(); return; }
    if (document.hidden) { pause(); return; }
    if (!renderer || drawing) return;
    if (running && previous && timestamp - previous < 1000 / 30) { schedule(); return; }
    if (running && previous) time = (time + Math.min(timestamp - previous, 100) / 1000) % 1;
    previous = timestamp;
    const active = renderer;
    dirty = false; drawing = true;
    try { await active.draw(time, parameters); }
    catch {
      if (!destroyed && renderer === active) {
        if (active.kind === 'webgpu') useCanvas('WebGPU stopped; the lesson continues in Canvas.');
        else {
          renderer.dispose(); renderer = undefined; pause();
          status.textContent = 'Visualization unavailable. The lesson below remains readable.';
        }
      }
    } finally {
      drawing = false;
      if (renderer !== active) dirty = true;
      schedule();
    }
  }
  async function selectRenderer() {
    const selected = ++generation;
    startup?.abort(); renderer?.dispose(); renderer = undefined;
    pause(); status.textContent = 'Preparing the visualization…';
    if (mode.value === 'canvas') { useCanvas(''); return; }
    const attempt = new AbortController(); startup = attempt;
    let timer;
    try {
      const deadline = new Promise((_, reject) => {
        timer = setTimeout(() => { attempt.abort(); reject(new Error('GPU startup timeout')); }, 5000);
      });
      const candidate = await Promise.race([createGpuRenderer(gpu, attempt.signal), deadline]);
      if (destroyed || selected !== generation || attempt.signal.aborted) { candidate.dispose(); return; }
      renderer = candidate; cpu.hidden = true; gpu.hidden = false; note = '';
      candidate.lost.then(() => {
        if (!destroyed && renderer === candidate) useCanvas('WebGPU stopped; the lesson continues in Canvas.');
      });
      dirty = true; describe(); schedule();
    } catch {
      if (!destroyed && selected === generation) useCanvas('WebGPU could not start on this browser.');
    } finally { clearTimeout(timer); }
  }
  for (const name of Object.keys(defaults)) {
    const input = required(`[data-control="${name}"]`), output = required(`[data-value="${name}"]`);
    input.value = parameters[name];
    output.value = name === 'phase' ? `${parameters[name]}°` : parameters[name].toFixed(2);
    input.addEventListener('input', () => {
      parameters[name] = bounded(name, input.value);
      output.value = name === 'phase' ? `${parameters[name]}°` : parameters[name].toFixed(2);
      dirty = true; schedule();
    }, { signal: listeners.signal });
  }
  mode.addEventListener('change', selectRenderer, { signal: listeners.signal });
  play.addEventListener('click', () => {
    if (!renderer || document.hidden) return;
    running = !running; previous = 0; describe(); schedule();
  }, { signal: listeners.signal });
  reset.addEventListener('click', () => {
    pause(); time = 0;
    for (const name of Object.keys(defaults)) {
      const input = required(`[data-control="${name}"]`);
      input.value = defaults[name]; input.dispatchEvent(new Event('input'));
    }
  }, { signal: listeners.signal });
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) pause(); else schedule();
  }, { signal: listeners.signal });
  function destroy() {
    if (destroyed) return;
    destroyed = true; running = false; generation++;
    cancelAnimationFrame(frame); listeners.abort(); startup?.abort(); renderer?.dispose();
    renderer = undefined; mounted.delete(root); describe();
  }
  const api = { ready: selectRenderer(), destroy };
  mounted.set(root, api);
  return api;
}
