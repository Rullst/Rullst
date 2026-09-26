import { mountWaveDemo } from './controller.mjs';

const instances = new Map();
function reconcile() {
  for (const [root, instance] of instances) {
    if (!root.isConnected) { instance.destroy(); instances.delete(root); }
  }
  for (const root of document.querySelectorAll('[data-wave-demo]')) {
    if (!instances.has(root)) instances.set(root, mountWaveDemo(root));
  }
}
const observer = new MutationObserver(reconcile);
function start() { reconcile(); observer.observe(document.body, { childList: true, subtree: true }); }
function stop() {
  observer.disconnect();
  for (const instance of instances.values()) instance.destroy();
  instances.clear();
}
// Also works with back/forward cache and optional HTMX partial navigation.
addEventListener('pagehide', stop);
addEventListener('pageshow', start);
document.addEventListener('htmx:beforeCleanupElement', event => {
  const element = event.detail?.elt;
  for (const [root, instance] of instances) {
    if (element === root || element?.contains(root)) { instance.destroy(); instances.delete(root); }
  }
});
start();
