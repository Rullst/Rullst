// Complete-snapshot recovery. Commands are never queued or replayed on reconnect.
export function connectLive(root, path, status = null) {
  if (!(root instanceof HTMLElement) || (status !== null && !(status instanceof HTMLElement))) {
    throw new TypeError('Live requires existing HTML elements');
  }
  const url = new URL(path, location.href);
  if (url.origin !== location.origin || url.username || url.password || url.hash ||
      !['http:', 'https:'].includes(url.protocol)) throw new TypeError('Live requires a same-origin URL');
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  const encoder = new TextEncoder();
  const identifier = /^[A-Za-z0-9_.:/-]{1,64}$/;
  const revisionPattern = /^(0|[1-9][0-9]{0,19})$/;
  const disabledBefore = new WeakMap();
  let socket = null, revision = null, pending = null, stopped = false;
  let attempts = 0, reconnectTimer = null, operationTimer = null, state = 'connecting';

  function setState(next) {
    state = next;
    root.dataset.liveState = next;
    root.setAttribute('aria-busy', String(next !== 'ready' || pending !== null));
    if (status) { status.textContent = next; status.setAttribute('role', 'status'); }
    for (const button of root.querySelectorAll('button[data-live-action],input[data-live-action]')) {
      if (!disabledBefore.has(button)) disabledBefore.set(button, button.disabled);
      button.disabled = disabledBefore.get(button) || next !== 'ready' || pending !== null;
    }
    root.dispatchEvent(new CustomEvent('rullst:live-state', { detail: { state: next, pending: pending !== null } }));
  }

  function uncertain() {
    if (pending) root.dispatchEvent(new CustomEvent('rullst:live-result', { detail: { id: pending, outcome: 'unknown' } }));
    pending = null;
    revision = null;
    clearTimeout(operationTimer);
  }

  function deadline(candidate) {
    clearTimeout(operationTimer);
    operationTimer = setTimeout(() => {
      if (socket === candidate) candidate.close(4001, 'refresh required');
    }, 30000);
  }

  function schedule() {
    if (stopped) return;
    if (++attempts > 12) { setState('unavailable'); return; }
    setState('recovering');
    const delay = Math.min(10000, 250 * 2 ** (attempts - 1)) * (0.8 + Math.random() * 0.4);
    reconnectTimer = setTimeout(open, delay);
  }

  function open() {
    if (stopped || socket) return;
    clearTimeout(reconnectTimer);
    setState('connecting');
    const candidate = new WebSocket(url, 'rullst.live.v1');
    socket = candidate;
    deadline(candidate);
    candidate.addEventListener('message', event => {
      if (socket !== candidate || stopped) return;
      try {
        if (typeof event.data !== 'string' || event.data.length > 524288 || encoder.encode(event.data).length > 524288) throw new Error('frame');
        const message = JSON.parse(event.data);
        if (!message || message.version !== 1 || message.kind !== 'snapshot' ||
            typeof message.revision !== 'string' || !revisionPattern.test(message.revision) ||
            BigInt(message.revision) > 18446744073709551615n ||
            typeof message.html !== 'string' || encoder.encode(message.html).length > 65536 ||
            !['recovered', 'applied', 'conflict'].includes(message.outcome) ||
            (pending === null ? revision !== null || message.id !== null || message.outcome !== 'recovered' :
              message.id !== pending || message.outcome === 'recovered') ||
            (message.outcome === 'applied' && BigInt(message.revision) <= BigInt(revision)) ||
            (revision !== null && BigInt(message.revision) < BigInt(revision))) throw new Error('snapshot');
        // Only trusted application-rendered HTML belongs here. Escape user input
        // in the server renderer; this transport is not an HTML sanitizer.
        root.innerHTML = message.html;
        revision = message.revision;
        pending = null;
        attempts = 0;
        clearTimeout(operationTimer);
        setState('ready');
        root.dispatchEvent(new CustomEvent('rullst:live-result', { detail: { id: message.id, outcome: message.outcome } }));
      } catch {
        candidate.close(4400, 'invalid protocol');
        stopped = true;
        root.replaceChildren();
        uncertain();
        setState('invalid');
      }
    });
    candidate.addEventListener('close', event => {
      if (socket !== candidate) return;
      socket = null;
      uncertain();
      if (stopped) return;
      if (event.code === 4401 || event.code === 4400) {
        root.replaceChildren();
        setState(event.code === 4401 ? 'denied' : 'invalid');
        return;
      }
      schedule();
    });
    candidate.addEventListener('error', () => { /* close controls bounded recovery */ });
  }

  function send(action, fields = {}) {
    if (stopped || state !== 'ready' || pending !== null || !socket || socket.readyState !== WebSocket.OPEN || socket.bufferedAmount !== 0) return false;
    if (typeof action !== 'string' || !identifier.test(action) || !fields || typeof fields !== 'object' || Array.isArray(fields)) return false;
    const entries = Object.entries(fields);
    if (entries.length > 32 || entries.some(([name, value]) => !identifier.test(name) || typeof value !== 'string' || value.includes('\0') || encoder.encode(value).length > 2048)) return false;
    const id = Array.from(crypto.getRandomValues(new Uint8Array(16)), byte => byte.toString(16).padStart(2, '0')).join('');
    const encoded = JSON.stringify({ version: 1, kind: 'action', id, revision, action, fields: Object.fromEntries(entries) });
    if (encoder.encode(encoded).length > 16384) return false;
    pending = id;
    setState('ready');
    deadline(socket);
    try { socket.send(encoded); } catch { socket.close(4001, 'refresh required'); }
    return true;
  }

  function click(event) {
    const element = event.target instanceof Element ? event.target.closest('[data-live-action]') : null;
    if (!element || !root.contains(element) || element instanceof HTMLFormElement) return;
    if ((element instanceof HTMLButtonElement || element instanceof HTMLInputElement) && element.type === 'submit' && element.form) return;
    event.preventDefault();
    send(element.dataset.liveAction);
  }

  function submit(event) {
    const form = event.target;
    if (!(form instanceof HTMLFormElement) || !form.dataset.liveAction || !root.contains(form)) return;
    event.preventDefault();
    const fields = Object.create(null);
    for (const [name, value] of new FormData(form)) {
      if (Object.hasOwn(fields, name) || typeof value !== 'string') return;
      fields[name] = value;
    }
    send(form.dataset.liveAction, fields);
  }

  root.addEventListener('click', click);
  root.addEventListener('submit', submit);
  open();
  return Object.freeze({
    send,
    reconnect() {
      if (stopped) return false;
      attempts = 0;
      if (socket) socket.close(4001, 'refresh required'); else open();
      return true;
    },
    dispose() {
      stopped = true;
      clearTimeout(reconnectTimer);
      clearTimeout(operationTimer);
      root.removeEventListener('click', click);
      root.removeEventListener('submit', submit);
      socket?.close(1000, 'complete');
      uncertain();
      setState('closed');
    },
    get state() { return state; },
  });
}
