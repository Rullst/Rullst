// Run with: node rullst-core/src/server/dev_reload/client_tests.cjs
// Executes the actual embedded client; no browser/network or third-party package.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const source = fs.readFileSync(path.join(__dirname, '../dev_reload.rs'), 'utf8');
const script = source.match(/const SCRIPT: &str = r#"([\s\S]*?)"#;/)[1];

async function main() {
  let marker = 'a'.repeat(32);
  let reloads = 0;
  let sequence = 0;
  const scheduled = new Map();
  const context = vm.createContext({
    document: { currentScript: { dataset: { generation: marker } } },
    window: { location: { reload() { reloads += 1; } } },
    AbortController,
    setTimeout(callback, delay) { const id = ++sequence; scheduled.set(id, { callback, delay }); return id; },
    clearTimeout(id) { scheduled.delete(id); },
    async fetch(url, options) {
      assert.equal(url, '/_rullst/dev-generation');
      assert.equal(options.credentials, 'same-origin');
      assert.equal(options.cache, 'no-store');
      return { ok: true, headers: { get() { return marker; } } };
    },
  });
  vm.runInContext(script, context);
  vm.runInContext(script, context);
  assert.equal(scheduled.size, 1, 'repeated script execution must retain one poller');
  async function poll() {
    const [id, timer] = scheduled.entries().next().value;
    scheduled.delete(id);
    await timer.callback();
  }
  await poll();
  assert.equal(reloads, 0);
  assert.equal(scheduled.size, 1);
  marker = 'invalid-marker';
  await poll();
  assert.equal(reloads, 0);
  marker = 'b'.repeat(32);
  await poll();
  assert.equal(reloads, 1);
  assert.equal(scheduled.size, 0, 'refresh must not also schedule another poll');
  console.log('dev-reload client: singleton, unchanged/invalid generation, refresh and timer cleanup passed');
}
main().catch(error => { console.error(error); process.exitCode = 1; });
