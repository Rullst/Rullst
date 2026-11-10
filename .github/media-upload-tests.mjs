// Deterministic fault contracts for the shipped client, no network/npm dependency.
import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
const { BunnyUpload } = await import(process.env.RULLST_MEDIA_PACKAGE_MODULE
  ? pathToFileURL(process.env.RULLST_MEDIA_PACKAGE_MODULE).href
  : new URL('../rullst-media/web/bunny-upload.mjs', import.meta.url).href);

const video = '12345678-1234-4234-8234-123456789abc';
const blob = new Blob([new Uint8Array(1500000).fill(17)], { type: 'video/mp4' });
const makeGrant = () => ({ endpoint: 'http://127.0.0.1:1234/tusupload', library: 7, video,
  mode: 'ProtocolFixture', expires_at: Math.floor(Date.now() / 1000) + 300, signature: 'a'.repeat(64) });
const reply = (status, headers = {}) => new Response(null, { status, headers: { 'Tus-Resumable': '1.0.0', ...headers } });

function fixture(options = {}) {
  let offset = 0; let patches = 0; let posts = 0;
  const calls = [];
  const fetchImpl = async (url, init) => {
    calls.push(init.method);
    assert.equal(init.credentials, 'omit'); assert.equal(init.redirect, 'error'); assert.equal(init.referrerPolicy, 'no-referrer');
    assert(!('AccessKey' in init.headers)); assert.equal(init.headers.VideoId, video);
    if (init.method === 'POST') {
      posts++; assert.equal(init.headers['Upload-Length'], String(blob.size));
      if (options.uncertain) throw new Error('response lost');
      return reply(201, { Location: options.location ?? '/tusupload/owned-upload' });
    }
    assert.equal(url, 'http://127.0.0.1:1234/tusupload/owned-upload');
    if (init.method === 'HEAD') return reply(200, { 'Upload-Offset': String(options.offset ?? offset), 'Upload-Length': String(blob.size) });
    assert.equal(init.method, 'PATCH'); assert.equal(init.headers['Upload-Offset'], String(offset));
    if (options.unauthorizedPatch && patches === 0) { patches++; return reply(401); }
    offset += init.body.size; patches++;
    if (options.lostPatch && patches === 1) throw new Error('response lost after commit');
    return reply(204, { 'Upload-Offset': String(options.badPatch ? offset + 1 : offset) });
  };
  const upload = new BunnyUpload({ file: blob, title: 'Aula de Rust', getGrant: options.getGrant ?? (async () => makeGrant()),
    allowProtocolFixture: true, fetchImpl });
  return { upload, calls, counts: () => ({ offset, patches, posts }) };
}

{
  const run = fixture({ lostPatch: true });
  assert.equal((await run.upload.start()).status, 'complete');
  assert.deepEqual(run.calls, ['POST', 'HEAD', 'PATCH', 'HEAD', 'PATCH']);
  assert.deepEqual(run.counts(), { offset: blob.size, patches: 2, posts: 1 });
  await run.upload.start(); assert.equal(run.counts().posts, 1);
}
{
  const run = fixture({ uncertain: true });
  await assert.rejects(run.upload.start(), /creation_uncertain/);
  await assert.rejects(run.upload.start(), /creation_uncertain/);
  assert.equal(run.counts().posts, 1);
}
for (const location of ['https://other.example/tusupload/id', '/unrelated/path', '/tusupload/%2e%2e/private', '/tusupload/id?token=x']) {
  const run = fixture({ location });
  await assert.rejects(run.upload.start(), /creation_uncertain/);
  assert.deepEqual(run.calls, ['POST']);
}
for (const options of [{ badPatch: true }, { offset: blob.size + 1 }, { offset: -1 }]) {
  const run = fixture(options); await assert.rejects(run.upload.start(), /interrupted/);
  assert(run.counts().patches <= 1);
}
{
  const run = fixture({ getGrant: async () => ({ ...makeGrant(), mode: 'Offline' }) });
  await assert.rejects(run.upload.start(), /interrupted/); assert.deepEqual(run.calls, []);
}
{
  let second;
  const entered = new Promise(resolve => { second = resolve; });
  const run = fixture({ getGrant: () => { second(); return new Promise(() => {}); } });
  const pending = run.upload.start(); await entered;
  run.upload.pause(); assert.equal((await pending).status, 'paused'); assert.deepEqual(run.calls, []);
  run.upload.cancel(); await assert.rejects(run.upload.start(), /cancelled/);
}
{
  let grants = 0;
  const run = fixture({ unauthorizedPatch: true, getGrant: async () => ({ ...makeGrant(), video: ++grants === 1 ? video : '22345678-1234-4234-8234-123456789abc' }) });
  await assert.rejects(run.upload.start(), /interrupted/); assert.equal(run.counts().patches, 1);
}
{
  // A maximum-length grant from a server clock ahead of the browser is valid.
  const run = fixture({ getGrant: async () => ({ ...makeGrant(), expires_at: Math.floor(Date.now() / 1000) + 3605 }) });
  assert.equal((await run.upload.start()).status, 'complete');
}
{
  // Beyond the skew allowance a grant is still refused before any request.
  const run = fixture({ getGrant: async () => ({ ...makeGrant(), expires_at: Math.floor(Date.now() / 1000) + 4500 }) });
  await assert.rejects(run.upload.start(), /interrupted/); assert.deepEqual(run.calls, []);
}
assert.throws(() => new BunnyUpload({ file: blob, title: 'Rust', getGrant: async () => makeGrant(), maxBytes: 1 }), /invalid_upload_input/);
// A sized stand-in for a large local file; slices allocate only one chunk.
class Sized extends Blob {
  #size;
  constructor(size, type = 'video/mp4') { super([], { type }); this.#size = size; }
  get size() { return this.#size; }
  slice(start, end) { return new Blob([new Uint8Array(Math.min(end, this.#size) - start)]); }
}
const GiB = 1073741824;
const accepts = options => new BunnyUpload({ title: 'Aula longa', getGrant: async () => makeGrant(), ...options });
assert.equal(BunnyUpload.DEFAULT_MAX_BYTES, GiB); assert.equal(BunnyUpload.MAX_BYTES, 64 * GiB);
assert.throws(() => accepts({ file: new Sized(GiB + 1) }), /invalid_upload_input/);
accepts({ file: new Sized(GiB) });
accepts({ file: new Sized(64 * GiB), maxBytes: 64 * GiB });
for (const maxBytes of [64 * GiB + 1, 0, 1.5, Number.MAX_SAFE_INTEGER + 1]) {
  assert.throws(() => accepts({ file: new Sized(1), maxBytes }), /invalid_upload_input/);
}
assert.throws(() => accepts({ file: new Sized(4 * GiB), maxBytes: 3 * GiB }), /invalid_upload_input/);
// The media type list is replaceable, bounded and video-only.
assert.throws(() => accepts({ file: new Sized(10, 'video/x-matroska') }), /invalid_upload_input/);
accepts({ file: new Sized(10, 'video/x-matroska'), types: ['video/x-matroska', 'video/mp4'] });
for (const types of [[], ['text/html'], ['video/'], ['Video/MP4'], 'video/mp4', [7], Array(17).fill('video/mp4')]) {
  assert.throws(() => accepts({ file: new Sized(10), types }), /invalid_upload_input/);
}
assert.throws(() => accepts({ file: new Sized(10), types: ['video/webm'] }), /invalid_upload_input/);
{
  // 5 GiB uploads in 3 MiB chunks: 1707 PATCH requests, within the budget.
  const size = 5 * GiB; let offset = 0; const bodies = new Set(); let patches = 0;
  const fetchImpl = async (url, init) => {
    if (init.method === 'POST') { assert.equal(init.headers['Upload-Length'], String(size)); return reply(201, { Location: '/tusupload/owned-upload' }); }
    if (init.method === 'HEAD') return reply(200, { 'Upload-Offset': String(offset), 'Upload-Length': String(size) });
    offset += init.body.size; bodies.add(init.body.size); patches++;
    return reply(204, { 'Upload-Offset': String(offset) });
  };
  const upload = new BunnyUpload({ file: new Sized(size), title: 'Aula longa', maxBytes: 8 * GiB, allowProtocolFixture: true,
    getGrant: async () => makeGrant(), fetchImpl });
  assert.equal((await upload.start()).status, 'complete');
  assert.equal(offset, size); assert.equal(patches, Math.ceil(size / (3 * 1048576)));
  assert.deepEqual([...bodies].sort((a, b) => b - a)[0], 3 * 1048576);
}
console.log('Bunny upload client: offset recovery, cancellation, identity, uncertain creation, grant lifetime, URL boundaries, size limit and media types passed.');
