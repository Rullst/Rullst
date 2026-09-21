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
assert.throws(() => new BunnyUpload({ file: blob, title: 'Rust', getGrant: async () => makeGrant(), maxBytes: 1 }), /invalid_upload_input/);
console.log('Bunny upload client: offset recovery, cancellation, identity, uncertain creation and URL boundaries passed.');
