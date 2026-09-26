export const defaults = Object.freeze({ wavelength: 0.45, separation: 0.70, phase: 0 });
const ranges = Object.freeze({ wavelength: [0.15, 1.2], separation: [0.15, 1.4], phase: [0, 180] });

export function bounded(name, value) {
  if (!Object.hasOwn(ranges, name)) throw new TypeError('Unknown wave parameter');
  const range = ranges[name];
  const number = Number(value);
  return Number.isFinite(number) ? Math.max(range[0], Math.min(range[1], number)) : defaults[name];
}

// This analytic teaching model deliberately omits attenuation and reflections.
export function displacement(x, y, time, { wavelength, separation, phase }) {
  const a = Math.hypot(x + separation / 2, y);
  const b = Math.hypot(x - separation / 2, y);
  const tau = 2 * Math.PI;
  return (Math.sin(tau * (a / wavelength - time))
    + Math.sin(tau * (b / wavelength - time) + phase * Math.PI / 180)) / 2;
}

export function createCanvasRenderer(canvas) {
  const context = canvas.getContext('2d', { alpha: false });
  if (!context) throw new Error('Canvas rendering is unavailable');
  const pixels = context.createImageData(canvas.width, canvas.height);
  let disposed = false;
  return {
    kind: 'canvas',
    async draw(time, parameters) {
      if (disposed) return;
      const width = canvas.width, height = canvas.height;
      for (let j = 0; j < height; j++) {
        for (let i = 0; i < width; i++) {
          const x = ((i + 0.5) / width * 2 - 1) * 1.6;
          const y = (j + 0.5) / height * 2 - 1;
          const value = (displacement(x, y, time, parameters) + 1) / 2;
          const dot = Math.min(Math.hypot(x + parameters.separation / 2, y),
            Math.hypot(x - parameters.separation / 2, y)) < 0.025;
          const offset = (j * width + i) * 4;
          pixels.data[offset] = dot ? 255 : (0.10 + 0.18 * value) * 255;
          pixels.data[offset + 1] = dot ? 243 : (0.20 + 0.67 * value) * 255;
          pixels.data[offset + 2] = dot ? 217 : (0.34 + 0.44 * value) * 255;
          pixels.data[offset + 3] = 255;
        }
      }
      context.putImageData(pixels, 0, 0);
    },
    dispose() { disposed = true; },
  };
}
