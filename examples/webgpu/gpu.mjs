// No fingerprinting/adapter details are collected or transmitted.
const shader = `
struct Parameters { time: f32, wavelength: f32, separation: f32, phase: f32 }
@group(0) @binding(0) var<uniform> p: Parameters;
struct Vertex { @builtin(position) position: vec4f, @location(0) uv: vec2f }
@vertex fn vertex(@builtin(vertex_index) index: u32) -> Vertex {
  let positions = array(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
  var out: Vertex;
  out.position = vec4f(positions[index], 0, 1);
  out.uv = positions[index];
  return out;
}
@fragment fn fragment(in: Vertex) -> @location(0) vec4f {
  let point = vec2f(in.uv.x * 1.6, in.uv.y);
  let a = distance(point, vec2f(-p.separation / 2, 0));
  let b = distance(point, vec2f(p.separation / 2, 0));
  let tau = 6.28318530718;
  let wave = (sin(tau * (a / p.wavelength - p.time))
    + sin(tau * (b / p.wavelength - p.time) + p.phase)) / 2;
  if (min(a, b) < 0.025) { return vec4f(1, 0.95294, 0.85098, 1); }
  return vec4f(mix(vec3f(0.10, 0.20, 0.34), vec3f(0.28, 0.87, 0.78), (wave + 1) / 2), 1);
}`;

export async function createGpuRenderer(canvas, signal) {
  let device, context, buffer, configured = false, disposed = false;
  const dispose = () => {
    if (disposed) return;
    disposed = true;
    if (configured) context.unconfigure();
    buffer?.destroy();
    device?.destroy();
  };
  try {
    signal.throwIfAborted();
    if (!globalThis.isSecureContext || !navigator.gpu) throw new Error('WebGPU unavailable');
    const adapter = await navigator.gpu.requestAdapter({ powerPreference: 'low-power' });
    signal.throwIfAborted();
    if (!adapter) throw new Error('No WebGPU adapter');
    device = await adapter.requestDevice();
    signal.throwIfAborted();
    context = canvas.getContext('webgpu');
    if (!context) throw new Error('No WebGPU canvas');
    const format = navigator.gpu.getPreferredCanvasFormat();
    device.pushErrorScope('validation');
    const module = device.createShaderModule({ code: shader });
    const pipeline = await device.createRenderPipelineAsync({
      layout: 'auto', vertex: { module, entryPoint: 'vertex' },
      fragment: { module, entryPoint: 'fragment', targets: [{ format }] },
      primitive: { topology: 'triangle-list' },
    });
    signal.throwIfAborted();
    buffer = device.createBuffer({ size: 16, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
    const bindings = device.createBindGroup({ layout: pipeline.getBindGroupLayout(0),
      entries: [{ binding: 0, resource: { buffer } }] });
    const error = await device.popErrorScope();
    signal.throwIfAborted();
    if (error) throw new Error('WebGPU pipeline validation failed');
    context.configure({ device, format, alphaMode: 'opaque' });
    configured = true;
    let failure = false;
    device.addEventListener('uncapturederror', () => { failure = true; });
    const lost = device.lost.then(() => { failure = true; });
    return {
      kind: 'webgpu', lost, dispose,
      async draw(time, parameters) {
        if (disposed || failure) throw new Error('WebGPU device unavailable');
        device.queue.writeBuffer(buffer, 0, new Float32Array([
          time, parameters.wavelength, parameters.separation, parameters.phase * Math.PI / 180,
        ]));
        const encoder = device.createCommandEncoder();
        const pass = encoder.beginRenderPass({ colorAttachments: [{
          view: context.getCurrentTexture().createView(), clearValue: { r: 0, g: 0, b: 0, a: 1 },
          loadOp: 'clear', storeOp: 'store',
        }] });
        pass.setPipeline(pipeline); pass.setBindGroup(0, bindings); pass.draw(3); pass.end();
        device.queue.submit([encoder.finish()]);
        // Backpressure: the controller never queues another frame before this completes.
        await device.queue.onSubmittedWorkDone();
        if (failure) throw new Error('WebGPU device unavailable');
      },
    };
  } catch (error) { dispose(); throw error; }
}
