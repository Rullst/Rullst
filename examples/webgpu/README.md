# Browser graphics with Rullst

A small educational application: change the wavelength, source distance and
phase of two circular waves to explore interference. Use it as a starting point
for interactive lessons or scientific illustrations in a Rullst application.
Ordinary forms, billing and dashboards do not need GPU rendering.

From the repository root, with Rust 1.96.0 or newer:

```sh
cargo run --locked -p rullst-webgpu-example
```

Open **http://127.0.0.1:3007/webgpu/**. Choose **Start animation**; it starts paused.
No account, API key, Node build, database or GPU dependency is needed to run the
example. This workspace binary has `publish = false`; it is not an additional
framework package on crates.io.

## What to reuse

- `src/lib.rs`: exact embedded assets served with Rullst's production
  CSRF/WAF/secure-header baseline. The demo binds to loopback. Use your existing
  Rullst router and HTTPS deployment when adapting it to a hosted application.
- `gpu.mjs`: native browser WebGPU setup, a WGSL shader, bounded uniform data,
  device-loss handling and explicit disposal. No `wgpu`/Wasm toolchain is required.
- `waves.mjs`: the teaching model and a lower-resolution Canvas renderer.
- `controller.mjs`: user controls, a five-second GPU startup deadline, fallback,
  at most one submitted frame in flight, a 30 FPS ceiling and lifecycle cleanup.
- `app.mjs`: mounting on document load/partial navigation, removal cleanup and
  back/forward cache handling. The optional HTMX cleanup event requires no HTMX
  dependency. Existing hosts can import the controller instead of this bootstrap:

```js
import { mountWaveDemo } from './controller.mjs';
const lesson = mountWaveDemo(document.querySelector('[data-wave-demo]'));
await lesson.ready;
// Before replacing this lesson during navigation:
lesson.destroy();
```

Copy the paired HTML/CSS/modules and preserve their relative asset URLs.
Mounting the same root twice returns the existing instance. Destroying an
instance cancels its listeners/animation and releases its GPU resources; a late
GPU initialization cannot reactivate it. The controller fixes backing sizes at
960×600 for GPU and 160×100 for Canvas, regardless of screen pixel density.
The CSS resizes presentation. Hidden pages pause and require explicit restart.
Animation never autoplays, including when reduced motion is requested.

## Boundaries

[WebGPU requires a secure context and browser/device support](https://developer.mozilla.org/en-US/docs/Web/API/WebGPU_API).
Loopback is suitable for local development; hosted applications should use HTTPS.
Missing support, adapter failure or device loss selects Canvas, with a visible
status. Users can also select Canvas directly. The fallback uses a smaller image
to bound CPU work. If Canvas is unavailable, the explanatory lesson remains.

This model sums two circular sine waves in relative units. It omits attenuation,
reflection and physical calibration; color represents signed displacement.
It is an illustration, not a scientific solver or performance benchmark.

All computation stays in the browser. The example loads no remote libraries,
collects no adapter details and requests no camera/microphone permission. The
server serves public read-only content with same-origin scripts and styles.
Add application authentication/tenant checks if lessons are private. Scores,
credentials, billing or authorization must remain server-owned; browser output
cannot establish that a learner completed an exercise.

## Verification

```sh
cargo test --locked -p rullst-webgpu-example
RULLST_WEBGPU_BROWSER_TESTS=1 cargo test --locked -p rullst-webgpu-example
```

The second command also requires Node 24 and installed `google-chrome`
(`CHROME_BIN` can select its executable). CI enables it in the Linux workspace
shard. The browser test compiles/renders the real WGSL through Chromium's
software GPU, compares pixels with Canvas, and checks fallback, controls,
navigation, disposal, CSP and absence of external page requests. Test-only GPU
flags belong to the disposable browser fixture, not production browser settings.

For frontend-only verification without a Rust build:

```sh
node .github/webgpu-browser-smoke.mjs --static
```

That mode serves the exact assets through a temporary Node loopback fixture. It
does not establish the Rust HTTP integration. Software-GPU tests do not establish
physical-device performance, power consumption or compatibility with all browsers.
