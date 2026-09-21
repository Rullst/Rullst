import { BunnyUpload } from '/static/upload.mjs';
const status = document.querySelector('#status');
const player = document.querySelector('#player');
let task;
const scope = { tenant: 'school-a', course: 'rust-course' };
async function api(action, signal) {
  const response = await fetch(`/video/${action}`, { method: 'POST', credentials: 'same-origin', signal,
    headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': document.querySelector('meta[name=csrf]').content }, body: JSON.stringify({ scope }) });
  if (!response.ok) throw new Error('request_rejected');
  return response.json();
}
const actions = {
  create: async () => { await api('create'); status.textContent = 'created'; },
  upload: async () => {
    task ??= new BunnyUpload({ file: document.querySelector('#file').files[0], title: 'Ownership in Rust', allowProtocolFixture: true,
      getGrant: async ({ signal }) => (await api('upload', signal)).upload });
    const result = await task.start(); status.textContent = result.status === 'complete' ? 'uploaded' : result.status;
  },
  pause: async () => { task?.pause(); status.textContent = 'paused'; },
  cancel: async () => { task?.cancel(); status.textContent = 'cancelled'; },
  publish: async () => { await api('publish'); status.textContent = 'published'; },
  playback: async () => {
    const { playback } = await api('playback');
    const url = new URL(playback.url);
    // This page is an explicit loopback protocol fixture. The production
    // application must allow only its configured provider player origin.
    if (playback.mode !== 'ProtocolFixture' || url.protocol !== 'http:' || url.hostname !== '127.0.0.1') throw new Error('invalid_playback');
    const frame = document.createElement('iframe'); frame.title = 'Rust lesson video'; frame.src = url.href;
    frame.referrerPolicy = 'no-referrer'; frame.allow = 'fullscreen';
    player.replaceChildren(frame); status.textContent = 'playing';
  },
  withdraw: async () => { await api('withdraw'); player.replaceChildren(); status.textContent = 'withdrawn'; },
  delete: async () => { await api('delete'); player.replaceChildren(); status.textContent = 'deleted'; },
};
for (const [name, action] of Object.entries(actions)) document.getElementById(name).addEventListener('click', () => action().catch(() => { status.textContent = 'rejected'; }));
window.addEventListener('pagehide', () => task?.pause());
